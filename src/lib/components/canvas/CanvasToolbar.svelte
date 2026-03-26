<script lang="ts">
  import type { BlockType } from "../../types";
  import { t } from "../../stores/language";

  export let drawMode = false;
  export let eraseMode = false;
  export let shapeMode = false;
  export let shapeType = "rect";
  export let shapeFill = "#6366f1";
  export let shapeStroke = "#1e1e2e";
  export let shapeStrokeWidth = 2;
  export let penColor = "#6366f1";
  export let penWidth = 3;
  export let penOpacity = 100;
  export let eraserSize = 24;
  export let zoom = 1.0;
  export let canUndo = false;
  export let selectMode = false;
  export let onAddBlock: (type: BlockType, hint?: string) => void;
  export let onClearStrokes: () => void;
  export let onUndo: () => void;
  export let editingShape: boolean = false;
  export let editingConnector: boolean = false;
  export let lineStyle: "solid" | "dashed" | "dotted" = "solid";

  let showAddMenu = false;
  let addBtnEl: HTMLElement;
  let menuBottom = 60;
  let menuLeft = 200;

  function toggleAddMenu() {
    if (!showAddMenu && addBtnEl) {
      const r = addBtnEl.getBoundingClientRect();
      menuBottom = window.innerHeight - r.top + 8;
      menuLeft = r.left;
    }
    showAddMenu = !showAddMenu;
  }

  const PEN_COLORS    = ["#e8e8f0","#6366f1","#ec4899","#f59e0b","#22c55e","#ef4444","#14b8a6","#f97316","#000000"];
  const FILL_COLORS  = ["#ffffff","#f3f4f6","#fef9c3","#dbeafe","#dcfce7","#fce7f3","#6366f1","#ec4899","#1e1e2e","transparent"];
  const STROKE_COLORS= ["#1e1e2e","#6366f1","#ec4899","#f59e0b","#22c55e","#ef4444","#14b8a6","#9ca3af","#ffffff"];
  const ERASER_SIZES = [
    { label: "S", size: 12 },
    { label: "M", size: 24 },
    { label: "L", size: 50 },
    { label: "XL", size: 100 },
  ];
  const SHAPE_TYPES = [
    { type: "rect",     icon: "◻" },
    { type: "circle",   icon: "◯" },
    { type: "triangle", icon: "△" },
    { type: "diamond",  icon: "◇" },
    { type: "heart",    icon: "♥" },
    { type: "line",     icon: "╱" },
    { type: "arrow",    icon: "→" },
  ];

  $: BLOCKS = [
    { type: "note"     as BlockType, icon: "📝", label: $t.toolbar.note },
    { type: "link"     as BlockType, icon: "🔗", label: $t.toolbar.link },
    { type: "file"     as BlockType, hint: "image",    icon: "🖼️", label: $t.toolbar.image },
    { type: "file"     as BlockType, hint: "video",    icon: "🎬", label: $t.toolbar.video },
    { type: "file"     as BlockType, hint: "audio",    icon: "🎵", label: $t.toolbar.audio },
    { type: "file"     as BlockType, hint: "document", icon: "📄", label: $t.toolbar.document },
    { type: "task"     as BlockType, icon: "✅", label: $t.toolbar.tasks },
    { type: "calendar" as BlockType, icon: "📅", label: $t.toolbar.calendar },
    { type: "clock"    as BlockType, icon: "🕐", label: $t.toolbar.clock },
    { type: "file"     as BlockType, hint: "record-audio", icon: "🎙", label: $t.toolbar.recordAudio, sep: true },
    { type: "link"     as BlockType, hint: "youtube", icon: `<svg viewBox="0 0 24 24" width="18" height="18" xmlns="http://www.w3.org/2000/svg"><rect width="24" height="24" rx="5" fill="#FF0000"/><polygon points="9.5,7 9.5,17 17.5,12" fill="#fff"/></svg>`, label: "YouTube", sep: true },
    { type: "link"     as BlockType, hint: "canva",   icon: `<svg viewBox="0 0 24 24" width="18" height="18" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="12" fill="#7D2AE8"/><path d="M15.5 8.5c-.6-.6-1.5-1-2.5-1-2.2 0-4 1.8-4 4s1.8 4 4 4c1 0 1.9-.4 2.5-1" stroke="#fff" stroke-width="2" stroke-linecap="round" fill="none"/></svg>`, label: "Canva" },
  ];

  function setZoom(z: number) { zoom = Math.round(Math.min(3, Math.max(0.2, z)) * 100) / 100; }
  function resetZoom() { zoom = 1.0; }

  $: zoomPct = Math.round(zoom * 100);
