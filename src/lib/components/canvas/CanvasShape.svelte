<script lang="ts">
  import { onDestroy } from "svelte";
  import type { Block } from "../../types";
  import { t } from "../../stores/language";

  export let block: Block;
  export let zoom: number = 1;
  export let selected: boolean = false;
  export let drawMode: boolean = false;
  export let multiSelected: boolean = false;
  export let allBlocks: Block[] = [];
  export let onUpdate: (id: string, changes: Partial<Block>) => void;
  export let onMove: ((id: string, x: number, y: number) => void) | undefined = undefined;
  export let connectorMode: boolean = false;
  export let onConnectorStart: ((blockId: string, port: string, x: number, y: number) => void) | undefined = undefined;
  export let onDelete: (id: string) => void;
  export let onSelect: (id: string) => void;
  export let onBringToFront: (id: string) => void;

  // ── Types ─────────────────────────────────────────────
  type ShapeType = "rect" | "circle" | "triangle" | "diamond" | "heart" | "line" | "arrow";
  type Port      = "n" | "s" | "e" | "w" | "center";
  type LineStyle = "solid" | "dashed" | "dotted";

  interface ShapeData {
    shape: ShapeType;
    fill: string; stroke: string; strokeWidth: number;
    text: string; rotation: number; lineStyle?: LineStyle;
    x1?: number; y1?: number; x2?: number; y2?: number;
    startId?: string | null; endId?: string | null;
    startPort?: Port; endPort?: Port;
  }

  function parse(s: string): ShapeData {
    try {
      return {
        shape: "rect", fill: "#ffffff", stroke: "#1e1e2e",
        strokeWidth: 2, text: "", rotation: 0, lineStyle: "solid",
        ...JSON.parse(s || "{}")
      };
    } catch {
      return { shape: "rect", fill: "#ffffff", stroke: "#1e1e2e", strokeWidth: 2, text: "", rotation: 0, lineStyle: "solid" };
    }
  }

  $: data = parse(block.content);
  $: isConnector  = data.shape === "line" || data.shape === "arrow";
  $: isTextOnly   = data.fill === "transparent" && (data.stroke === "transparent" || data.strokeWidth === 0);
  // Text color: dark when background is transparent, white otherwise
  $: textColor    = isTextOnly ? "var(--text-primary)" : "#fff";
  $: textShadow   = isTextOnly ? "none" : "0 1px 4px rgba(0,0,0,0.65)";

  // ── Local geometry ────────────────────────────────────
  let lx = block.x, ly = block.y, lw = block.width, lh = block.height;
  let localRotation: number = parse(block.content).rotation ?? 0;

  $: if (!dragging && !resizing && !isConnector) {
    lx = block.x; ly = block.y; lw = block.width; lh = block.height;
  }

  const TRIANGLE = "M50,0 L100,100 L0,100 Z";
  const DIAMOND  = "M50,0 L100,50 L50,100 L0,50 Z";
  // Symmetric heart: two cubic bezier bumps meeting at the bottom point
  const HEART    = "M50,82 C50,82 8,57 8,33 C8,18 18,8 33,8 C41,8 47,12 50,18 C53,12 59,8 67,8 C82,8 92,18 92,33 C92,57 50,82 50,82 Z";

  let shapeEl: HTMLDivElement;
  let hovered = false;

  // ── Port dot connector start ──────────────────────────
  const PORT_POSITIONS: { port: string; css: string }[] = [
    { port: "n", css: "left:50%;top:0%;transform:translate(-50%,-50%)" },
    { port: "s", css: "left:50%;top:100%;transform:translate(-50%,-50%)" },
    { port: "e", css: "left:100%;top:50%;transform:translate(-50%,-50%)" },
    { port: "w", css: "left:0%;top:50%;transform:translate(-50%,-50%)" },
  ];

  function onPortPointerDown(e: PointerEvent, port: string) {
    e.stopPropagation();
    e.preventDefault();
    const cx = lx + lw / 2, cy = ly + lh / 2;
    const px = port === "w" ? lx : port === "e" ? lx + lw : cx;
    const py = port === "n" ? ly : port === "s" ? ly + lh : cy;
    onConnectorStart?.(block.id, port, px, py);
  }

  // ── Drag ─────────────────────────────────────────────
  let dragging = false;
  let dragSX = 0, dragSY = 0, dragBX = 0, dragBY = 0;

  function onShapePointerDown(e: PointerEvent) {
    if (drawMode || e.button !== 0) return;
    if (connectorMode) return; // let viewport handle connector creation — don't stopPropagation
    if (dragging) onDragEnd(); // clean stuck state
    (document.activeElement as HTMLElement)?.blur();
    e.preventDefault(); e.stopPropagation();
    onSelect(block.id); onBringToFront(block.id);
    dragging = true;
    dragSX = e.clientX; dragSY = e.clientY;
    dragBX = lx; dragBY = ly;
    window.addEventListener("pointermove", onDragMove);
    window.addEventListener("pointerup",   onDragEnd);
    window.addEventListener("pointercancel", onDragEnd);
  }

  function onDragEnd() {
    if (!dragging) return;
    dragging = false;
    window.removeEventListener("pointermove", onDragMove);
    window.removeEventListener("pointerup",   onDragEnd);
    window.removeEventListener("pointercancel", onDragEnd);
    onUpdate(block.id, { x: lx, y: ly });
  }

  function onDragMove(e: PointerEvent) {
    if (!dragging) return;
    lx = Math.max(0, dragBX + (e.clientX - dragSX) / zoom);
    ly = Math.max(0, dragBY + (e.clientY - dragSY) / zoom);
    onMove?.(block.id, lx, ly);
  }

  // ── Resize ────────────────────────────────────────────
  type Handle = "se" | "sw" | "ne" | "nw";
  let resizing = false;
  let resizeHandle: Handle = "se";
  let resSX = 0, resSY = 0, resBX = 0, resBY = 0, resBW = 0, resBH = 0;
  const MIN = 32;

  function onHandlePointerDown(e: PointerEvent, handle: Handle) {
    e.preventDefault(); e.stopPropagation();
    if (resizing) onResizeEnd();
    resizing = true; resizeHandle = handle;
    resSX = e.clientX; resSY = e.clientY;
    resBX = lx; resBY = ly; resBW = lw; resBH = lh;
    window.addEventListener("pointermove", onResizeMove);
    window.addEventListener("pointerup",   onResizeEnd);
    window.addEventListener("pointercancel", onResizeEnd);
  }

  function onResizeMove(e: PointerEvent) {
    if (!resizing) return;
    const dx = (e.clientX - resSX) / zoom;
    const dy = (e.clientY - resSY) / zoom;
    if (resizeHandle === "se") {
      lw = Math.max(MIN, resBW + dx); lh = Math.max(MIN, resBH + dy);
    } else if (resizeHandle === "sw") {
      const nw = Math.max(MIN, resBW - dx); lx = resBX + (resBW - nw); lw = nw;
      lh = Math.max(MIN, resBH + dy);
    } else if (resizeHandle === "ne") {
      lw = Math.max(MIN, resBW + dx);
      const nh = Math.max(MIN, resBH - dy); ly = resBY + (resBH - nh); lh = nh;
    } else {
      const nw = Math.max(MIN, resBW - dx); const nh = Math.max(MIN, resBH - dy);
      lx = resBX + (resBW - nw); ly = resBY + (resBH - nh); lw = nw; lh = nh;
    }
  }

  function onResizeEnd() {
    if (!resizing) return;
    resizing = false;
    window.removeEventListener("pointermove", onResizeMove);
    window.removeEventListener("pointerup",   onResizeEnd);
    window.removeEventListener("pointercancel", onResizeEnd);
    onUpdate(block.id, { x: lx, y: ly, width: lw, height: lh });
  }

  // ── Rotate ────────────────────────────────────────────
  let rotating = false;
  let rotCX = 0, rotCY = 0, rotStartA = 0, rotBaseA = 0;

  function onRotateHandleDown(e: PointerEvent) {
    e.preventDefault(); e.stopPropagation();
    if (rotating) onRotateEnd();
    rotating = true;
    const r = shapeEl.getBoundingClientRect();
    rotCX = r.left + r.width / 2; rotCY = r.top + r.height / 2;
    rotStartA = Math.atan2(e.clientY - rotCY, e.clientX - rotCX) * 180 / Math.PI;
    rotBaseA = localRotation;
    window.addEventListener("pointermove", onRotateMove);
    window.addEventListener("pointerup",   onRotateEnd);
    window.addEventListener("pointercancel", onRotateEnd);
  }

  function onRotateMove(e: PointerEvent) {
    if (!rotating) return;
    const a = Math.atan2(e.clientY - rotCY, e.clientX - rotCX) * 180 / Math.PI;
    localRotation = rotBaseA + (a - rotStartA);
  }

  function onRotateEnd() {
    if (!rotating) return;
    rotating = false;
    window.removeEventListener("pointermove", onRotateMove);
    window.removeEventListener("pointerup",   onRotateEnd);
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

  // ── Connector helpers ─────────────────────────────────
  type SnapResult = { block: Block; x: number; y: number; port: Port } | null;

  function getPortCoords(b: Block, port: Port | string | undefined): [number, number] {
    const cx = b.x + b.width / 2, cy = b.y + b.height / 2;
    switch (port) {
      case "n": return [cx, b.y];
      case "s": return [cx, b.y + b.height];
      case "e": return [b.x + b.width, cy];
      case "w": return [b.x, cy];
      default:  return [cx, cy];
    }
  }

  const SNAP_R = 40; // snap radius in canvas units
  const PORTS: Port[] = ["n", "s", "e", "w", "center"];

  function findNearestSnap(cx: number, cy: number): SnapResult {
    let best: SnapResult = null;
    let bestDist = SNAP_R;
    for (const b of allBlocks) {
      if (b.id === block.id) continue;
      if (b.block_type === "shape") {
        try { const d = JSON.parse(b.content); if (d.shape === "line" || d.shape === "arrow") continue; } catch {}
      }
      for (const port of PORTS) {
        const [px, py] = getPortCoords(b, port);
        const d = Math.hypot(px - cx, py - cy);
        if (d < bestDist) { bestDist = d; best = { block: b, x: px, y: py, port }; }
      }
    }
    return best;
  }

  // Dash patterns scaled by strokeWidth
  function dashArray(ls: LineStyle | undefined, sw: number): string {
    if (ls === "dashed") return `${sw * 6},${sw * 3}`;
    if (ls === "dotted") return `${sw * 1.5},${sw * 3}`;
    return "none";
  }

  // ── Connector local state ─────────────────────────────
  let ex1 = 0, ey1 = 0, ex2 = 0, ey2 = 0;
  $: if (!endpointDragging && isConnector) {
    // For connected endpoints, read live position from allBlocks so the connector
    // follows instantly when a connected block is dragged (no lag or jump).
    if (data.startId) {
      const sb = allBlocks.find(b => b.id === data.startId);
      [ex1, ey1] = sb
        ? getPortCoords(sb, data.startPort ?? "center")
        : [data.x1 ?? 0, data.y1 ?? 0];
    } else {
      ex1 = data.x1 ?? 0; ey1 = data.y1 ?? 0;
    }
    if (data.endId) {
      const eb = allBlocks.find(b => b.id === data.endId);
      [ex2, ey2] = eb
        ? getPortCoords(eb, data.endPort ?? "center")
        : [data.x2 ?? 0, data.y2 ?? 0];
    } else {
      ex2 = data.x2 ?? 0; ey2 = data.y2 ?? 0;
    }
  }

  const PAD = 28;
  $: svgLeft   = Math.min(ex1, ex2) - PAD;
  $: svgTop    = Math.min(ey1, ey2) - PAD;
  $: svgW      = Math.abs(ex2 - ex1) + PAD * 2;
  $: svgH      = Math.abs(ey2 - ey1) + PAD * 2;
  $: lx1       = ex1 - svgLeft;
  $: ly1       = ey1 - svgTop;
  $: lx2       = ex2 - svgLeft;
  $: ly2       = ey2 - svgTop;
  $: midX      = (ex1 + ex2) / 2;
  $: midY      = (ey1 + ey2) / 2;

  // ── Connector drag ────────────────────────────────────
  let endpointDragging: "start" | "end" | "body" | null = null;
  let cdx0 = 0, cdy0 = 0;
  let cex1_0 = 0, cey1_0 = 0, cex2_0 = 0, cey2_0 = 0;
  let snapResult: SnapResult = null;

  function startConnectorDrag(e: PointerEvent, mode: "start" | "end" | "body") {
    if (drawMode) return;
    e.preventDefault(); e.stopPropagation();
    onSelect(block.id); onBringToFront(block.id);
    endpointDragging = mode;
    cdx0 = e.clientX; cdy0 = e.clientY;
    cex1_0 = ex1; cey1_0 = ey1; cex2_0 = ex2; cey2_0 = ey2;
    window.addEventListener("pointermove", onConnectorMove);
    window.addEventListener("pointerup",   onConnectorUp);
    window.addEventListener("pointercancel", onConnectorUp);
  }

  function onConnectorMove(e: PointerEvent) {
    if (!endpointDragging) return;
    const dx = (e.clientX - cdx0) / zoom;
    const dy = (e.clientY - cdy0) / zoom;
    if (endpointDragging === "body") {
      // Move whole connector; keep snap data as-is (don't clear startId/endId here)
      ex1 = Math.max(0, cex1_0 + dx); ey1 = Math.max(0, cey1_0 + dy);
      ex2 = Math.max(0, cex2_0 + dx); ey2 = Math.max(0, cey2_0 + dy);
      snapResult = null;
    } else {
      const bx = endpointDragging === "start" ? cex1_0 : cex2_0;
      const by = endpointDragging === "start" ? cey1_0 : cey2_0;
      const nx = bx + dx, ny = by + dy;
      snapResult = findNearestSnap(nx, ny);
      const fx = snapResult ? snapResult.x : Math.max(0, nx);
      const fy = snapResult ? snapResult.y : Math.max(0, ny);
      if (endpointDragging === "start") { ex1 = fx; ey1 = fy; }
      else                              { ex2 = fx; ey2 = fy; }
    }
  }

  async function onConnectorUp() {
    if (!endpointDragging) return;
    const mode = endpointDragging;
    endpointDragging = null;
    const snap = snapResult;
    snapResult = null;

    // Preserve existing connections unless this endpoint was explicitly moved
    let newStartId   = data.startId;
    let newEndId     = data.endId;
    let newStartPort = data.startPort ?? "center" as Port;
    let newEndPort   = data.endPort   ?? "center" as Port;

    if (mode === "start") {
      newStartId   = snap?.block.id ?? null;
      newStartPort = snap?.port ?? "center";
    } else if (mode === "end") {
      newEndId   = snap?.block.id ?? null;
      newEndPort = snap?.port ?? "center";
    }
    // body drag: connections (startId/endId) remain unchanged

    const newContent = JSON.stringify({
      ...data, x1: ex1, y1: ey1, x2: ex2, y2: ey2,
      startId: newStartId, endId: newEndId,
      startPort: newStartPort, endPort: newEndPort,
    });
    const nx = Math.min(ex1, ex2), ny = Math.min(ey1, ey2);
    const nw = Math.max(Math.abs(ex2 - ex1), 10), nh = Math.max(Math.abs(ey2 - ey1), 10);
    onUpdate(block.id, { content: newContent, x: nx, y: ny, width: nw, height: nh });

    window.removeEventListener("pointermove", onConnectorMove);
    window.removeEventListener("pointerup",   onConnectorUp);
    window.removeEventListener("pointercancel", onConnectorUp);
  }

  onDestroy(() => {
    window.removeEventListener("pointermove", onDragMove);
    window.removeEventListener("pointerup",   onDragEnd);
    window.removeEventListener("pointercancel", onDragEnd);
    window.removeEventListener("pointermove", onResizeMove);
    window.removeEventListener("pointerup",   onResizeEnd);
    window.removeEventListener("pointercancel", onResizeEnd);
    window.removeEventListener("pointermove", onRotateMove);
    window.removeEventListener("pointerup",   onRotateEnd);
    window.removeEventListener("pointercancel", onRotateEnd);
    window.removeEventListener("pointermove", onConnectorMove);
    window.removeEventListener("pointerup",   onConnectorUp);
    window.removeEventListener("pointercancel", onConnectorUp);
  });
</script>

{#if isConnector}
  <!-- ── SVG connector ── -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <svg
    style="position:absolute; left:{svgLeft}px; top:{svgTop}px; overflow:visible; pointer-events:none; z-index:{block.z_index};"
    width={svgW} height={svgH}
  >
    <defs>
      {#if data.shape === "arrow"}
        <!--
          markerUnits="strokeWidth" → marker dims scale with line thickness.
          refX=6 aligns the polygon tip (vertex at x=6) with the line endpoint.
          The polygon: base at x=0, tip at x=6 — pointing right (auto-oriented).
        -->
        <marker
          id="ah-{block.id}"
          markerWidth="6" markerHeight="4"
          refX="6" refY="2"
          orient="auto"
          markerUnits="strokeWidth"
        >
          <polygon points="0 0, 6 2, 0 4" fill={data.stroke} />
        </marker>
      {/if}
    </defs>

    <!-- Selection glow -->
    {#if selected}
      <line x1={lx1} y1={ly1} x2={lx2} y2={ly2}
            stroke="var(--accent)" stroke-width={data.strokeWidth + 8}
            stroke-linecap="round" opacity="0.18" style="pointer-events:none" />
    {/if}

    <!-- Actual line -->
    <line x1={lx1} y1={ly1} x2={lx2} y2={ly2}
          stroke={data.stroke} stroke-width={data.strokeWidth}
          stroke-linecap="round"
          stroke-dasharray={dashArray(data.lineStyle, data.strokeWidth)}
          marker-end={data.shape === "arrow" ? `url(#ah-${block.id})` : ""}
          style="pointer-events:none" />

    <!-- Wide transparent hit area -->
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <line x1={lx1} y1={ly1} x2={lx2} y2={ly2}
          stroke="transparent" stroke-width="22"
          style="pointer-events:stroke; cursor:move"
          on:pointerdown={e => startConnectorDrag(e, "body")} />

    <!-- Snap highlight while dragging endpoint -->
    {#if snapResult}
      <circle
        cx={snapResult.x - svgLeft} cy={snapResult.y - svgTop}
        r="10" fill="rgba(99,102,241,0.15)" stroke="var(--accent)"
        stroke-width="2" style="pointer-events:none" />
    {/if}

    <!-- Endpoint handles when selected -->
    {#if selected && !drawMode}
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <circle cx={lx1} cy={ly1} r="7"
              fill="white" stroke="var(--accent)" stroke-width="2.5"
              style="pointer-events:all; cursor:move"
              on:pointerdown={e => startConnectorDrag(e, "start")} />
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <circle cx={lx2} cy={ly2} r="7"
              fill="white" stroke="var(--accent)" stroke-width="2.5"
              style="pointer-events:all; cursor:move"
              on:pointerdown={e => startConnectorDrag(e, "end")} />
      <!-- Filled dot when connected to a block -->
      {#if data.startId}
        <circle cx={lx1} cy={ly1} r="4" fill="var(--accent)" style="pointer-events:none" />
      {/if}
      {#if data.endId}
        <circle cx={lx2} cy={ly2} r="4" fill="var(--accent)" style="pointer-events:none" />
      {/if}
    {/if}
  </svg>

  <!-- Delete button (outside SVG, in canvas space) -->
  {#if selected && !drawMode}
    <div class="connector-ctrl" style="left:{midX}px; top:{midY - 28}px;">
      <button class="del-btn" on:click|stopPropagation={() => onDelete(block.id)} title={$t.canvas.delete}>×</button>
    </div>
  {/if}

{:else}
<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  bind:this={shapeEl}
  class="shape-root"
  class:selected
  class:multi-selected={multiSelected}
  class:dragging
  class:rotating
  style="left:{lx}px; top:{ly}px; width:{lw}px; height:{lh}px; z-index:{block.z_index}; transform:rotate({localRotation}deg); transform-origin:center;"
  on:pointerdown={onShapePointerDown}
  on:dblclick={onDblClick}
  on:pointerenter={() => hovered = true}
  on:pointerleave={() => hovered = false}
>
  <svg viewBox="0 0 100 100" preserveAspectRatio="none" class="shape-svg" overflow="visible">
    {#if data.shape === "rect"}
      <rect x="0" y="0" width="100" height="100" rx="6"
        fill={data.fill} stroke={data.stroke} stroke-width={data.strokeWidth} />
    {:else if data.shape === "circle"}
      <ellipse cx="50" cy="50" rx="50" ry="50"
        fill={data.fill} stroke={data.stroke} stroke-width={data.strokeWidth} />
    {:else if data.shape === "triangle"}
      <path d={TRIANGLE}
        fill={data.fill} stroke={data.stroke} stroke-width={data.strokeWidth} stroke-linejoin="round" />
    {:else if data.shape === "diamond"}
      <path d={DIAMOND}
        fill={data.fill} stroke={data.stroke} stroke-width={data.strokeWidth} stroke-linejoin="round" />
    {:else if data.shape === "heart"}
      <path d={HEART}
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
        class:text-editor-dark={isTextOnly}
        on:blur={commitText}
        on:keydown={e => { if (e.key === "Escape") commitText(); }}
        placeholder={$t.canvas.writeHere}
      />
    </div>
  {:else if data.text}
    <div class="text-overlay">
      <span class="text-display" class:text-only={isTextOnly} style="color:{textColor}; text-shadow:{textShadow};">{data.text}</span>
    </div>
  {/if}

  <!-- Selection controls -->
  {#if selected && !drawMode}
    <div class="ctrl-row">
      <button class="del-btn" on:click|stopPropagation={() => onDelete(block.id)} title={$t.canvas.delete}>×</button>
      <div class="rotate-handle" on:pointerdown={onRotateHandleDown} title={$t.canvas.rotate}>↻</div>
    </div>
    <div class="handle nw" on:pointerdown={e => onHandlePointerDown(e, "nw")}></div>
    <div class="handle ne" on:pointerdown={e => onHandlePointerDown(e, "ne")}></div>
    <div class="handle sw" on:pointerdown={e => onHandlePointerDown(e, "sw")}></div>
    <div class="handle se" on:pointerdown={e => onHandlePointerDown(e, "se")}></div>
  {/if}

  <!-- Port dots — visible on hover or select, for starting connectors -->
  {#if (hovered || selected) && !drawMode && !editing}
    {#each PORT_POSITIONS as p}
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <div
        class="port-dot"
        style={p.css}
        on:pointerdown={e => onPortPointerDown(e, p.port)}
      ></div>
    {/each}
  {/if}
</div>
{/if}

<style>
  .shape-root {
    position: absolute;
    cursor: move;
    user-select: none;
    touch-action: none;
    overflow: visible;
    container-type: size;
  }
  .shape-root:active, .shape-root.dragging { cursor: move; }

  .shape-root.selected::before,
  .shape-root.multi-selected::before {
    content: '';
    position: absolute; inset: -4px;
    border: 1.5px solid var(--accent);
    border-radius: 4px;
    pointer-events: none;
    opacity: 0.7;
  }
  .shape-root.multi-selected::before { opacity: 0.45; }

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
  .text-overlay.editing { pointer-events: all; padding: 0; }
  .text-display {
    max-width: 80%; text-align: center; word-break: break-word;
    font-size: clamp(11px, 3cqw, 16px); font-weight: 600;
    color: #fff; text-shadow: 0 1px 4px rgba(0,0,0,0.65);
    pointer-events: none; line-height: 1.3;
  }
  /* Text-only blocks: font scales with block width */
  .text-display.text-only {
    max-width: 100%; font-size: clamp(11px, 8cqw, 120px);
    font-weight: 400; white-space: pre-wrap;
    padding: 4px 8px; line-height: 1.35;
  }
  .text-editor {
    width: 70%; min-height: 40px; resize: none;
    background: rgba(0,0,0,0.45); color: #fff;
    border: 1px solid rgba(255,255,255,0.4); border-radius: 4px;
    padding: 4px 8px; font-size: 13px; font-weight: 600;
    text-align: center; outline: none; backdrop-filter: blur(4px);
  }
  /* Text-only mode: editor scales with block, light background */
  .text-editor.text-editor-dark {
    width: 100%; height: 100%; box-sizing: border-box;
    background: rgba(255,255,255,0.92); color: var(--text-primary);
    border: 1px solid var(--accent); border-radius: 4px;
    font-size: clamp(11px, 8cqw, 120px); font-weight: 400;
    text-align: left; padding: 6px 8px; resize: none;
    backdrop-filter: none;
  }

  /* Controls pill */
  .ctrl-row {
    position: absolute;
    bottom: calc(100% + 14px); left: 50%;
    transform: translateX(-50%);
    display: flex; align-items: center; gap: 6px;
    background: rgba(255,255,255,0.92);
    border: 1px solid rgba(0,0,0,0.1);
    border-radius: 20px; padding: 4px 8px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.15); z-index: 12;
  }
  .ctrl-row::after {
    content: '';
    position: absolute; top: 100%; left: 50%;
    transform: translateX(-50%);
    width: 1px; height: 14px;
    background: var(--accent); opacity: 0.5;
  }

  .del-btn {
    width: 22px; height: 22px; border-radius: 50%;
    background: transparent;
    border: 1.5px solid rgba(150,150,165,0.5);
    color: rgba(120,120,140,0.85);
    font-size: 14px; font-weight: 500; line-height: 1;
    display: flex; align-items: center; justify-content: center;
    transition: all 0.15s; cursor: pointer; flex-shrink: 0; padding: 0;
  }
  .del-btn:hover { border-color: var(--red); color: var(--red); background: rgba(239,68,68,0.08); }

  .rotate-handle {
    width: 22px; height: 22px; border-radius: 50%;
    background: transparent; border: 1.5px solid var(--accent);
    color: var(--accent); font-size: 14px; line-height: 1;
    display: flex; align-items: center; justify-content: center;
    cursor: move; flex-shrink: 0;
    transition: background 0.15s, color 0.15s; user-select: none;
  }
  .rotate-handle:hover { background: rgba(99,102,241,0.1); }
  .shape-root.rotating { cursor: move; }
  .shape-root.rotating .rotate-handle { cursor: move; }

  /* Resize handles */
  .handle {
    position: absolute;
    width: 10px; height: 10px; border-radius: 2px;
    background: #fff; border: 2px solid var(--accent);
    box-shadow: 0 1px 4px rgba(0,0,0,0.35); z-index: 10;
  }
  .handle.nw { top: -5px;    left: -5px;   cursor: nw-resize; }
  .handle.ne { top: -5px;    right: -5px;  cursor: ne-resize; }
  .handle.sw { bottom: -5px; left: -5px;   cursor: sw-resize; }
  .handle.se { bottom: -5px; right: -5px;  cursor: se-resize; }
  .handle:hover { background: var(--accent); }

  /* Port dots */
  .port-dot {
    position: absolute;
    width: 12px; height: 12px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid #fff;
    box-shadow: 0 1px 4px rgba(0,0,0,0.3);
    cursor: crosshair;
    z-index: 15;
    transition: transform 0.1s, background 0.1s;
    pointer-events: all;
  }
  .port-dot:hover {
    transform: translate(-50%, -50%) scale(1.35) !important;
    background: #4f46e5;
  }

  /* Connector delete pill */
  .connector-ctrl {
    position: absolute; pointer-events: all;
    transform: translate(-50%, 0);
    display: flex; align-items: center; gap: 4px;
    background: rgba(255,255,255,0.92); border: 1px solid rgba(0,0,0,0.1);
    border-radius: 20px; padding: 3px 6px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.15); z-index: 999;
  }
</style>
