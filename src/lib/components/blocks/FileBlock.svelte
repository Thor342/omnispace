<script lang="ts">
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { tick, onDestroy } from "svelte";
  import { readFileAsBase64 } from "../../api";
  import type { FileContent } from "../../types";
  import DOMPurify from "dompurify";
  import PdfBookView from "./PdfBookView.svelte";
  import { t } from "../../stores/language";

  export let content: string;
  export let onUpdate: ((content: string) => void) | undefined = undefined;

  $: data = JSON.parse(content || '{"stored_path":"","name":"","file_type":"other","size":0}') as FileContent;
  // Asset protocol tiene soporte nativo de Range requests en WebView2 (dev y producción).
  // El protocolo stream custom no es accesible por el pipeline de medios de WebView2.
  function streamSrc(path: string): string {
    if (!path) return '';
    return convertFileSrc(path);
  }

  // ── Image loader (base64 to avoid asset protocol issues) ──────────────────
  let imageSrc = "";
  let _imagePath = "";
  $: if (data.file_type === "image" && data.stored_path && data.stored_path !== _imagePath) {
    _imagePath = data.stored_path;
    imageSrc = "";
    loadImage(data.stored_path);
  }
  async function loadImage(path: string) {
    try {
      const b64 = await readFileAsBase64(path);
      const ext = path.split('.').pop()?.toLowerCase() ?? 'jpg';
      const mimes: Record<string, string> = { jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png', gif: 'image/gif', webp: 'image/webp', bmp: 'image/bmp', svg: 'image/svg+xml' };
      imageSrc = `data:${mimes[ext] ?? 'image/jpeg'};base64,${b64}`;
    } catch { imageSrc = ""; }
  }

  // ── PDF loader — lazy (only loads when user clicks "Ver PDF") ─────────────
  let pdfBlobUrl = "";
  let pdfOpened  = false;
  let pdfLoading = false;
  let showBookView = false;

  async function openPdf() {
    pdfOpened  = true;
    pdfLoading = true;
    await loadPdf(data.stored_path);
    pdfLoading = false;
  }

  async function loadPdf(path: string) {
    try {
      const b64   = await readFileAsBase64(path);
      const bytes = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
      const blob  = new Blob([bytes], { type: 'application/pdf' });
      if (pdfBlobUrl) URL.revokeObjectURL(pdfBlobUrl);
      pdfBlobUrl = URL.createObjectURL(blob);
    } catch { pdfBlobUrl = ""; }
  }

  function formatSize(b: number) {
    if (b < 1024) return `${b} B`;
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
    return `${(b / (1024 * 1024)).toFixed(1)} MB`;
  }

  // ── Helpers ────────────────────────────────────────────
  function b64ToArrayBuffer(b64: string): ArrayBuffer {
    const binary = atob(b64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes.buffer;
  }

  // ── Word viewer/editor ─────────────────────────────────
  let wordHtml = "";
  let wordLoading = false;
  let wordEditing = false;
  let wordEditEl: HTMLDivElement;

  // Restaurar HTML guardado o cargar desde el archivo
  $: if (data.file_type === "word" && data.stored_path && !wordHtml) {
    if (data.word_html) { wordHtml = DOMPurify.sanitize(data.word_html); } else { loadWord(); }
  }

  async function loadWord() {
    if (wordLoading) return;
    wordLoading = true;
    try {
      const { default: mammoth } = await import("mammoth");
      const b64 = await readFileAsBase64(data.stored_path);
      const ab = b64ToArrayBuffer(b64);
      const result = await mammoth.convertToHtml({ arrayBuffer: ab });
      wordHtml = DOMPurify.sanitize(result.value);
    } catch (e) {
      wordHtml = `<p style="color:red">Error al cargar: ${e}</p>`;
    } finally {
      wordLoading = false;
    }
  }

  let wordSaving = false;

  function escXml(s: string): string {
    return s.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;").replace(/"/g,"&quot;");
  }

  function nodeToRuns(node: Node): string {
    if (node.nodeType === Node.TEXT_NODE) {
      const t = node.textContent ?? "";
      return t ? `<w:r><w:t xml:space="preserve">${escXml(t)}</w:t></w:r>` : "";
    }
    if (node.nodeType === Node.ELEMENT_NODE) {
      const el = node as Element;
      const tag = el.tagName.toLowerCase();
      const children = Array.from(el.childNodes).map(nodeToRuns).join("");
      const isBold = tag === "strong" || tag === "b";
      const isItalic = tag === "em" || tag === "i";
      if (isBold || isItalic) {
        const rPr = `<w:rPr>${isBold ? "<w:b/>" : ""}${isItalic ? "<w:i/>" : ""}</w:rPr>`;
        return children.replace(/<w:r>/g, `<w:r>${rPr}`);
      }
      return children;
    }
    return "";
  }

  function elToOoxml(el: Element): string {
    const tag = el.tagName.toLowerCase();
    const styleMap: Record<string, string> = { h1:"Heading1", h2:"Heading2", h3:"Heading3" };
    const style = styleMap[tag];
    const pPr = style ? `<w:pPr><w:pStyle w:val="${style}"/></w:pPr>` : "";
    const runs = Array.from(el.childNodes).map(nodeToRuns).join("") ||
                 `<w:r><w:t>${escXml(el.textContent ?? "")}</w:t></w:r>`;
    return `<w:p>${pPr}${runs}</w:p>`;
  }

  async function saveWordEdit() {
    if (!wordEditEl || !onUpdate) return;
    const newHtml = DOMPurify.sanitize(wordEditEl.innerHTML);
    wordHtml = newHtml;
    wordEditing = false;
    wordSaving = true;
    try {
      const { default: JSZip } = await import("jszip");
      const { writeFile } = await import("@tauri-apps/plugin-fs");

      // Leer el .docx original
      const b64 = await readFileAsBase64(data.stored_path);
      const ab = b64ToArrayBuffer(b64);
      const zip = await JSZip.loadAsync(ab);

      // Convertir HTML editado a párrafos OOXML
      const htmlDoc = new DOMParser().parseFromString(newHtml, "text/html");
      const elems = Array.from(htmlDoc.body.children);
      const paragraphsXml = elems.length > 0
        ? elems.map(elToOoxml).join("\n")
        : `<w:p><w:r><w:t>${escXml(htmlDoc.body.textContent ?? "")}</w:t></w:r></w:p>`;

      // Preservar sectPr (márgenes, orientación de página) del documento original
      const docXml = await zip.files["word/document.xml"].async("string");
      const sectPrMatch = docXml.match(/<w:sectPr[\s\S]*?<\/w:sectPr>/);
      const sectPr = sectPrMatch ? sectPrMatch[0] : "<w:sectPr/>";

      const newDocXml = docXml.replace(
        /<w:body>[\s\S]*?<\/w:body>/,
        `<w:body>\n${paragraphsXml}\n${sectPr}\n</w:body>`
      );

      zip.file("word/document.xml", newDocXml);
      const newBuffer = await zip.generateAsync({ type: "uint8array" });
      await writeFile(data.stored_path, newBuffer);
    } catch (e) {
      console.error("Error guardando docx:", e);
    } finally {
      wordSaving = false;
    }
    onUpdate(JSON.stringify({ ...data, word_html: newHtml }));
  }

  // ── Excel viewer/editor ────────────────────────────────
  let excelSheets: string[] = [];
  let activeSheet = 0;
  let excelLoading = false;
  let excelSaving = false;
  let excelWb: any = null;
  // Grid data: rows of cells {v: value, t: type}
  interface XCell { v: string; t: string; addr: string; }
  let excelGrid: XCell[][] = [];
  let excelEditing = false;

  $: if (data.file_type === "excel" && !excelWb && data.stored_path) loadExcel();

  async function loadExcel() {
    if (excelLoading) return;
    excelLoading = true;
    try {
      const XLSX = await import("xlsx");
      const b64 = await readFileAsBase64(data.stored_path);
      const ab = b64ToArrayBuffer(b64);
      excelWb = XLSX.read(new Uint8Array(ab), { type: "array" });
      excelSheets = excelWb.SheetNames;
      renderSheetGrid(0);
    } catch (e) {
      excelGrid = [];
    } finally {
      excelLoading = false;
    }
  }

  async function renderSheetGrid(idx: number) {
    if (!excelWb) return;
    activeSheet = idx;
    const XLSX = await import("xlsx");
    const ws = excelWb.Sheets[excelSheets[idx]];
    const range = XLSX.utils.decode_range(ws["!ref"] || "A1:A1");
    const rows: XCell[][] = [];
    for (let r = range.s.r; r <= Math.min(range.e.r, 200); r++) {
      const row: XCell[] = [];
      for (let c = range.s.c; c <= Math.min(range.e.c, 50); c++) {
        const addr = XLSX.utils.encode_cell({ r, c });
        const cell = ws[addr];
        row.push({ v: cell ? String(cell.v ?? "") : "", t: cell?.t ?? "s", addr });
      }
      rows.push(row);
    }
    excelGrid = rows;
  }

  function onCellInput(e: Event, r: number, c: number) {
    onCellEdit(r, c, (e.target as HTMLInputElement).value);
  }

  function onCellEdit(r: number, c: number, val: string) {
    excelGrid = excelGrid.map((row, ri) =>
      ri === r ? row.map((cell, ci) => ci === c ? { ...cell, v: val } : cell) : row
    );
    excelEditing = true;
  }

  async function saveExcel() {
    if (!excelWb || excelSaving) return;
    excelSaving = true;
    try {
      const XLSX = await import("xlsx");
      const { writeFile } = await import("@tauri-apps/plugin-fs");
      const ws = excelWb.Sheets[excelSheets[activeSheet]];
      // Actualizar celdas modificadas
      for (let r = 0; r < excelGrid.length; r++) {
        for (let c = 0; c < excelGrid[r].length; c++) {
          const { addr, v } = excelGrid[r][c];
          if (ws[addr]) ws[addr].v = isNaN(Number(v)) ? v : Number(v);
          else if (v) ws[addr] = { v, t: "s" };
        }
      }
      const buf = XLSX.write(excelWb, { type: "array", bookType: "xlsx" });
      await writeFile(data.stored_path, new Uint8Array(buf));
      excelEditing = false;
    } catch (e) { console.error("Error guardando Excel:", e); }
    finally { excelSaving = false; }
  }

  // ── Audio player ───────────────────────────────────────
  let audioEl: HTMLAudioElement;
  let audioPlaying = false;
  let audioCurrentTime = 0;
  let audioDuration = 0;
  let audioPlayerEl: HTMLDivElement;
  let audioPlayerH = 300;
  let _audioRO: ResizeObserver | null = null;

  $: if (audioPlayerEl) {
    _audioRO?.disconnect();
    _audioRO = new ResizeObserver(entries => {
      audioPlayerH = entries[0]?.contentRect.height ?? audioPlayerEl.clientHeight;
    });
    _audioRO.observe(audioPlayerEl);
    audioPlayerH = audioPlayerEl.clientHeight;
  }

  onDestroy(() => {
    _audioRO?.disconnect();
    if (pdfBlobUrl) URL.revokeObjectURL(pdfBlobUrl);
  });

  $: audioTitle = data.name
    .replace(/\.[^.]+$/, "")
    .replace(/^clipboard_[a-f0-9]+_?/i, "")
    .replace(/_/g, " ")
    .trim() || data.name;

  function formatAudioTime(s: number): string {
    if (!isFinite(s) || s < 0) return "0:00";
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60);
    return `${m}:${sec.toString().padStart(2, "0")}`;
  }

  function toggleAudio() {
    if (!audioEl) return;
    if (audioPlaying) audioEl.pause(); else audioEl.play();
  }

  function onAudioSeek(e: MouseEvent) {
    if (!audioEl || !isFinite(audioDuration) || audioDuration <= 0) return;
    const bar = e.currentTarget as HTMLElement;
    const rect = bar.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    audioEl.currentTime = ratio * audioDuration;
  }