</script>

{#if showAddMenu}
  <div class="overlay" on:click={() => showAddMenu = false} role="presentation" />
  <div class="add-menu" style="bottom:{menuBottom}px; left:{menuLeft}px;">
    {#each BLOCKS as b}
      {#if b.sep}<div class="menu-sep" />{/if}
      <button class="menu-item" class:menu-item-canva={b.hint === 'canva'} class:menu-item-youtube={b.hint === 'youtube'} on:click={() => { onAddBlock(b.type, b.hint); showAddMenu = false; }}>
        <span class="menu-icon">{@html b.icon}</span><span>{b.label}</span>
      </button>
    {/each}
  </div>
{/if}

<div class="toolbar">
  <div class="sep" />
  {#if drawMode}
    <!-- ── Draw mode ── -->
    <button class="tool-btn" on:click={() => { drawMode = false; eraseMode = false; }}>✖</button>
    <div class="sep" />
    <button class="tool-btn" disabled={!canUndo} on:click={onUndo} title={$t.toolbar.undo}>↩</button>
    <div class="sep" />

    <button class="tool-btn" class:active={!eraseMode} on:click={() => eraseMode = false}
      title={$t.toolbar.pencil}>✏️ {$t.toolbar.pencil}</button>
    <button class="tool-btn" class:active={eraseMode}  on:click={() => eraseMode = true}
      title={$t.toolbar.eraser}>◻ {$t.toolbar.eraser}</button>

    {#if eraseMode}
      <div class="sep" />
      <span class="lbl">{$t.toolbar.size}</span>
      {#each ERASER_SIZES as es}
        <button class="size-btn" class:active={eraserSize === es.size}
          on:click={() => eraserSize = es.size} title="{$t.toolbar.eraser} {es.label}">{es.label}</button>
      {/each}
    {:else}
      <div class="sep" />
      <div class="color-row">
        {#each PEN_COLORS as c}
          <button class="color-sw" class:sel={penColor === c} style="background:{c}" on:click={() => penColor = c} />
        {/each}
      </div>
      <div class="sep" />
      <div class="width-row">
        <span class="lbl">{$t.toolbar.thickness}</span>
        <input type="range" min="1" max="20" bind:value={penWidth} />
        <span class="lbl">{penWidth}px</span>
      </div>
      <div class="sep" />
      <div class="width-row">
        <span class="lbl">{$t.toolbar.blur}</span>
        <input type="range" min="10" max="100" bind:value={penOpacity} />
        <span class="lbl">{penOpacity}%</span>
      </div>
    {/if}

    <div class="sep" />
    <button class="tool-btn danger" on:click={onClearStrokes}>🗑</button>

  {:else if shapeMode}
    <!-- ── Shape mode ── -->
    <button class="tool-btn" on:click={() => { shapeMode = false; shapeType = ""; }}>✖</button>
    <div class="sep" />
    <span class="lbl">{$t.toolbar.shape}</span>
    {#each SHAPE_TYPES as s}
      <button
        class="tool-btn shape-type-btn"
        class:active={shapeType === s.type}
        on:click={() => shapeType = s.type}
        title={s.type}
      >{s.icon}</button>
    {/each}
    <div class="sep" />

    {#if shapeType !== "line" && shapeType !== "arrow"}
    <span class="lbl">{$t.toolbar.fill}</span>
    <div class="color-row">
      {#each FILL_COLORS as c}
        <button
          class="color-sw fill-sw"
          class:sel={shapeFill === c}
          style={c === "transparent" ? "background:none;border:1px dashed #9ca3af;font-size:10px;color:#9ca3af" : `background:${c};border:1px solid rgba(0,0,0,0.15)`}
          on:click={() => shapeFill = c}
          title={c === "transparent" ? $t.toolbar.noFill : c}
        >{c === "transparent" ? "∅" : ""}</button>
      {/each}
      <input type="color" class="color-pick-input" bind:value={shapeFill} title={$t.toolbar.custom} />
    </div>
    <div class="sep" />
    {/if}

    <span class="lbl">{$t.toolbar.stroke}</span>
    <div class="color-row">
      {#each STROKE_COLORS as c}
        <button class="color-sw" class:sel={shapeStroke === c} style="background:{c}" on:click={() => shapeStroke = c} />
      {/each}
      <input type="color" class="color-pick-input" bind:value={shapeStroke} title={$t.toolbar.custom} />
    </div>
    <div class="sep" />

    <span class="lbl">{$t.toolbar.strokeWidth}</span>
    <input type="range" min="1" max="12" bind:value={shapeStrokeWidth} class="stroke-range" />
    <span class="lbl">{shapeStrokeWidth}px</span>

    <!-- Line style: shown for connectors and when creating line/arrow -->
    {#if shapeType === "line" || shapeType === "arrow" || editingConnector}
      <div class="sep" />
      <span class="lbl">{$t.toolbar.line}</span>
      <button class="style-btn" class:active={lineStyle === "solid"}  on:click={() => lineStyle = "solid"}  title={$t.toolbar.solid}>—</button>
      <button class="style-btn" class:active={lineStyle === "dashed"} on:click={() => lineStyle = "dashed"} title={$t.toolbar.dashed}>╌</button>
      <button class="style-btn" class:active={lineStyle === "dotted"} on:click={() => lineStyle = "dotted"} title={$t.toolbar.dotted}>···</button>
    {/if}

  {:else}
    <!-- ── Normal mode ── -->
    <button class="tool-btn" class:active={!selectMode} on:click={() => selectMode = false} title={$t.toolbar.hand}>🖐</button>
    <button class="tool-btn" class:active={selectMode} on:click={() => selectMode = true} title={$t.toolbar.select}>↖</button>
    <div class="sep" />
    <button class="tool-btn" disabled={!canUndo} on:click={onUndo} title={$t.toolbar.undo}>
      ↩ {$t.toolbar.undo.split(":")[0].split(" ")[0]}
    </button>
    <div class="sep" />

    <button class="tool-btn" on:click={() => { drawMode = true; }} title={$t.toolbar.drawMode}>
      ✏️ {$t.toolbar.pencil}
    </button>
    <button class="tool-btn" class:active={shapeMode} on:click={() => { shapeMode = true; shapeType = ""; }} title={$t.toolbar.shapeMode}>
      ◻ {$t.toolbar.shape.replace(":", "")}
    </button>
    <button bind:this={addBtnEl} class="tool-btn accent" on:click={toggleAddMenu}>+ {$t.toolbar.add} ▾</button>

  {/if}

  <!-- Zoom controls — always visible -->
  <div class="zoom-group">
    <button class="zoom-btn" on:click={() => setZoom(zoom - 0.1)} title={$t.toolbar.zoomOut}>−</button>
    <button class="zoom-pct" on:click={resetZoom} title={$t.toolbar.resetZoom}>{zoomPct}%</button>
    <button class="zoom-btn" on:click={() => setZoom(zoom + 0.1)} title={$t.toolbar.zoomIn}>+</button>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 49; }

  .toolbar {
    display: flex; align-items: center; gap: 6px;
    padding: 7px 14px 7px var(--sidebar-w-current, 52px);
    background: var(--bg-surface);
    border-top: 1px solid var(--border); flex-shrink: 0;
    overflow-x: auto; overflow-y: hidden;
    scrollbar-width: none;
    transition: padding-left 0.2s ease;
  }
  .toolbar::-webkit-scrollbar { display: none; }

  .tool-btn {
    display: flex; align-items: center; gap: 5px;
    padding: 5px 11px; border-radius: var(--radius-sm);
    font-size: 12px; color: var(--text-secondary);
    background: var(--bg-overlay); border: 1px solid var(--border);
    transition: all var(--transition); white-space: nowrap;
  }
  .tool-btn:hover:not(:disabled) { color: var(--text-primary); background: var(--bg-hover); }
  .tool-btn:disabled { opacity: 0.35; cursor: not-allowed; }
  .tool-btn.active { background: var(--accent-dim); color: var(--accent); border-color: var(--accent); }
  .tool-btn.accent { background: var(--accent); color: #fff; border-color: var(--accent); }
  .tool-btn.accent:hover { opacity: 0.9; }
  .tool-btn.danger { color: var(--red); border-color: var(--red); }
  .tool-btn.danger:hover { background: rgba(239,68,68,0.15); }

  .sep { width: 1px; height: 22px; background: var(--border); flex-shrink: 0; }
  .lbl { font-size: 11px; color: var(--text-muted); white-space: nowrap; }
  .editing-lbl { color: var(--accent); font-weight: 600; }
  .shape-type-btn { font-size: 16px; padding: 3px 9px; }

  /* Shape fill/stroke colors */
  .fill-sw { display: flex; align-items: center; justify-content: center; }
  .stroke-range { width: 64px; accent-color: var(--accent); }

  /* Shared color input */
  .color-pick-input {
    width: 20px; height: 20px; border-radius: 50%; border: 1px solid var(--border);
    padding: 0; cursor: pointer; background: none; overflow: hidden; flex-shrink: 0;
  }
  .color-pick-input::-webkit-color-swatch-wrapper { padding: 0; }
  .color-pick-input::-webkit-color-swatch { border: none; border-radius: 50%; }

  /* Line style buttons */
  .style-btn {
    padding: 3px 10px; border-radius: var(--radius-sm);
    font-size: 13px; font-weight: 600; letter-spacing: 1px;
    color: var(--text-muted); background: var(--bg-overlay); border: 1px solid var(--border);
    transition: all var(--transition);
  }
  .style-btn:hover { color: var(--text-primary); background: var(--bg-hover); }
  .style-btn.active { background: var(--accent-dim); color: var(--accent); border-color: var(--accent); }

  /* Eraser size buttons */
  .size-btn {
    padding: 3px 9px; border-radius: var(--radius-sm);
    font-size: 11px; font-weight: 700;
    color: var(--text-muted); background: var(--bg-overlay); border: 1px solid var(--border);
    transition: all var(--transition);
  }
  .size-btn:hover { color: var(--text-primary); background: var(--bg-hover); }
  .size-btn.active { background: var(--accent-dim); color: var(--accent); border-color: var(--accent); }

  /* Pen */
  .color-row { display: flex; gap: 4px; align-items: center; }
  .color-sw {
    width: 18px; height: 18px; border-radius: 50%;
    border: 2px solid transparent; transition: transform var(--transition);
  }
  .color-sw:hover { transform: scale(1.2); }
  .color-sw.sel { outline: 2px solid var(--text-primary); outline-offset: 2px; }

  .width-row { display: flex; align-items: center; gap: 5px; }
  .width-row input[type=range] { width: 72px; accent-color: var(--accent); }

  /* Add menu */
  .add-menu {
    position: fixed;
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius-md); padding: 6px; min-width: 180px;
    z-index: 50; box-shadow: 0 8px 32px rgba(0,0,0,0.4);
  }
  .menu-sep {
    height: 1px; background: var(--border); margin: 4px 0;
  }
  .menu-item {
    display: flex; align-items: center; gap: 10px;
    width: 100%; padding: 8px 12px; border-radius: var(--radius-sm);
    font-size: 13px; color: var(--text-primary);
    transition: background var(--transition);
  }
  .menu-item:hover { background: var(--bg-hover); }
  .menu-item-canva { color: #7d2ae8; }
  .menu-item-canva:hover { background: rgba(125,42,232,0.1); }
  .menu-item-youtube { color: #ff0000; }
  .menu-item-youtube:hover { background: rgba(255,0,0,0.08); }
  .menu-icon { display: flex; align-items: center; justify-content: center; width: 18px; height: 18px; flex-shrink: 0; }

  /* Zoom */
  .zoom-group {
    display: flex; align-items: center; gap: 2px;
    margin-left: auto; background: var(--bg-overlay);
    border: 1px solid var(--border); border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .zoom-btn {
    padding: 4px 10px; font-size: 15px; font-weight: 700;
    color: var(--text-secondary);
    transition: background var(--transition), color var(--transition);
  }
  .zoom-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .zoom-pct {
    padding: 4px 8px; font-size: 12px; font-weight: 600;
    color: var(--text-primary); min-width: 44px; text-align: center;
    border-left: 1px solid var(--border); border-right: 1px solid var(--border);
    transition: background var(--transition);
  }
  .zoom-pct:hover { background: var(--accent-dim); color: var(--accent); }
</style>
