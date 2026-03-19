<script lang="ts">
  import { onDestroy } from "svelte";
  import type { Block } from "../../types";

  export let block: Block;
  export let zoom: number = 1;
  export let selected: boolean = false;
  export let drawMode: boolean = false;
  export let multiSelected: boolean = false;
  export let allBlocks: Block[] = [];
  export let onUpdate: (id: string, changes: Partial<Block>) => void;
  export let onDelete: (id: string) => void;
  export let onSelect: (id: string) => void;
  export let onBringToFront: (id: string) => void;

  // ── Shape data ────────────────────────────────────────
  type ShapeType = "rect" | "circle" | "triangle" | "diamond" | "line" | "arrow";
  interface ShapeData {
    shape: ShapeType; fill: string; stroke: string;
    strokeWidth: number; text: string; rotation: number;
    // Connector fields (line/arrow only):
    x1?: number; y1?: number; x2?: number; y2?: number;
    startId?: string | null; endId?: string | null;
  }

  function parse(s: string): ShapeData {
    try {
      return { shape: "rect", fill: "#ffffff", stroke: "#1e1e2e", strokeWidth: 2, text: "", rotation: 0, ...JSON.parse(s || "{}") };
    } catch {
      return { shape: "rect", fill: "#ffffff", stroke: "#1e1e2e", strokeWidth: 2, text: "", rotation: 0 };
    }
  }

  $: data = parse(block.content);
  $: isConnector = data.shape === "line" || data.shape === "arrow";

  // ── Local geometry (init once, updated locally during drag/resize) ──
  let lx = block.x, ly = block.y, lw = block.width, lh = block.height;
  // Init rotation from content once; updated locally during rotate, saved on end
  let localRotation: number = parse(block.content).rotation ?? 0;

  // Sync from parent for non-connectors
  $: if (!dragging && !resizing && !isConnector) { lx = block.x; ly = block.y; lw = block.width; lh = block.height; }

  const TRIANGLE = "M50,4 L96,88 L4,88 Z";
  const DIAMOND  = "M50,4 L96,50 L50,96 L4,50 Z";

  // ── Shape element ref (needed for rotation center) ────
  let shapeEl: HTMLDivElement;

  // ── Drag ─────────────────────────────────────────────
  let dragging = false;
  let dragSX = 0, dragSY = 0, dragBX = 0, dragBY = 0;

  function onShapePointerDown(e: PointerEvent) {
    if (drawMode || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    onSelect(block.id);
    onBringToFront(block.id);
    dragging = true;
    dragSX = e.clientX; dragSY = e.clientY;
    dragBX = lx; dragBY = ly;
    window.addEventListener("pointermove", onDragMove);
    window.addEventListener("pointerup", onDragEnd);
    window.addEventListener("pointercancel", onDragEnd);
  }

  function onDragMove(e: PointerEvent) {
    if (!dragging) return;
    lx = Math.max(0, dragBX + (e.clientX - dragSX) / zoom);
    ly = Math.max(0, dragBY + (e.clientY - dragSY) / zoom);
  }

  function onDragEnd() {
    if (!dragging) return;
    dragging = false;
    window.removeEventListener("pointermove", onDragMove);
    window.removeEventListener("pointerup", onDragEnd);
    window.removeEventListener("pointercancel", onDragEnd);
    onUpdate(block.id, { x: lx, y: ly });
  }

  // ── Resize ────────────────────────────────────────────
  type Handle = "se" | "sw" | "ne" | "nw";
  let resizing = false;
  let resizeHandle: Handle = "se";
  let resSX = 0, resSY = 0, resBX = 0, resBY = 0, resBW = 0, resBH = 0;
  const MIN = 32;

  function onHandlePointerDown(e: PointerEvent, handle: Handle) {
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    resizeHandle = handle;
    resSX = e.clientX; resSY = e.clientY;
    resBX = lx; resBY = ly; resBW = lw; resBH = lh;
    window.addEventListener("pointermove", onResizeMove);
    window.addEventListener("pointerup", onResizeEnd);
    window.addEventListener("pointercancel", onResizeEnd);
  }

  function onResizeMove(e: PointerEvent) {
    if (!resizing) return;
    const dx = (e.clientX - resSX) / zoom;
    const dy = (e.clientY - resSY) / zoom;
    if (resizeHandle === "se") {
      lw = Math.max(MIN, resBW + dx);
      lh = Math.max(MIN, resBH + dy);
    } else if (resizeHandle === "sw") {
      const nw = Math.max(MIN, resBW - dx);
      lx = resBX + (resBW - nw); lw = nw;
      lh = Math.max(MIN, resBH + dy);
    } else if (resizeHandle === "ne") {
      lw = Math.max(MIN, resBW + dx);
      const nh = Math.max(MIN, resBH - dy);
      ly = resBY + (resBH - nh); lh = nh;
    } else {
      const nw = Math.max(MIN, resBW - dx);
      const nh = Math.max(MIN, resBH - dy);
      lx = resBX + (resBW - nw); ly = resBY + (resBH - nh);
      lw = nw; lh = nh;
    }
  }

  function onResizeEnd() {
    if (!resizing) return;
    resizing = false;
    window.removeEventListener("pointermove", onResizeMove);
    window.removeEventListener("pointerup", onResizeEnd);
    window.removeEventListener("pointercancel", onResizeEnd);
    onUpdate(block.id, { x: lx, y: ly, width: lw, height: lh });
  }

  // ── Rotate ────────────────────────────────────────────
  let rotating = false;
  let rotCenterX = 0, rotCenterY = 0, rotStartAngle = 0, rotBaseAngle = 0;

  function onRotateHandleDown(e: PointerEvent) {
    e.preventDefault();
    e.stopPropagation();
    rotating = true;
    const rect = shapeEl.getBoundingClientRect();
    rotCenterX = rect.left + rect.width / 2;
    rotCenterY = rect.top + rect.height / 2;
    rotStartAngle = Math.atan2(e.clientY - rotCenterY, e.clientX - rotCenterX) * 180 / Math.PI;
    rotBaseAngle = localRotation;
    window.addEventListener("pointermove", onRotateMove);
    window.addEventListener("pointerup", onRotateEnd);
    window.addEventListener("pointercancel", onRotateEnd);
  }

  function onRotateMove(e: PointerEvent) {
    if (!rotating) return;
    const angle = Math.atan2(e.clientY - rotCenterY, e.clientX - rotCenterX) * 180 / Math.PI;
    localRotation = rotBaseAngle + (angle - rotStartAngle);
  }

  function onRotateEnd() {
    if (!rotating) return;
    rotating = false;
    window.removeEventListener("pointermove", onRotateMove);
    window.removeEventListener("pointerup", onRotateEnd);
    window.removeEventListener("pointercancel", onRotateEnd);
    onUpdate(block.id, { content: JSON.stringify({ ...data, rotation: localRotation }) });
  }

  // ── Text editing ──────────────────────────────────────
  let editing = false;
  let editText = "";
  let textareaEl: HTMLTextAreaElement;

  function onDblClick(e: MouseEvent) {
    e.stopPropagation();
    editText = data.text;
    editing = true;
    setTimeout(() => textareaEl?.focus(), 0);
  }

  function commitText() {
    editing = false;
    onUpdate(block.id, { content: JSON.stringify({ ...data, rotation: localRotation, text: editText }) });
  }

  // ── Connector local state ─────────────────────────────
  let ex1 = 0, ey1 = 0, ex2 = 0, ey2 = 0;
  // Sync from data when not dragging
  $: if (!endpointDragging && isConnector) {
    ex1 = data.x1 ?? 0; ey1 = data.y1 ?? 0;
    ex2 = data.x2 ?? 0; ey2 = data.y2 ?? 0;
  }
  const PAD = 20;
  $: svgLeft   = Math.min(ex1, ex2) - PAD;
  $: svgTop    = Math.min(ey1, ey2) - PAD;
  $: svgWidth  = Math.abs(ex2 - ex1) + PAD * 2;
  $: svgHeight = Math.abs(ey2 - ey1) + PAD * 2;
  $: lsvgX1 = ex1 - svgLeft;
  $: lsvgY1 = ey1 - svgTop;
  $: lsvgX2 = ex2 - svgLeft;
  $: lsvgY2 = ey2 - svgTop;
  // Midpoint for delete button
  $: midX = (ex1 + ex2) / 2;
  $: midY = (ey1 + ey2) / 2;

  // ── Connector drag handling ───────────────────────────
  let endpointDragging: "start" | "end" | "body" | null = null;
  let connDragStartClient = { x: 0, y: 0 };
  let connDragStartEx1 = 0, connDragStartEy1 = 0, connDragStartEx2 = 0, connDragStartEy2 = 0;
  let snapTarget: Block | null = null;

  const SNAP_PAD = 28;
  function findNearestSnap(cx: number, cy: number): Block | null {
    for (const b of allBlocks) {
      if (b.id === block.id) continue;
      if (b.block_type === "shape") {
        try { const d = JSON.parse(b.content); if (d.shape === "line" || d.shape === "arrow") continue; } catch {}
      }
      if (cx >= b.x - SNAP_PAD && cx <= b.x + b.width  + SNAP_PAD &&
          cy >= b.y - SNAP_PAD && cy <= b.y + b.height + SNAP_PAD) return b;
    }
    return null;
  }

  function startConnectorDrag(e: PointerEvent, mode: "start" | "end" | "body") {
    if (drawMode) return;
    e.preventDefault(); e.stopPropagation();
    if (mode === "body") { onSelect(block.id); onBringToFront(block.id); }
    endpointDragging = mode;
    connDragStartClient = { x: e.clientX, y: e.clientY };
    connDragStartEx1 = ex1; connDragStartEy1 = ey1;
    connDragStartEx2 = ex2; connDragStartEy2 = ey2;
    window.addEventListener("pointermove", onConnectorMove);
    window.addEventListener("pointerup", onConnectorUp);
    window.addEventListener("pointercancel", onConnectorUp);
  }

  function onConnectorMove(e: PointerEvent) {
    if (!endpointDragging) return;
    const dx = (e.clientX - connDragStartClient.x) / zoom;
    const dy = (e.clientY - connDragStartClient.y) / zoom;
    if (endpointDragging === "body") {
      ex1 = Math.max(0, connDragStartEx1 + dx); ey1 = Math.max(0, connDragStartEy1 + dy);
      ex2 = Math.max(0, connDragStartEx2 + dx); ey2 = Math.max(0, connDragStartEy2 + dy);
      snapTarget = null;
    } else {
      const baseX = endpointDragging === "start" ? connDragStartEx1 : connDragStartEx2;
      const baseY = endpointDragging === "start" ? connDragStartEy1 : connDragStartEy2;
      let nx = baseX + dx, ny = baseY + dy;
      snapTarget = findNearestSnap(nx, ny);
      if (snapTarget) { nx = snapTarget.x + snapTarget.width/2; ny = snapTarget.y + snapTarget.height/2; }
      if (endpointDragging === "start") { ex1 = nx; ey1 = ny; }
      else { ex2 = nx; ey2 = ny; }
    }
  }

  async function onConnectorUp() {
    if (!endpointDragging) return;
    const mode = endpointDragging;
    endpointDragging = null;
    const snapped = snapTarget;
    snapTarget = null;
    const newData = {
      ...data, x1: ex1, y1: ey1, x2: ex2, y2: ey2,
      startId: mode === "start" ? (snapped?.id ?? null) : (mode === "body" ? null : data.startId),
      endId:   mode === "end"   ? (snapped?.id ?? null) : (mode === "body" ? null : data.endId),
    };
    const nx = Math.min(ex1,ex2), ny = Math.min(ey1,ey2);
    const nw = Math.max(Math.abs(ex2-ex1),10), nh = Math.max(Math.abs(ey2-ey1),10);
    onUpdate(block.id, { content: JSON.stringify(newData), x: nx, y: ny, width: nw, height: nh });
    window.removeEventListener("pointermove", onConnectorMove);
    window.removeEventListener("pointerup", onConnectorUp);
    window.removeEventListener("pointercancel", onConnectorUp);
  }

  onDestroy(() => {
    window.removeEventListener("pointermove", onDragMove);
    window.removeEventListener("pointerup", onDragEnd);
    window.removeEventListener("pointercancel", onDragEnd);
    window.removeEventListener("pointermove", onResizeMove);
    window.removeEventListener("pointerup", onResizeEnd);
    window.removeEventListener("pointercancel", onResizeEnd);
    window.removeEventListener("pointermove", onRotateMove);
    window.removeEventListener("pointerup", onRotateEnd);
    window.removeEventListener("pointercancel", onRotateEnd);
    window.removeEventListener("pointermove", onConnectorMove);
    window.removeEventListener("pointerup", onConnectorUp);
    window.removeEventListener("pointercancel", onConnectorUp);
  });
</script>

{#if isConnector}
  <!-- SVG connector -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <svg
    class="connector-svg"
    class:conn-selected={selected}
    style="position:absolute; left:{svgLeft}px; top:{svgTop}px; overflow:visible; pointer-events:none; z-index:{block.z_index};"
    width={svgWidth} height={svgHeight}
  >
    <defs>
      {#if data.shape === "arrow"}
        <marker id="ah-{block.id}" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
          <polygon points="0 0, 10 3.5, 0 7" fill={data.stroke} />
        </marker>
      {/if}
    </defs>

    <!-- Selection glow -->
    {#if selected}
      <line x1={lsvgX1} y1={lsvgY1} x2={lsvgX2} y2={lsvgY2}
            stroke="var(--accent)" stroke-width={data.strokeWidth + 6}
            stroke-linecap="round" opacity="0.25" style="pointer-events:none" />
    {/if}

    <!-- Actual line -->
    <line x1={lsvgX1} y1={lsvgY1} x2={lsvgX2} y2={lsvgY2}
          stroke={data.stroke} stroke-width={data.strokeWidth}
          stroke-linecap="round"
          marker-end={data.shape === "arrow" ? `url(#ah-${block.id})` : ""}
          style="pointer-events:none" />

    <!-- Hit area (transparent thick line for clicking) -->
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <line x1={lsvgX1} y1={lsvgY1} x2={lsvgX2} y2={lsvgY2}
          stroke="transparent" stroke-width="20"
          style="pointer-events:stroke; cursor:{endpointDragging==='body'?'grabbing':'grab'}"
          on:pointerdown={e => startConnectorDrag(e, "body")} />

    <!-- Snap highlight -->
    {#if snapTarget}
      <circle
        cx={snapTarget.x + snapTarget.width/2 - svgLeft}
        cy={snapTarget.y + snapTarget.height/2 - svgTop}
        r="20" fill="rgba(99,102,241,0.12)" stroke="var(--accent)"
        stroke-width="2" stroke-dasharray="5 3" style="pointer-events:none" />
    {/if}

    <!-- Endpoint handles (when selected) -->
    {#if selected && !drawMode}
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <circle cx={lsvgX1} cy={lsvgY1} r="7"
              fill="white" stroke="var(--accent)" stroke-width="2.5"
              style="pointer-events:all; cursor:grab"
              on:pointerdown={e => startConnectorDrag(e, "start")} />
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <circle cx={lsvgX2} cy={lsvgY2} r="7"
              fill="white" stroke="var(--accent)" stroke-width="2.5"
              style="pointer-events:all; cursor:grab"
              on:pointerdown={e => startConnectorDrag(e, "end")} />
      <!-- Connection indicator dots (filled when connected) -->
      {#if data.startId}
        <circle cx={lsvgX1} cy={lsvgY1} r="4" fill="var(--accent)" style="pointer-events:none" />
      {/if}
      {#if data.endId}
        <circle cx={lsvgX2} cy={lsvgY2} r="4" fill="var(--accent)" style="pointer-events:none" />
      {/if}
    {/if}
  </svg>

  <!-- Delete button (outside SVG) -->
  {#if selected && !drawMode}
    <div class="connector-ctrl" style="left:{midX}px; top:{midY - 28}px;">
      <button class="del-btn" on:click|stopPropagation={() => onDelete(block.id)} title="Eliminar">×</button>
    </div>
  {/if}

{:else}
<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  bind:this={shapeEl}
  class="shape-root"
  class:selected
  class:dragging
  class:rotating
  style="left:{lx}px; top:{ly}px; width:{lw}px; height:{lh}px; z-index:{block.z_index}; transform: rotate({localRotation}deg); transform-origin: center;"
  on:pointerdown={onShapePointerDown}
  on:dblclick={onDblClick}
>
  <!-- SVG shape -->
  <svg viewBox="0 0 100 100" preserveAspectRatio="none" class="shape-svg">
    {#if data.shape === "rect"}
      <rect x="2" y="2" width="96" height="96" rx="6"
        fill={data.fill} stroke={data.stroke} stroke-width={data.strokeWidth} />
    {:else if data.shape === "circle"}
      <ellipse cx="50" cy="50" rx="48" ry="48"
        fill={data.fill} stroke={data.stroke} stroke-width={data.strokeWidth} />
    {:else if data.shape === "triangle"}
      <path d={TRIANGLE}
        fill={data.fill} stroke={data.stroke} stroke-width={data.strokeWidth} stroke-linejoin="round" />
    {:else if data.shape === "diamond"}
      <path d={DIAMOND}
        fill={data.fill} stroke={data.stroke} stroke-width={data.strokeWidth} stroke-linejoin="round" />
    {/if}
  </svg>

  <!-- Text overlay -->
  {#if editing}
    <div class="text-overlay editing" on:click|stopPropagation>
      <textarea
        bind:this={textareaEl}
        bind:value={editText}
        class="text-editor"
        on:blur={commitText}
        on:keydown={e => { if (e.key === "Escape") commitText(); }}
        placeholder="Escribe aquí…"
      />
    </div>
  {:else if data.text}
    <div class="text-overlay">
      <span class="text-display">{data.text}</span>
    </div>
  {/if}

  <!-- Selection controls (only when selected) -->
  {#if selected && !drawMode}
    <!-- Controls row: delete + rotate, floating above center -->
    <div class="ctrl-row">
      <button class="del-btn" on:click|stopPropagation={() => onDelete(block.id)} title="Eliminar">×</button>
      <div class="rotate-handle" on:pointerdown={onRotateHandleDown} title="Rotar">↻</div>
    </div>

    <!-- Resize handles: 4 corners -->
    <div class="handle nw" on:pointerdown={e => onHandlePointerDown(e, "nw")} />
    <div class="handle ne" on:pointerdown={e => onHandlePointerDown(e, "ne")} />
    <div class="handle sw" on:pointerdown={e => onHandlePointerDown(e, "sw")} />
    <div class="handle se" on:pointerdown={e => onHandlePointerDown(e, "se")} />
  {/if}
</div>
{/if}

<style>
  .shape-root {
    position: absolute;
    cursor: grab;
    user-select: none;
    touch-action: none;
    overflow: visible;
  }
  .shape-root:active, .shape-root.dragging { cursor: grabbing; }

  /* Selection outline */
  .shape-root.selected::before {
    content: '';
    position: absolute;
    inset: -4px;
    border: 1.5px solid var(--accent);
    border-radius: 4px;
    pointer-events: none;
    opacity: 0.7;
  }

  .shape-svg {
    width: 100%; height: 100%;
    display: block;
    pointer-events: all;
  }

  /* Text */
  .text-overlay {
    position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
    pointer-events: none;
  }
  .text-overlay.editing { pointer-events: all; }

  .text-display {
    max-width: 80%; text-align: center; word-break: break-word;
    font-size: clamp(11px, 3cqw, 16px); font-weight: 600;
    color: #fff; text-shadow: 0 1px 4px rgba(0,0,0,0.65);
    pointer-events: none; line-height: 1.3;
  }

  .text-editor {
    width: 70%; min-height: 40px; resize: none;
    background: rgba(0,0,0,0.45); color: #fff;
    border: 1px solid rgba(255,255,255,0.4); border-radius: 4px;
    padding: 4px 8px; font-size: 13px; font-weight: 600;
    text-align: center; outline: none; backdrop-filter: blur(4px);
  }

  /* Controls row: floats above the shape center */
  .ctrl-row {
    position: absolute;
    bottom: calc(100% + 14px); left: 50%;
    transform: translateX(-50%);
    display: flex; align-items: center; gap: 6px;
    background: rgba(255,255,255,0.92);
    border: 1px solid rgba(0,0,0,0.1);
    border-radius: 20px;
    padding: 4px 8px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.15);
    z-index: 12;
  }
  /* Line from pill down to shape edge */
  .ctrl-row::after {
    content: '';
    position: absolute;
    top: 100%; left: 50%;
    transform: translateX(-50%);
    width: 1px; height: 14px;
    background: var(--accent);
    opacity: 0.5;
  }

  /* Delete button */
  .del-btn {
    width: 22px; height: 22px; border-radius: 50%;
    background: transparent;
    border: 1.5px solid rgba(150,150,165,0.5);
    color: rgba(120,120,140,0.85);
    font-size: 14px; font-weight: 500; line-height: 1;
    display: flex; align-items: center; justify-content: center;
    transition: all 0.15s;
    cursor: pointer; flex-shrink: 0; padding: 0;
  }
  .del-btn:hover {
    border-color: var(--red);
    color: var(--red);
    background: rgba(239,68,68,0.08);
  }

  /* Rotate handle */
  .rotate-handle {
    width: 22px; height: 22px; border-radius: 50%;
    background: transparent;
    border: 1.5px solid var(--accent);
    color: var(--accent);
    font-size: 14px; line-height: 1;
    display: flex; align-items: center; justify-content: center;
    cursor: grab; flex-shrink: 0;
    transition: background 0.15s, color 0.15s;
    user-select: none;
  }
  .rotate-handle:hover { background: rgba(99,102,241,0.1); }
  .shape-root.rotating { cursor: grabbing; }
  .shape-root.rotating .rotate-handle { cursor: grabbing; }

  /* Resize handles */
  .handle {
    position: absolute;
    width: 10px; height: 10px; border-radius: 2px;
    background: #fff; border: 2px solid var(--accent);
    box-shadow: 0 1px 4px rgba(0,0,0,0.35);
    z-index: 10;
  }
  .handle.nw { top: -5px;  left: -5px;  cursor: nw-resize; }
  .handle.ne { top: -5px;  right: -5px; cursor: ne-resize; }
  .handle.sw { bottom: -5px; left: -5px;  cursor: sw-resize; }
  .handle.se { bottom: -5px; right: -5px; cursor: se-resize; }
  .handle:hover { background: var(--accent); }

  /* Connector (line/arrow) */
  .connector-ctrl {
    position: absolute; pointer-events: all;
    transform: translate(-50%, 0);
    display: flex; align-items: center; gap: 4px;
    background: rgba(255,255,255,0.92); border: 1px solid rgba(0,0,0,0.1);
    border-radius: 20px; padding: 3px 6px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.15); z-index: 999;
  }
</style>
