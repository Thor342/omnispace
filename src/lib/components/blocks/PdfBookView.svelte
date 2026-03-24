<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { readFileAsBase64 } from "../../api";

  // Monta en document.body para escapar de transforms del bloque
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy() { node.remove(); } };
  }

  export let path: string;
  export let name: string;
  export let onClose: () => void;

  // ── Modal position & size ────────────────────────────────
  let x = 0, y = 0, w = 0, h = 0;

  // ── Drag modal (header) ──────────────────────────────────
  let dragging = false;
  let dragOX = 0, dragOY = 0;

  function startDrag(e: MouseEvent) {
    e.preventDefault();
    dragging = true;
    dragOX = e.clientX - x;
    dragOY = e.clientY - y;
  }

  // ── Resize modal (grip) ──────────────────────────────────
  let resizing = false;
  let resX = 0, resY = 0, resW = 0, resH = 0;

  function startResize(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    resX = e.clientX; resY = e.clientY;
    resW = w;         resH = h;
  }

  // ── Zoom & Pan ───────────────────────────────────────────
  let zoomLevel    = 1;
  let renderedZoom = 1;   // zoom al que el canvas fue renderizado por última vez
  let panX = 0, panY = 0;
  let isPanning = false;
  let panSX = 0, panSY = 0;
  let pagesContainer: HTMLDivElement;
  let rerenderTimer: ReturnType<typeof setTimeout> | null = null;

  // Scale CSS = ratio entre zoom pedido y zoom renderizado.
  // Cuando son iguales → 1 (sin escala CSS, máxima nitidez).
  // Mientras el timer no dispara → feedback inmediato con ligero blur.
  $: visualScale = zoomLevel / renderedZoom;

  function scheduleRerender() {
    if (rerenderTimer) clearTimeout(rerenderTimer);
    rerenderTimer = setTimeout(async () => {
      rerenderTimer = null;
      await renderSpread(); // renderSpread actualiza renderedZoom al final
    }, 180);
  }

  function startPan(e: MouseEvent) {
    if (dragging || resizing) return;
    isPanning = true;
    panSX = e.clientX - panX;
    panSY = e.clientY - panY;
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const factor  = e.deltaY < 0 ? 1.12 : 1 / 1.12;
    const newZoom = Math.max(0.25, Math.min(8, zoomLevel * factor));

    const rect  = pagesContainer.getBoundingClientRect();
    const cx    = e.clientX - (rect.left + rect.width  / 2);
    const cy    = e.clientY - (rect.top  + rect.height / 2);
    const ratio = newZoom / zoomLevel;
    panX = cx * (1 - ratio) + panX * ratio;
    panY = cy * (1 - ratio) + panY * ratio;
    zoomLevel = newZoom;
    scheduleRerender();
  }

  function adjustZoom(factor: number) {
    const newZoom = Math.max(0.25, Math.min(8, zoomLevel * factor));
    const ratio   = newZoom / zoomLevel;
    panX = panX * ratio;
    panY = panY * ratio;
    zoomLevel = newZoom;
    scheduleRerender();
  }

  function resetZoomPan() {
    if (rerenderTimer) { clearTimeout(rerenderTimer); rerenderTimer = null; }
    zoomLevel    = 1;
    renderedZoom = 1;
    panX = 0; panY = 0;
    renderSpread();
  }

  // ── Manejadores globales (drag modal + resize + pan) ─────
  function onMouseMove(e: MouseEvent) {
    if (dragging) {
      x = Math.max(0, Math.min(window.innerWidth  - w, e.clientX - dragOX));
      y = Math.max(0, Math.min(window.innerHeight - h, e.clientY - dragOY));
    }
    if (resizing) {
      w = Math.max(460, Math.min(window.innerWidth  - x, resW + (e.clientX - resX)));
      h = Math.max(320, Math.min(window.innerHeight - y, resH + (e.clientY - resY)));
    }
    if (isPanning) {
      panX = e.clientX - panSX;
      panY = e.clientY - panSY;
    }
  }

  async function onMouseUp() {
    const wasResizing = resizing;
    dragging  = false;
    resizing  = false;
    isPanning = false;
    if (wasResizing) await renderSpread();
  }

  // ── PDF state ────────────────────────────────────────────
  let spread = 0;
  let totalPages = 0;
  let pdfDoc: any = null;
  let loading = true;
  let error = "";
  let rendering = false;

  let leftCanvas:  HTMLCanvasElement;
  let rightCanvas: HTMLCanvasElement;

  $: leftPageNum  = spread * 2 + 1;
  $: rightPageNum = spread * 2 + 2;
  $: totalSpreads = Math.ceil(totalPages / 2);
  $: zoomPct      = Math.round(zoomLevel * 100);

  onMount(async () => {
    w = Math.min(Math.round(window.innerWidth  * 0.96), 1440);
    h = Math.round(window.innerHeight * 0.94);
    x = Math.round((window.innerWidth  - w) / 2);
    y = Math.round((window.innerHeight - h) / 2);

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup",   onMouseUp);
    window.addEventListener("keydown",   onKey);

    await loadBook();
  });

  onDestroy(() => {
    if (rerenderTimer) clearTimeout(rerenderTimer);
    pdfDoc?.destroy();
    pagesContainer?.removeEventListener("wheel", onWheel);
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup",   onMouseUp);
    window.removeEventListener("keydown",   onKey);
  });

  // El wheel listener se agrega cuando el contenedor está en el DOM
  function bindWheel(node: HTMLDivElement) {
    pagesContainer = node;
    node.addEventListener("wheel", onWheel, { passive: false });
    return { destroy() { node.removeEventListener("wheel", onWheel); } };
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowRight" || e.key === "ArrowDown") nextSpread();
    if (e.key === "ArrowLeft"  || e.key === "ArrowUp")   prevSpread();
    if (e.key === "Escape") onClose();
    if (e.key === "+" || e.key === "=") adjustZoom(1.25);
    if (e.key === "-") adjustZoom(1 / 1.25);
    if (e.key === "0") resetZoomPan();
  }

  async function loadBook() {
    loading = true;
    error = "";
    try {
      const pdfjsLib = await import("pdfjs-dist");
      pdfjsLib.GlobalWorkerOptions.workerSrc = new URL(
        "pdfjs-dist/build/pdf.worker.min.mjs",
        import.meta.url
      ).href;

      const b64  = await readFileAsBase64(path);
      const data = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
      pdfDoc     = await pdfjsLib.getDocument({ data }).promise;
      totalPages = pdfDoc.numPages;
      loading    = false;
      await tick();
      await renderSpread();
    } catch (e) {
      error   = `Error al cargar el PDF: ${e}`;
      loading = false;
    }
  }

  async function renderSpread() {
    if (!pdfDoc || rendering) return;
    rendering = true;
    await Promise.all([
      renderPage(leftPageNum, leftCanvas),
      rightPageNum <= totalPages
        ? renderPage(rightPageNum, rightCanvas)
        : clearCanvas(rightCanvas),
    ]);
    rendering = false;
    renderedZoom = zoomLevel; // canvas está ahora al zoom actual → sin CSS scale
  }

  async function renderPage(num: number, canvas: HTMLCanvasElement) {
    if (!canvas || !pdfDoc) return;
    const page = await pdfDoc.getPage(num);

    // Escala base: página encaja en su mitad del modal a zoom 1
    const headerH = 44, footerH = 40, padH = 24, padW = 120;
    const maxW = (w - padW) / 2;
    const maxH =  h - headerH - footerH - padH;
    const vp1       = page.getViewport({ scale: 1 });
    const baseScale = Math.min(maxH / vp1.height, maxW / vp1.width, 2.5);

    // Incluir zoomLevel en el render → canvas nítido a cualquier zoom
    const dpr   = window.devicePixelRatio || 1;
    const scale = baseScale * dpr * zoomLevel;
    const vp    = page.getViewport({ scale });

    canvas.width  = vp.width;
    canvas.height = vp.height;
    // Tamaño CSS = píxeles lógicos (incluye zoom, sin DPR)
    canvas.style.width  = (vp.width  / dpr) + "px";
    canvas.style.height = (vp.height / dpr) + "px";

    const ctx = canvas.getContext("2d")!;
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    await page.render({ canvasContext: ctx, viewport: vp }).promise;
  }

  function clearCanvas(canvas: HTMLCanvasElement) {
    if (!canvas) return;
    canvas.width = 0; canvas.height = 0;
  }

  async function prevSpread() {
    if (spread <= 0 || rendering) return;
    spread--;
    resetZoomPan();
    await renderSpread();
  }

  async function nextSpread() {
    if (spread >= totalSpreads - 1 || rendering) return;
    spread++;
    resetZoomPan();
    await renderSpread();
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  use:portal
  class="book-overlay"
  class:dragging
  class:resizing
  class:panning={isPanning}
  on:click={e => e.target === e.currentTarget && onClose()}
  on:keydown={e => e.key === "Escape" && onClose()}
  role="presentation"
  tabindex="-1"
>
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="book-modal"
    style="left:{x}px; top:{y}px; width:{w}px; height:{h}px;"
    on:click|stopPropagation
    on:keydown|stopPropagation
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >

    <!-- Header (arrastrable) -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="book-header" on:mousedown={startDrag}>
      <span class="book-title">📖 {name}</span>

      {#if totalPages}
        <span class="book-pages-info">
          Pág.&nbsp;{leftPageNum}{rightPageNum <= totalPages ? `–${rightPageNum}` : ""}
          &nbsp;/&nbsp;{totalPages}
        </span>
      {/if}

      <!-- Controles de zoom -->
      {#if !loading && !error}
        <div class="zoom-controls" on:mousedown|stopPropagation>
          <button class="zoom-btn" on:click={() => adjustZoom(1/1.25)} title="Alejar (-)">−</button>
          <button class="zoom-pct" on:click={resetZoomPan} title="Restablecer zoom (0)">{zoomPct}%</button>
          <button class="zoom-btn" on:click={() => adjustZoom(1.25)} title="Acercar (+)">+</button>
        </div>
      {/if}

      <button class="book-close" on:click={onClose} on:mousedown|stopPropagation>✕</button>
    </div>

    <!-- Cuerpo -->
    {#if loading}
      <div class="book-center">
        <span class="book-spinner">⏳</span>
        <span>Cargando PDF…</span>
      </div>
    {:else if error}
      <div class="book-center book-error">{error}</div>
    {:else}
      <div class="book-content">

        <button class="book-nav" disabled={spread === 0 || rendering}
          on:click={prevSpread} title="Anterior (←)">‹</button>

        <!-- Área de páginas con zoom/pan -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
          class="book-pages"
          class:zoomable={zoomLevel > 1}
          class:panning={isPanning}
          use:bindWheel
          on:mousedown={startPan}
        >
          <!-- Wrapper que recibe el transform de zoom+pan -->
          <div
            class="book-zoom-wrapper"
            style="transform: translate({panX}px, {panY}px) scale({visualScale}); transform-origin: center center;"
          >
            <div class="book-page left">
              <canvas bind:this={leftCanvas}></canvas>
            </div>
            <div class="book-spine"></div>
            <div class="book-page right">
              <canvas
                bind:this={rightCanvas}
                style:visibility={rightPageNum > totalPages ? "hidden" : "visible"}
              ></canvas>
            </div>
          </div>
        </div>

        <button class="book-nav" disabled={spread >= totalSpreads - 1 || rendering}
          on:click={nextSpread} title="Siguiente (→)">›</button>

      </div>

      <div class="book-footer">
        <button class="book-foot-btn" disabled={spread === 0 || rendering} on:click={prevSpread}>← Anterior</button>
        <span class="book-foot-info">Hoja {spread + 1} de {totalSpreads}</span>
        <button class="book-foot-btn" disabled={spread >= totalSpreads - 1 || rendering} on:click={nextSpread}>Siguiente →</button>
      </div>
    {/if}

    <!-- Grip resize -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="book-resize-grip" on:mousedown={startResize} title="Redimensionar">
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
        <line x1="11" y1="1"  x2="1"  y2="11" stroke="currentColor" stroke-width="1.5"/>
        <line x1="11" y1="5"  x2="5"  y2="11" stroke="currentColor" stroke-width="1.5"/>
        <line x1="11" y1="9"  x2="9"  y2="11" stroke="currentColor" stroke-width="1.5"/>
      </svg>
    </div>

  </div>
</div>

<style>
  .book-overlay {
    position: fixed; inset: 0;
    background: rgba(0, 0, 0, 0.55);
    z-index: 1000;
    pointer-events: auto;
  }
  .book-overlay.dragging  { cursor: grabbing !important; }
  .book-overlay.resizing  { cursor: nwse-resize !important; }
  .book-overlay.panning   { cursor: grabbing !important; }
  .book-overlay.dragging  *,
  .book-overlay.resizing  *,
  .book-overlay.panning   * { user-select: none; pointer-events: none; }
  .book-overlay.dragging  .book-modal,
  .book-overlay.resizing  .book-modal,
  .book-overlay.panning   .book-modal { pointer-events: auto; }

  /* Modal */
  .book-modal {
    position: absolute;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    display: flex; flex-direction: column;
    overflow: hidden;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.65);
  }

  /* Header */
  .book-header {
    display: flex; align-items: center; gap: 10px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    cursor: grab; user-select: none;
    background: var(--bg-surface);
  }
  .book-header:active { cursor: grabbing; }
  .book-title      { flex: 1; font-size: 13px; font-weight: 600; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .book-pages-info { font-size: 11px; color: var(--text-muted); white-space: nowrap; flex-shrink: 0; }
  .book-close      { padding: 4px 8px; border-radius: 6px; color: var(--text-muted); font-size: 13px; cursor: pointer; flex-shrink: 0; }
  .book-close:hover { background: var(--bg-hover); color: var(--text-primary); }

  /* Controles de zoom */
  .zoom-controls {
    display: flex; align-items: center; gap: 2px;
    flex-shrink: 0;
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px;
  }
  .zoom-btn {
    width: 24px; height: 24px;
    display: flex; align-items: center; justify-content: center;
    border-radius: 4px; font-size: 16px; font-weight: 500;
    color: var(--text-secondary); cursor: pointer; transition: all 0.12s;
  }
  .zoom-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .zoom-pct {
    min-width: 44px; height: 24px; font-size: 11px; font-weight: 600;
    color: var(--text-secondary); text-align: center;
    border-radius: 4px; cursor: pointer; transition: all 0.12s;
  }
  .zoom-pct:hover { background: var(--bg-hover); color: var(--accent); }

  /* Loading / error */
  .book-center {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 10px; color: var(--text-muted); font-size: 14px;
  }
  .book-spinner { font-size: 28px; animation: spin 1.2s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .book-error { color: var(--red, #ef4444); }

  /* Contenido */
  .book-content {
    flex: 1; display: flex; align-items: center;
    gap: 6px; padding: 10px 6px; overflow: hidden; min-height: 0;
  }

  /* Nav buttons */
  .book-nav {
    flex-shrink: 0; width: 34px; height: 60px;
    background: var(--bg-overlay); border: 1px solid var(--border);
    border-radius: var(--radius-sm); font-size: 24px; color: var(--text-secondary);
    display: flex; align-items: center; justify-content: center;
    transition: all 0.15s; cursor: pointer; z-index: 1;
  }
  .book-nav:hover:not(:disabled) { background: var(--bg-hover); color: var(--text-primary); border-color: var(--accent); }
  .book-nav:disabled { opacity: 0.22; cursor: default; }

  /* Zona de páginas */
  .book-pages {
    flex: 1; height: 100%;
    display: flex; align-items: center; justify-content: center;
    overflow: hidden; min-height: 0;
    cursor: default;
  }
  .book-pages.zoomable { cursor: grab; }
  .book-pages.panning  { cursor: grabbing; }

  /* Wrapper del zoom (recibe el transform) */
  .book-zoom-wrapper {
    display: flex; align-items: center; justify-content: center;
    will-change: transform;
  }

  /* Páginas individuales */
  .book-page {
    display: flex; align-items: center; justify-content: center;
  }
  .book-page canvas { display: block; }
  .book-page.left  canvas { box-shadow:  4px 0 18px rgba(0,0,0,0.45), -1px 0 4px rgba(0,0,0,0.08); }
  .book-page.right canvas { box-shadow: -4px 0 18px rgba(0,0,0,0.45),  1px 0 4px rgba(0,0,0,0.08); }

  /* Lomo */
  .book-spine {
    flex-shrink: 0; width: 6px; height: 86%;
    background: linear-gradient(to right,
      rgba(0,0,0,0.28) 0%, rgba(0,0,0,0.07) 40%,
      rgba(255,255,255,0.04) 60%, rgba(0,0,0,0.18) 100%
    );
    border-left:  1px solid rgba(0,0,0,0.22);
    border-right: 1px solid rgba(255,255,255,0.06);
  }

  /* Footer */
  .book-footer {
    display: flex; align-items: center; justify-content: space-between;
    padding: 7px 16px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }
  .book-foot-info { font-size: 12px; color: var(--text-muted); }
  .book-foot-btn {
    font-size: 12px; color: var(--text-secondary);
    padding: 4px 12px; border-radius: var(--radius-sm);
    border: 1px solid var(--border); transition: all 0.15s; cursor: pointer;
  }
  .book-foot-btn:hover:not(:disabled) { background: var(--bg-hover); color: var(--text-primary); border-color: var(--accent); }
  .book-foot-btn:disabled { opacity: 0.3; cursor: default; }

  /* Grip resize */
  .book-resize-grip {
    position: absolute; bottom: 0; right: 0;
    width: 20px; height: 20px; cursor: nwse-resize;
    display: flex; align-items: flex-end; justify-content: flex-end;
    padding: 3px; color: var(--text-muted);
    border-radius: 0 0 var(--radius-lg) 0;
    transition: color 0.15s;
  }
  .book-resize-grip:hover { color: var(--text-secondary); }
</style>