</script>

<div class="file-block">
  {#if !data.stored_path}
    <div class="empty-file">
      <span>📄</span>
      <p>{$t.canvas.noDoc}</p>
    </div>

  {:else if data.file_type === "image"}
    {#if imageSrc}
      <img src={imageSrc} alt={data.name} class="file-img" />
    {:else}
      <div class="empty-file"><span>🖼️</span><p>{$t.canvas.loadingImage}</p></div>
    {/if}

  {:else if data.file_type === "video"}
    <video controls class="file-video">
      <source src={streamSrc(data.stored_path)} />
      <track kind="captions" />
    </video>

  {:else if data.file_type === "pdf"}
    {#if !pdfOpened}
      <!-- Placeholder: no carga nada hasta que el usuario haga clic -->
      <div class="pdf-placeholder">
        <span class="pdf-ph-icon">📄</span>
        <span class="pdf-ph-name">{data.name}</span>
        <span class="pdf-ph-size">{formatSize(data.size)}</span>
        <button class="pdf-ph-btn" on:click={openPdf}>{$t.canvas.viewPdf}</button>
      </div>
    {:else}
      <div class="pdf-wrapper">
        {#if pdfLoading}
          <div class="doc-loading">{$t.canvas.loadingPdf}</div>
        {:else if pdfBlobUrl}
          <iframe src="{pdfBlobUrl}#toolbar=0&view=FitH" class="file-pdf" title={data.name}></iframe>
          <div class="pdf-btn-group">
            <button class="pdf-action-btn" on:click={() => showBookView = true} title={$t.canvas.bookView}>📖</button>
            <button class="pdf-action-btn" on:click={() => invoke('open_file', { path: data.stored_path })} title={$t.canvas.openSystem}>↗</button>
          </div>
        {:else}
          <div class="doc-loading">{$t.canvas.errorPdf}</div>
        {/if}
      </div>
      {#if showBookView}
        <PdfBookView path={data.stored_path} name={data.name} onClose={() => showBookView = false} />
      {/if}
    {/if}

  {:else if data.file_type === "word"}
    <div class="doc-wrapper">
      <!-- Toolbar -->
      <div class="doc-toolbar">
        <div class="doc-actions">
          {#if !wordEditing}
            <button class="doc-btn" on:click={async () => { wordEditing = true; await tick(); if (wordEditEl) { wordEditEl.innerHTML = wordHtml; wordEditEl.focus(); } }}>{$t.canvas.editDoc}</button>
          {:else}
            <button class="doc-btn accent" disabled={wordSaving} on:click={saveWordEdit}>
              {wordSaving ? $t.canvas.saving : $t.canvas.saveDoc}
            </button>
            <button class="doc-btn" on:click={() => wordEditing = false}>{$t.canvas.cancelDoc}</button>
          {/if}
          <button class="doc-btn" on:click={() => invoke('open_file', { path: data.stored_path })} title={$t.canvas.openWord}>↗</button>
        </div>
      </div>

      {#if wordLoading}
        <div class="doc-loading">{$t.canvas.loadingDoc}</div>
      {:else if wordEditing}
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
          class="word-editor"
          bind:this={wordEditEl}
          contenteditable="true"
          role="textbox"
          tabindex="0"
          aria-multiline="true"
          aria-label="Editor de documento"
          on:keydown={e => e.ctrlKey && e.key === 's' && (e.preventDefault(), saveWordEdit())}
        >
          <!-- Will be filled via wordHtml reactivity -->
        </div>
      {:else}
        <div class="word-view">
          {@html wordHtml || `<p style='color:var(--text-muted);text-align:center;margin-top:40px'>${$t.canvas.emptyDoc}</p>`}
        </div>
      {/if}
    </div>

  {:else if data.file_type === "excel"}
    <div class="excel-wrapper">
      <div class="doc-toolbar">
        <div class="sheet-tabs-inline">
          {#each excelSheets as sheet, i}
            <button class="sheet-tab" class:active={activeSheet === i}
              on:click={() => renderSheetGrid(i)}>{sheet}</button>
          {/each}
        </div>
        <div class="doc-actions">
          {#if excelEditing}
            <button class="doc-btn accent" disabled={excelSaving} on:click={saveExcel}>
              {excelSaving ? $t.canvas.saving : $t.canvas.saveDoc}
            </button>
          {/if}
          <button class="doc-btn" on:click={() => invoke('open_file', { path: data.stored_path })} title={$t.canvas.openExcel}>↗</button>
        </div>
      </div>

      {#if excelLoading}
        <div class="doc-loading">{$t.canvas.loadingSheet}</div>
      {:else}
        <div class="excel-scroll">
          <table class="excel-table">
            <thead>
              <tr>
                <th class="excel-row-header"></th>
                {#each excelGrid[0] ?? [] as _, c}
                  <th class="excel-col-header">{String.fromCharCode(65 + c)}</th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each excelGrid as row, r}
                <tr>
                  <td class="excel-row-header">{r + 1}</td>
                  {#each row as cell, c}
                    <td class="excel-cell">
                      <input
                        class="cell-input"
                        value={cell.v}
                        on:input={e => onCellInput(e, r, c)}
                        on:keydown={e => e.ctrlKey && e.key === 's' && (e.preventDefault(), saveExcel())}
                      />
                    </td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>

  {:else if data.file_type === "powerpoint"}
    <div class="ppt-card">
      <div class="ppt-icon-wrap">
        <svg class="ppt-icon" viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg">
          <rect width="48" height="48" rx="10" fill="#C43E1C"/>
          <rect x="8" y="9" width="24" height="30" rx="2" fill="white" opacity="0.12"/>
          <rect x="10" y="11" width="20" height="3" rx="1" fill="white" opacity="0.8"/>
          <rect x="10" y="16" width="15" height="2" rx="1" fill="white" opacity="0.55"/>
          <rect x="10" y="20" width="18" height="2" rx="1" fill="white" opacity="0.55"/>
          <rect x="10" y="24" width="12" height="2" rx="1" fill="white" opacity="0.55"/>
          <circle cx="36" cy="36" r="10" fill="#E84D1A"/>
          <path d="M33 31 L33 41 L41 36 Z" fill="white"/>
        </svg>
      </div>
      <div class="ppt-info">
        <p class="ppt-name" title={data.name}>{data.name.replace(/\.[^.]+$/, "")}</p>
        <p class="ppt-meta">PowerPoint · {formatSize(data.size)}</p>
      </div>
      <button class="ppt-open-btn" on:click={() => invoke('open_file', { path: data.stored_path })}>
        <svg viewBox="0 0 20 20" width="14" height="14" fill="currentColor"><path d="M11 3a1 1 0 100 2h2.586l-6.293 6.293a1 1 0 101.414 1.414L15 6.414V9a1 1 0 102 0V4a1 1 0 00-1-1h-5z"/><path d="M5 5a2 2 0 00-2 2v8a2 2 0 002 2h8a2 2 0 002-2v-3a1 1 0 10-2 0v3H5V7h3a1 1 0 000-2H5z"/></svg>
        {$t.canvas.openPpt}
      </button>
    </div>

  {:else if data.file_type === "audio"}
    <div class="audio-player" bind:this={audioPlayerEl} class:ap-compact={audioPlayerH < 170} class:ap-mini={audioPlayerH < 110}>

      <!-- Album art — hidden when small -->
      {#if audioPlayerH >= 170}
        <div class="audio-art-wrap">
          <div class="audio-art" class:ap-playing={audioPlaying}>
            <div class="eq-bars" class:active={audioPlaying}>
              <span></span><span></span><span></span><span></span><span></span>
            </div>
          </div>
        </div>
      {/if}

      <!-- Track info — hidden in mini mode -->
      {#if audioPlayerH >= 110}
        <div class="ap-info">
          <p class="ap-title" title={data.name}>{audioTitle || data.name}</p>
          {#if audioPlayerH >= 140}
            <p class="ap-sub">{formatSize(data.size)}</p>
          {/if}
        </div>
      {/if}

      <!-- Progress bar — hidden only in mini mode -->
      {#if audioPlayerH >= 90}
        <div class="ap-seek-row">
          <span class="ap-ts">{formatAudioTime(audioCurrentTime)}</span>
          <!-- svelte-ignore a11y-no-static-element-interactions -->
          <div class="ap-seek-track" role="slider" tabindex="0"
            aria-valuenow={audioCurrentTime} aria-valuemin={0} aria-valuemax={audioDuration}
            aria-label="Progreso de reproducción"
            on:click={onAudioSeek}
            on:keydown={e => { if (e.key === 'ArrowRight') { audioEl.currentTime = Math.min(audioDuration, audioEl.currentTime + 5); } if (e.key === 'ArrowLeft') { audioEl.currentTime = Math.max(0, audioEl.currentTime - 5); } }}
          >
            <div class="ap-seek-fill" style="width:{audioDuration > 0 ? (audioCurrentTime / audioDuration * 100).toFixed(2) : 0}%">
              <div class="ap-seek-thumb"></div>
            </div>
          </div>
          <span class="ap-ts">{formatAudioTime(audioDuration)}</span>
        </div>
      {/if}

      <!-- Controls — always visible -->
      <div class="ap-controls">
        {#if audioPlayerH >= 90}
          <button class="ap-skip-btn" title="−10s"
            on:click={() => audioEl && (audioEl.currentTime = Math.max(0, audioEl.currentTime - 10))}>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M6 6h2v12H6zm3.5 6 8.5 6V6z"/></svg>
          </button>
        {/if}
        <button class="ap-play-btn" on:click={toggleAudio} aria-label={audioPlaying ? 'Pausar' : 'Reproducir'}>
          {#if audioPlaying}
            <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor"><rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/></svg>
          {:else}
            <svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg>
          {/if}
        </button>
        {#if audioPlayerH >= 90}
          <button class="ap-skip-btn" title="+10s"
            on:click={() => audioEl && (audioEl.currentTime = Math.min(audioDuration, audioEl.currentTime + 10))}>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M18 6h-2v12h2zm-3.5 6L6 6v12z"/></svg>
          </button>
        {/if}
      </div>

      <!-- Hidden native audio -->
      <!-- svelte-ignore a11y-media-has-caption -->
      <audio bind:this={audioEl} src={streamSrc(data.stored_path)}
        on:timeupdate={() => { audioCurrentTime = audioEl.currentTime; }}
        on:loadedmetadata={() => { audioDuration = audioEl.duration; }}
        on:play={() => { audioPlaying = true; }}
        on:pause={() => { audioPlaying = false; }}
        on:ended={() => { audioPlaying = false; audioCurrentTime = 0; }}
      ></audio>
    </div>

  {:else}
    <div class="empty-file">
      <span>📄</span>
      <p>{data.name}</p>
      <p class="size">{formatSize(data.size)}</p>
    </div>
  {/if}
</div>

<style>
  .file-block { display: flex; flex-direction: column; flex: 1; overflow: hidden; }

  .empty-file {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 8px;
    text-align: center; padding: 20px;
    background: linear-gradient(160deg, var(--bg-surface) 60%, var(--bg-overlay));
  }
  .empty-file span { font-size: 52px; filter: drop-shadow(0 3px 8px rgba(0,0,0,0.12)); line-height: 1; }
  .empty-file p { color: var(--text-secondary); font-size: 13px; font-weight: 600; max-width: 150px; word-break: break-word; line-height: 1.4; }
  .size { font-size: 11px !important; color: var(--text-muted) !important; font-weight: 400 !important; }

  .file-img  { flex: 1; width: 100%; height: 100%; object-fit: cover; object-position: top; min-height: 0; display: block; }
  .file-video { flex: 1; width: 100%; height: 100%; min-height: 0; display: block; }
  .file-video::-webkit-media-controls-overflow-button { display: none; }

  /* ── PDF placeholder (before user opens) ── */
  .pdf-placeholder {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 8px; padding: 20px;
    background: var(--bg-overlay);
  }
  .pdf-ph-icon { font-size: 44px; line-height: 1; filter: drop-shadow(0 2px 6px rgba(0,0,0,0.15)); }
  .pdf-ph-name {
    font-size: 13px; font-weight: 600; color: var(--text-primary);
    text-align: center; word-break: break-all; max-width: 100%;
    overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical;
  }
  .pdf-ph-size { font-size: 11px; color: var(--text-muted); }
  .pdf-ph-btn {
    margin-top: 4px;
    padding: 7px 20px; border-radius: var(--radius-sm);
    background: var(--accent); color: #fff;
    font-size: 13px; font-weight: 500;
    transition: opacity 0.15s;
  }
  .pdf-ph-btn:hover { opacity: 0.88; }

  /* ── PDF viewer (after opened) ── */
  .pdf-wrapper {
    flex: 1; display: flex; flex-direction: column;
    overflow: hidden; min-height: 0; background: #525659; position: relative;
  }
  .pdf-btn-group {
    position: absolute; top: 8px; right: 8px; z-index: 10;
    display: flex; gap: 4px;
    opacity: 0; transition: opacity 0.2s;
  }
  .pdf-wrapper:hover .pdf-btn-group { opacity: 1; }
  .pdf-action-btn {
    background: rgba(0,0,0,0.55); color: #fff;
    border: 1px solid rgba(255,255,255,0.2); border-radius: 6px;
    padding: 4px 8px; font-size: 13px; line-height: 1;
    backdrop-filter: blur(4px); cursor: pointer;
  }
  .pdf-action-btn:hover { background: rgba(0,0,0,0.75); }
  .file-pdf { flex: 1; width: 100%; height: 100%; border: none; min-height: 0; display: block; }

  /* ── Audio player (transparent, Spotify-style) ── */
  .audio-player {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: space-between;
    padding: 12px 14px 12px; gap: 8px; overflow: hidden;
    background: transparent;
  }
  /* Compact mode: tighter padding */
  .audio-player.ap-compact { padding: 8px 12px; gap: 6px; }
  /* Mini mode: minimal padding, just controls */
  .audio-player.ap-mini { padding: 4px 10px; gap: 4px; justify-content: center; }

  /* Album art — grows to fill available space but stays square */
  .audio-art-wrap {
    flex: 1; min-height: 0; width: 100%;
    display: flex; align-items: center; justify-content: center;
  }
  .audio-art {
    width: min(100%, 200px);
    aspect-ratio: 1 / 1;
    border-radius: 14px;
    background: linear-gradient(135deg, #6366f1 0%, #a855f7 100%);
    display: flex; align-items: center; justify-content: center;
    box-shadow: 0 8px 28px rgba(99,102,241,0.35);
    transition: box-shadow 0.3s, transform 0.3s;
    flex-shrink: 0;
  }
  .audio-art.ap-playing {
    box-shadow: 0 12px 40px rgba(99,102,241,0.6);
    animation: ap-pulse 2s ease-in-out infinite;
  }
  @keyframes ap-pulse {
    0%, 100% { transform: scale(1); }
    50%       { transform: scale(1.04); }
  }

  /* Equalizer bars */
  .eq-bars {
    display: flex; align-items: flex-end; gap: clamp(2px, 3%, 5px);
    height: 40%; width: 50%; padding-bottom: 4px;
  }
  .eq-bars span {
    flex: 1; border-radius: 2px;
    background: rgba(255,255,255,0.85);
    height: 8%; min-height: 3px;
  }
  .eq-bars.active span:nth-child(1) { animation: eq 0.75s ease-in-out infinite; }
  .eq-bars.active span:nth-child(2) { animation: eq 0.55s ease-in-out infinite 0.12s; }
  .eq-bars.active span:nth-child(3) { animation: eq 0.95s ease-in-out infinite 0.24s; }
  .eq-bars.active span:nth-child(4) { animation: eq 0.65s ease-in-out infinite 0.08s; }
  .eq-bars.active span:nth-child(5) { animation: eq 0.85s ease-in-out infinite 0.18s; }
  @keyframes eq {
    0%, 100% { height: 8%; opacity: 0.5; }
    50%       { height: 80%; opacity: 1; }
  }

  /* Track info */
  .ap-info { text-align: center; width: 100%; flex-shrink: 0; }
  .ap-title {
    font-size: clamp(11px, 4%, 14px); font-weight: 700; color: var(--text-primary);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    max-width: 100%; letter-spacing: 0.1px;
  }
  .ap-sub { font-size: clamp(9px, 3%, 11px); color: var(--text-muted); margin-top: 2px; }

  /* Seek bar */
  .ap-seek-row {
    display: flex; align-items: center; gap: 6px; width: 100%; flex-shrink: 0;
  }
  .ap-ts { font-size: 10px; color: var(--text-muted); white-space: nowrap; min-width: 24px; }
  .ap-ts:last-child { text-align: right; }
  .ap-seek-track {
    flex: 1; height: 4px; border-radius: 2px;
    background: var(--border); cursor: pointer; position: relative;
    transition: height 0.15s;
  }
  .ap-seek-track:hover { height: 6px; }
  .ap-seek-fill {
    height: 100%; border-radius: inherit;
    background: var(--accent); position: relative;
    transition: width 0.1s linear;
  }
  .ap-seek-thumb {
    position: absolute; right: -5px; top: 50%; transform: translateY(-50%);
    width: 10px; height: 10px; border-radius: 50%; background: var(--accent);
    opacity: 0; transition: opacity 0.15s;
  }
  .ap-seek-track:hover .ap-seek-thumb { opacity: 1; }

  /* Controls */
  .ap-controls { display: flex; align-items: center; gap: 12px; flex-shrink: 0; }
  .ap-skip-btn {
    background: none; border: none; color: var(--text-secondary);
    cursor: pointer; padding: 4px; border-radius: 50%;
    display: flex; align-items: center; justify-content: center;
    transition: color 0.15s, transform 0.1s;
  }
  .ap-skip-btn:hover { color: var(--text-primary); transform: scale(1.1); }
  .ap-play-btn {
    width: 40px; height: 40px; border-radius: 50%;
    background: var(--accent); color: #fff; border: none;
    display: flex; align-items: center; justify-content: center;
    cursor: pointer; flex-shrink: 0;
    box-shadow: 0 4px 14px rgba(99,102,241,0.4);
    transition: transform 0.15s, box-shadow 0.15s;
  }
  .ap-play-btn:hover { transform: scale(1.06); box-shadow: 0 6px 20px rgba(99,102,241,0.6); }
  .ap-play-btn:active { transform: scale(0.96); }

  /* ── Word ── */
  .doc-wrapper { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0; }

  .doc-toolbar {
    display: flex; align-items: center; justify-content: space-between;
    padding: 4px 10px; background: var(--bg-surface);
    border-bottom: 1px solid var(--border); flex-shrink: 0; gap: 6px;
  }
  .doc-actions { display: flex; gap: 4px; flex-shrink: 0; }
  .doc-btn {
    padding: 3px 8px; font-size: 11px; border-radius: 4px;
    background: var(--bg-overlay); color: var(--text-secondary);
    border: 1px solid var(--border); cursor: pointer;
    transition: all 0.15s; white-space: nowrap;
  }
  .doc-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .doc-btn.accent { background: var(--accent); color: #fff; border-color: var(--accent); }
  .doc-btn.accent:hover { opacity: 0.85; }

  .doc-loading {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: var(--text-muted); font-size: 13px;
  }

  .word-view {
    flex: 1; overflow-y: auto; padding: 20px 24px;
    background: #fff; color: #1a1a1a;
    font-family: Georgia, 'Times New Roman', serif; font-size: 14px; line-height: 1.7;
  }
  /* Styles for mammoth HTML output */
  .word-view :global(h1) { font-size: 22px; font-weight: 700; margin: 16px 0 8px; }
  .word-view :global(h2) { font-size: 18px; font-weight: 700; margin: 14px 0 6px; }
  .word-view :global(h3) { font-size: 15px; font-weight: 600; margin: 12px 0 4px; }
  .word-view :global(p)  { margin: 0 0 8px; }
  .word-view :global(ul), .word-view :global(ol) { padding-left: 24px; margin: 8px 0; }
  .word-view :global(table) { border-collapse: collapse; margin: 12px 0; width: 100%; }
  .word-view :global(td), .word-view :global(th) { border: 1px solid #ccc; padding: 4px 8px; font-size: 13px; }

  .word-editor {
    flex: 1; overflow-y: auto; padding: 20px 24px;
    background: #fff; color: #1a1a1a;
    font-family: Georgia, 'Times New Roman', serif; font-size: 14px; line-height: 1.7;
    outline: none; border: 2px solid var(--accent);
    min-height: 0;
  }

  /* ── Excel ── */
  .excel-wrapper { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0; }

  .sheet-tabs-inline {
    display: flex; gap: 2px; overflow-x: auto; scrollbar-width: none; flex: 1;
  }
  .sheet-tab {
    padding: 3px 12px; font-size: 11px; border-radius: 4px;
    background: var(--bg-overlay); color: var(--text-muted);
    border: 1px solid var(--border); cursor: pointer;
    white-space: nowrap; transition: all 0.15s;
  }
  .sheet-tab.active { background: var(--accent-dim); color: var(--accent); border-color: var(--accent); }
  .sheet-tab:hover:not(.active) { color: var(--text-primary); background: var(--bg-hover); }

  .excel-scroll { flex: 1; overflow: auto; background: #fff; min-height: 0; }

  .excel-table {
    border-collapse: collapse; font-size: 12px; font-family: monospace;
    color: #111; white-space: nowrap; width: max-content;
  }
  .excel-row-header {
    background: #f3f4f6; font-weight: 600; font-size: 11px; color: #666;
    border: 1px solid #d0d0d0; padding: 2px 6px; text-align: center;
    min-width: 32px; position: sticky; left: 0; z-index: 2;
    user-select: none;
  }
  thead .excel-row-header { position: sticky; top: 0; left: 0; z-index: 3; }
  .excel-col-header {
    background: #f3f4f6; font-weight: 600; font-size: 11px; color: #666;
    border: 1px solid #d0d0d0; padding: 2px 8px; text-align: center;
    min-width: 80px; position: sticky; top: 0; z-index: 1;
    user-select: none;
  }
  .excel-cell {
    border: 1px solid #e0e0e0; padding: 0; min-width: 80px;
  }
  .cell-input {
    width: 100%; min-width: 80px; padding: 2px 6px;
    border: none; outline: none; background: transparent;
    font-size: 12px; font-family: monospace; color: #111;
    height: 22px; box-sizing: border-box;
  }
  .cell-input:focus { background: #fffbea; outline: 2px solid #4a90e2; outline-offset: -1px; }

  /* ── PowerPoint card ── */
  .ppt-card {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 14px; padding: 20px 16px;
  }
  .ppt-icon-wrap { flex-shrink: 0; }
  .ppt-icon { width: 64px; height: 64px; display: block; filter: drop-shadow(0 4px 12px rgba(196,62,28,0.35)); }
  .ppt-info { text-align: center; width: 100%; }
  .ppt-name {
    font-size: 13px; font-weight: 700; color: var(--text-primary);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    max-width: 100%; margin: 0 0 4px;
  }
  .ppt-meta { font-size: 11px; color: var(--text-muted); margin: 0; }
  .ppt-open-btn {
    display: flex; align-items: center; gap: 7px;
    padding: 9px 18px; border-radius: 9px;
    background: #C43E1C; color: #fff;
    font-size: 13px; font-weight: 600;
    box-shadow: 0 4px 14px rgba(196,62,28,0.35);
    transition: opacity 0.15s, transform 0.1s;
    flex-shrink: 0;
  }
  .ppt-open-btn:hover { opacity: 0.88; transform: translateY(-1px); }
  .ppt-open-btn:active { transform: translateY(0); }
</style>
