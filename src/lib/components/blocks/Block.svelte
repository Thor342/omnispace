<script lang="ts">
  import type { Block } from "../../types";
  import { t } from "../../stores/language";
  import NoteBlock     from "./NoteBlock.svelte";
  import LinkBlock     from "./LinkBlock.svelte";
  import FileBlock     from "./FileBlock.svelte";
  import TaskBlock     from "./TaskBlock.svelte";
  import CalendarBlock from "./CalendarBlock.svelte";
  import ClockBlock    from "./ClockBlock.svelte";
  import ShapeBlock    from "./ShapeBlock.svelte";

  export let block: Block;
  export let drawMode: boolean;
  export let zoom: number = 1;
  export let selected: boolean = false;
  export let onUpdate: (id: string, changes: Partial<Block>) => void;
  export let onMove: ((id: string, x: number, y: number) => void) | undefined = undefined;
  export let onDelete: (id: string) => void;
  export let onBringToFront: (id: string) => void;
  export let onSelect: (id: string) => void = () => {};
  export let multiSelected: boolean = false;
  export let connectorMode: boolean = false;

  // ── File subtype (for audio/etc transparent styling) ──
  $: fileSubtype = block.block_type === "file"
    ? (() => { try { return JSON.parse(block.content || "{}").file_type ?? ""; } catch { return ""; } })()
    : "";

  // ── Local geometry ───────────────────────────────────
  let lx = block.x, ly = block.y, lw = block.width, lh = block.height;

  // Sync from parent when not dragging/resizing locally (e.g. multi-drag from PageCanvas)
  $: if (!dragging && !resizing) { lx = block.x; ly = block.y; lw = block.width; lh = block.height; }

  // ── Minimize ─────────────────────────────────────────
  const MINI_KEY = `block_mini_${block.id}`;
  let minimized: boolean = localStorage.getItem(MINI_KEY) === "true";

  function toggleMinimize() {
    minimized = !minimized;
    localStorage.setItem(MINI_KEY, String(minimized));
  }

  // ── Custom label ──────────────────────────────────────
  const LABEL_KEY = `block_label_${block.id}`;
  let customLabel: string = localStorage.getItem(LABEL_KEY) ?? "";
  let editingLabel = false;
  let labelInput = "";

  function startEditLabel(e: MouseEvent | KeyboardEvent) {
    e.stopPropagation();
    labelInput = customLabel || blockLabel;
    editingLabel = true;
  }

  function commitLabel() {
    customLabel = labelInput.trim();
    localStorage.setItem(LABEL_KEY, customLabel);
    editingLabel = false;
  }

  $: displayLabel = customLabel || blockLabel;

  function requestDelete() {
    onDelete(block.id);
  }

  // ── Expand (fullscreen) ───────────────────────────────
  // Portal action: teleports the modal to document.body so it escapes
  // the canvas-content CSS transform (which breaks position:fixed)
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy() { node.remove(); } };
  }

  let expanded = false;

  // ── Drag ─────────────────────────────────────────────
  let dragging = false;
  let dragSX = 0, dragSY = 0, dragBX = 0, dragBY = 0;

  function onHeaderPointerDown(e: PointerEvent) {
    if (drawMode || editingLabel || connectorMode) return;
    e.preventDefault();
    dragging = true;
    dragSX = e.clientX; dragSY = e.clientY;
    dragBX = lx; dragBY = ly;
    window.addEventListener("pointermove", onDragMove);
    window.addEventListener("pointerup",   onDragEnd);
    window.addEventListener("pointercancel", onDragEnd);
  }

  function onDragMove(e: PointerEvent) {
    if (!dragging) return;
    lx = Math.max(0, dragBX + (e.clientX - dragSX) / zoom);
    ly = Math.max(0, dragBY + (e.clientY - dragSY) / zoom);
    onMove?.(block.id, lx, ly);
  }

  function onDragEnd() {
    if (!dragging) return;
    dragging = false;
    window.removeEventListener("pointermove", onDragMove);
    window.removeEventListener("pointerup",   onDragEnd);
    window.removeEventListener("pointercancel", onDragEnd);
    onUpdate(block.id, { x: lx, y: ly, width: lw, height: lh });
  }

  // ── Resize ───────────────────────────────────────────
  let resizing = false;
  let resSX = 0, resSY = 0, resSW = 0, resSH = 0;

  function onResizePointerDown(e: PointerEvent) {
    if (minimized) return;
    e.preventDefault(); e.stopPropagation();
    resizing = true;
    resSX = e.clientX; resSY = e.clientY;
    resSW = lw; resSH = lh;
    onBringToFront(block.id);
    window.addEventListener("pointermove", onResizeMove);
    window.addEventListener("pointerup",   onResizeEnd);
    window.addEventListener("pointercancel", onResizeEnd);
  }

  function onResizeMove(e: PointerEvent) {
    if (!resizing) return;
    lw = Math.max(220, resSW + (e.clientX - resSX) / zoom);
    lh = Math.max(140, resSH + (e.clientY - resSY) / zoom);
  }

  function onResizeEnd() {
    if (!resizing) return;
    resizing = false;
    window.removeEventListener("pointermove", onResizeMove);
    window.removeEventListener("pointerup",   onResizeEnd);
    window.removeEventListener("pointercancel", onResizeEnd);
    onUpdate(block.id, { x: lx, y: ly, width: lw, height: lh });
  }

  // localContent: siempre up-to-date aunque el padre no haya propagado aún
  let localContent = block.content;

  function handleContentChange(newContent: string) {
    localContent = newContent;          // actualiza inmediato → expand ve lo mismo
    onUpdate(block.id, { content: newContent });
  }

  const ICONS: Record<string, string> = { note:"📝", link:"🔗", file:"📎", task:"✅", calendar:"📅", clock:"🕐", shape:"◻" };
  $: LABELS = { note: $t.blockHeader.note, link: $t.blockHeader.link, file: $t.blockHeader.file, task: $t.blockHeader.task, calendar: $t.blockHeader.calendar, clock: $t.blockHeader.clock, shape: $t.blockHeader.shape };
  $: blockIcon  = ICONS[block.block_type]  ?? "📦";
  $: blockLabel = block.block_type === "file"
    ? (() => { try { return JSON.parse(localContent || "{}").name || $t.blockHeader.file; } catch { return $t.blockHeader.file; } })()
    : (LABELS[block.block_type] ?? block.block_type);

  $: displayH = minimized ? 36 : (lh < 36 ? 36 : lh);

  function focusEl(node: HTMLElement) { node.focus(); return {}; }
</script>

<!-- ── Fullscreen expand modal ─────────────────────────── -->
{#if expanded}
  <div use:portal class="expand-backdrop"
    role="button" tabindex="0"
    aria-label={$t.blockHeader.close}
    on:click={e => { if (e.target === e.currentTarget) expanded = false; }}
    on:keydown={e => e.key === 'Escape' && (expanded = false)}
  >
    <div class="expand-modal" role="dialog" aria-modal="true" aria-label={displayLabel}>
      <div class="expand-header">
        <span class="block-icon">{blockIcon}</span>
        <span class="expand-title">{displayLabel}</span>
        <button class="expand-close" on:click={() => expanded = false} title={$t.blockHeader.close}>×</button>
      </div>
      <div class="expand-body">
        {#if block.block_type === "file"}
          <FileBlock content={localContent} onUpdate={handleContentChange} />
        {:else if block.block_type === "note"}
          <NoteBlock content={localContent} onContentChange={handleContentChange} />
        {:else if block.block_type === "link"}
          <LinkBlock content={localContent} onContentChange={handleContentChange} />
        {:else if block.block_type === "task"}
          <TaskBlock content={localContent} onContentChange={handleContentChange} />
        {:else if block.block_type === "calendar"}
          <CalendarBlock content={localContent} onContentChange={handleContentChange} />
        {:else if block.block_type === "clock"}
          <ClockBlock content={localContent} onContentChange={handleContentChange} />
        {:else if block.block_type === "shape"}
          <ShapeBlock content={localContent} onContentChange={handleContentChange} />
        {/if}
      </div>
    </div>
  </div>
{/if}

<!-- ── Block card ────────────────────────────────────── -->
<div
  class="block"
  class:dragging
  class:draw-blocked={drawMode}
  class:minimized
  class:selected
  class:multi-selected={multiSelected}
  data-type={block.block_type}
  data-subtype={fileSubtype}
  style="transform:translate({lx}px,{ly}px); width:{lw}px; height:{displayH}px; z-index:{block.z_index}; opacity:{minimized ? 0.65 : 1}"
  on:pointerdown={() => { if (!drawMode && !connectorMode) { onSelect(block.id); onBringToFront(block.id); } }}
>
  <!-- Header -->
  <div class="block-header" on:pointerdown={onHeaderPointerDown} role="toolbar" aria-label="mover bloque">
    <span class="block-icon">{blockIcon}</span>

    <!-- Editable label -->
    {#if editingLabel}
      <input
        class="label-input"
        bind:value={labelInput}
        on:blur={commitLabel}
        on:keydown={e => { if (e.key === "Enter") commitLabel(); if (e.key === "Escape") editingLabel = false; }}
        on:click|stopPropagation
        on:pointerdown|stopPropagation
        use:focusEl
      />
    {:else}
      <span
        class="block-label"
        title={$t.blockHeader.rename}
        role="button"
        tabindex="0"
        on:dblclick={startEditLabel}
        on:keydown={e => e.key === 'Enter' && startEditLabel(e)}
      >{displayLabel}</span>
    {/if}

    <!-- Expand button (always visible, all block types) -->
    <button class="hdr-btn expand-btn" on:pointerdown|stopPropagation on:click|stopPropagation={() => expanded = true} title={$t.blockHeader.expand}>⛶</button>

    <button class="hdr-btn" on:pointerdown|stopPropagation on:click|stopPropagation={toggleMinimize} title={minimized ? $t.blockHeader.restore : $t.blockHeader.minimize}>
      {minimized ? "□" : "—"}
    </button>

    <button class="hdr-btn close" on:pointerdown|stopPropagation on:click|stopPropagation={requestDelete} title={$t.blockHeader.delete}>×</button>
  </div>

  <!-- Content -->
  {#if !minimized}
    <div class="block-body">
      {#if resizing}
        <div class="resize-shield" />
      {/if}

      {#if block.block_type === "note"}
        <NoteBlock content={localContent} onContentChange={handleContentChange} />
      {:else if block.block_type === "link"}
        <LinkBlock content={localContent} onContentChange={handleContentChange} />
      {:else if block.block_type === "file"}
        <FileBlock content={localContent} onUpdate={handleContentChange} />
      {:else if block.block_type === "task"}
        <TaskBlock content={localContent} onContentChange={handleContentChange} />
      {:else if block.block_type === "calendar"}
        <CalendarBlock content={localContent} onContentChange={handleContentChange} />
      {:else if block.block_type === "clock"}
        <ClockBlock content={localContent} onContentChange={handleContentChange} />
      {:else if block.block_type === "shape"}
        <ShapeBlock content={localContent} onContentChange={handleContentChange} />
      {/if}
    </div>
  {/if}

  {#if !minimized}
    {@const hs = Math.min(48, Math.max(20, Math.round(24 / zoom)))}
    <div class="resize-handle" style="width:{hs}px;height:{hs}px" on:pointerdown={onResizePointerDown} />
  {/if}
</div>

<style>
  .block {
    position: absolute;
    left: 0; top: 0;
    display: flex; flex-direction: column;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.18), 0 1px 4px rgba(0,0,0,0.1);
    overflow: hidden;
    transition: border-color 0.15s;
    touch-action: none;
    cursor: default;
  }
  .block:hover {
    border-color: var(--border-focus);
    box-shadow: 0 8px 28px rgba(0,0,0,0.22), 0 2px 6px rgba(0,0,0,0.1);
  }
  .block.dragging {
    box-shadow: 0 16px 48px rgba(0,0,0,0.40);
    will-change: transform;
  }
  .block.draw-blocked { pointer-events: none; }
  .block.minimized {
    box-shadow: 0 2px 8px rgba(0,0,0,0.15);
    border-radius: 10px;
  }
  .block.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-dim), 0 8px 28px rgba(0,0,0,0.28);
  }
  .block.multi-selected {
    outline: 2px dashed var(--accent);
    outline-offset: 2px;
  }
  .block.selected.dragging {
    box-shadow: 0 0 0 2px var(--accent-dim), 0 20px 56px rgba(0,0,0,0.45);
  }

  /* ── Header ── */
  .block-header {
    display: flex; align-items: center; gap: 6px;
    padding: 0 10px; height: 34px;
    background: var(--bg-overlay);
    border-bottom: 1px solid var(--border);
    cursor: grab; flex-shrink: 0; user-select: none;
    transition: background 0.15s;
  }
  .block-header:active { cursor: grabbing; }

  .block-icon { font-size: 12px; flex-shrink: 0; }

  .block-label {
    flex: 1; font-size: 10px; font-weight: 700;
    letter-spacing: 0.7px; color: var(--text-muted);
    text-transform: uppercase; cursor: text;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    transition: color 0.15s;
  }
  .block-label:hover { color: var(--text-secondary); }

  .label-input {
    flex: 1; font-size: 10px; font-weight: 700; letter-spacing: 0.7px;
    text-transform: uppercase; color: var(--text-primary);
    background: var(--bg-base); border: 1px solid var(--accent);
    border-radius: 4px; padding: 1px 6px; min-width: 0;
    cursor: text;
  }

  .hdr-btn {
    font-size: 14px; color: var(--text-muted); padding: 2px 5px;
    border-radius: 4px; line-height: 1; opacity: 0;
    transition: opacity 0.15s, color 0.15s, background 0.15s;
  }
  .block:hover .hdr-btn { opacity: 1; }
  .hdr-btn:hover { color: var(--text-primary); background: var(--bg-hover); }
  .hdr-btn.close:hover { color: #ef4444; background: rgba(239,68,68,0.12); }
  .hdr-btn.expand-btn { font-size: 12px; }


  /* ── Body ── */
  .block-body {
    flex: 1; overflow: hidden;
    display: flex; flex-direction: column;
    position: relative; cursor: default;
  }

  .resize-shield { position: absolute; inset: 0; z-index: 99; cursor: se-resize; }

  .resize-handle {
    position: absolute; bottom: 0; right: 0;
    /* size set via inline style (scales with 1/zoom for consistent screen size) */
    cursor: se-resize;
    background: linear-gradient(135deg, transparent 55%, rgba(99,102,241,0.3) 55%);
    border-radius: 0 0 14px 0;
    touch-action: none;
    transition: background 0.15s;
  }
  .resize-handle:hover {
    background: linear-gradient(135deg, transparent 55%, var(--accent) 55%);
  }

  /* ── Per-type header theming ── */
  [data-type="note"] .block-header {
    background: #FFFDE7;
    border-bottom-color: rgba(180,140,0,0.18);
    height: 26px;
  }
  [data-type="note"] .block-icon { opacity: 0; transition: opacity 0.2s; }
  [data-type="note"] .block-label { opacity: 0; transition: opacity 0.2s; }
  [data-type="note"]:hover .block-icon,
  [data-type="note"]:hover .block-label { opacity: 0.35; }
  [data-type="note"].selected .block-header { background: #FFF9C4; }

  [data-type="clock"] {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }
  [data-type="clock"]:hover {
    border-color: transparent;
    box-shadow: none;
  }
  [data-type="clock"].dragging { box-shadow: none; }
  [data-type="clock"].selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-dim);
  }
  [data-type="clock"] .block-header {
    position: absolute; top: 0; left: 0; right: 0; z-index: 10;
    background: transparent; border: none; height: 32px;
    opacity: 0; transition: opacity 0.2s;
  }
  [data-type="clock"]:hover .block-header {
    opacity: 1;
    background: linear-gradient(180deg, rgba(0,0,0,0.12) 0%, transparent 100%);
  }
  [data-type="clock"] .resize-handle { opacity: 0; transition: opacity 0.2s; }
  [data-type="clock"]:hover .resize-handle { opacity: 1; }

  [data-type="calendar"] {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }
  [data-type="calendar"]:hover {
    border-color: transparent;
    box-shadow: none;
  }
  [data-type="calendar"].dragging { box-shadow: none; }
  [data-type="calendar"].selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-dim);
  }
  [data-type="calendar"] .block-header {
    position: absolute; top: 0; left: 0; right: 0; z-index: 10;
    background: transparent; border: none; height: 32px;
    opacity: 0; transition: opacity 0.2s;
  }
  [data-type="calendar"]:hover .block-header {
    opacity: 1;
    background: linear-gradient(180deg, rgba(0,0,0,0.10) 0%, transparent 100%);
  }
  [data-type="calendar"] .resize-handle { opacity: 0; transition: opacity 0.2s; }
  [data-type="calendar"]:hover .resize-handle { opacity: 1; }

  [data-type="link"] .block-header {
    background: linear-gradient(90deg, rgba(20,184,166,0.08), var(--bg-overlay));
  }

  [data-type="file"] .block-header {
    background: var(--bg-overlay);
    border-bottom-color: rgba(0,0,0,0.08);
  }

  /* ── Audio file: transparent block, floating header ── */
  [data-type="file"][data-subtype="audio"] {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }
  [data-type="file"][data-subtype="audio"]:hover {
    border-color: transparent;
    box-shadow: none;
  }
  [data-type="file"][data-subtype="audio"].dragging { box-shadow: none; }
  [data-type="file"][data-subtype="audio"].selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-dim);
  }
  [data-type="file"][data-subtype="audio"] .block-header {
    position: absolute; top: 0; left: 0; right: 0; z-index: 10;
    background: transparent; border: none; height: 32px;
    opacity: 0; transition: opacity 0.2s;
  }
  [data-type="file"][data-subtype="audio"]:hover .block-header {
    opacity: 1;
    background: linear-gradient(180deg, rgba(0,0,0,0.35) 0%, transparent 100%);
  }
  [data-type="file"][data-subtype="audio"] .block-header .block-label { color: rgba(255,255,255,0.7); }
  [data-type="file"][data-subtype="audio"] .block-header .hdr-btn { color: rgba(255,255,255,0.6); }
  [data-type="file"][data-subtype="audio"] .block-header .hdr-btn:hover { color: #fff; background: rgba(255,255,255,0.12); }
  [data-type="file"][data-subtype="audio"] .resize-handle { opacity: 0; transition: opacity 0.2s; }
  [data-type="file"][data-subtype="audio"]:hover .resize-handle { opacity: 1; }

  [data-type="task"] .block-header {
    background: linear-gradient(90deg, rgba(34,197,94,0.08), var(--bg-overlay));
  }

  /* ── Expand modal ── */
  .expand-backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.72);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000; backdrop-filter: blur(6px);
  }
  .expand-modal {
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: 18px;
    width: min(92vw, 1100px); height: min(88vh, 800px);
    display: flex; flex-direction: column;
    box-shadow: 0 32px 80px rgba(0,0,0,0.5);
    overflow: hidden;
  }
  .expand-header {
    display: flex; align-items: center; gap: 8px;
    padding: 0 18px; height: 46px;
    background: var(--bg-overlay); border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .expand-title {
    flex: 1; font-size: 13px; font-weight: 600;
    color: var(--text-primary); text-transform: uppercase; letter-spacing: 0.6px;
  }
  .expand-close {
    font-size: 20px; color: var(--text-muted); padding: 3px 9px;
    border-radius: 6px; line-height: 1;
    transition: color 0.15s, background 0.15s;
  }
  .expand-close:hover { color: #ef4444; background: rgba(239,68,68,0.12); }
  .expand-body {
    flex: 1; overflow: hidden;
    display: flex; flex-direction: column;
  }
</style>
