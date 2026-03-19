<script lang="ts">
  import { onDestroy } from "svelte";

  export let content: string;
  export let onContentChange: (c: string) => void = () => {};

  type ShapeType = "rect" | "circle" | "triangle" | "diamond";
  interface ShapeData { shape: ShapeType; fill: string; stroke: string; text: string; }

  function parse(s: string): ShapeData {
    try { return { shape: "rect", fill: "#ffffff", stroke: "#1e1e2e", text: "", ...JSON.parse(s || "{}") }; }
    catch { return { shape: "rect", fill: "#ffffff", stroke: "#1e1e2e", text: "" }; }
  }

  let isDirty = false;
  let data: ShapeData = parse(content);
  $: if (!isDirty) { data = parse(content); }

  let timer: ReturnType<typeof setTimeout>;
  function save() {
    isDirty = true;
    clearTimeout(timer);
    timer = setTimeout(() => { onContentChange(JSON.stringify(data)); isDirty = false; }, 400);
  }

  function setShape(s: ShapeType) { data = { ...data, shape: s }; save(); }
  function setFill(f: string)     { data = { ...data, fill: f }; save(); }
  function setStroke(k: string)   { data = { ...data, stroke: k }; save(); }
  function onFillInput(e: Event)   { setFill((e.target as HTMLInputElement).value); }
  function onStrokeInput(e: Event) { setStroke((e.target as HTMLInputElement).value); }

  // ── Text editing ─────────────────────────────────────
  let editing = false;
  let textareaEl: HTMLTextAreaElement;

  function startEdit(e: MouseEvent) {
    e.stopPropagation();
    editing = true;
    setTimeout(() => textareaEl?.focus(), 0);
  }
  function commitText() {
    editing = false;
    onContentChange(JSON.stringify(data));
    isDirty = false;
  }

  // ── SVG path helpers ─────────────────────────────────
  const TRIANGLE_PATH = "M50,8 L92,88 L8,88 Z";
  const DIAMOND_PATH  = "M50,5 L95,50 L50,95 L5,50 Z";

  const FILL_COLORS  = ["#ffffff","#f3f4f6","#fef9c3","#dbeafe","#dcfce7","#fce7f3","#6366f1","#ec4899","#1e1e2e"];
  const STROKE_COLORS = ["#1e1e2e","#6366f1","#ec4899","#f59e0b","#22c55e","#ef4444","#14b8a6","#9ca3af","#ffffff"];

  const SHAPES = [
    { type: "rect",     icon: "◻" },
    { type: "circle",   icon: "◯" },
    { type: "triangle", icon: "△" },
    { type: "diamond",  icon: "◇" },
  ] as const;

  onDestroy(() => clearTimeout(timer));
</script>

<div class="shape-block">
  <!-- Mini toolbar -->
  <div class="shape-toolbar">
    <div class="shape-types">
      {#each SHAPES as s}
        <button
          class="shape-btn"
          class:active={data.shape === s.type}
          on:click={() => setShape(s.type)}
          title={s.type}
        >{s.icon}</button>
      {/each}
    </div>

    <div class="tb-sep" />

    <div class="color-group">
      <span class="tb-lbl">Relleno</span>
      <div class="swatches">
        {#each FILL_COLORS as c}
          <button
            class="swatch"
            class:active={data.fill === c}
            style="background:{c}"
            on:click={() => setFill(c)}
          />
        {/each}
        <input type="color" class="color-input" value={data.fill} on:input={onFillInput} title="Color personalizado" />
      </div>
    </div>

    <div class="tb-sep" />

    <div class="color-group">
      <span class="tb-lbl">Borde</span>
      <div class="swatches">
        {#each STROKE_COLORS as c}
          <button
            class="swatch"
            class:active={data.stroke === c}
            style="background:{c}"
            on:click={() => setStroke(c)}
          />
        {/each}
        <input type="color" class="color-input" value={data.stroke} on:input={onStrokeInput} title="Color personalizado" />
      </div>
    </div>
  </div>

  <!-- Shape canvas -->
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="shape-canvas" on:dblclick={startEdit}>
    <svg viewBox="0 0 100 100" preserveAspectRatio="xMidYMid meet" class="shape-svg">
      {#if data.shape === "rect"}
        <rect x="4" y="4" width="92" height="92" rx="14"
          fill={data.fill} stroke={data.stroke} stroke-width="2.5" />
      {:else if data.shape === "circle"}
        <circle cx="50" cy="50" r="45"
          fill={data.fill} stroke={data.stroke} stroke-width="2.5" />
      {:else if data.shape === "triangle"}
        <path d={TRIANGLE_PATH}
          fill={data.fill} stroke={data.stroke} stroke-width="2.5" stroke-linejoin="round" />
      {:else if data.shape === "diamond"}
        <path d={DIAMOND_PATH}
          fill={data.fill} stroke={data.stroke} stroke-width="2.5" stroke-linejoin="round" />
      {/if}
    </svg>

    <!-- Text overlay -->
    {#if editing}
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <div class="text-overlay editing" on:click|stopPropagation>
        <textarea
          bind:this={textareaEl}
          bind:value={data.text}
          on:blur={commitText}
          on:input={save}
          on:keydown={e => { if (e.key === "Escape") commitText(); }}
          placeholder="Escribe aquí…"
          class="text-editor"
        />
      </div>
    {:else if data.text}
      <div class="text-overlay">
        <span class="text-display">{data.text}</span>
      </div>
    {:else}
      <div class="text-hint">doble clic para escribir</div>
    {/if}
  </div>
</div>

<style>
  .shape-block {
    display: flex; flex-direction: column; flex: 1; overflow: hidden;
    background: var(--bg-base);
  }

  /* Toolbar */
  .shape-toolbar {
    display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
    padding: 5px 8px; border-bottom: 1px solid var(--border);
    background: var(--bg-overlay); flex-shrink: 0;
  }
  .tb-sep { width: 1px; height: 18px; background: var(--border); flex-shrink: 0; }
  .tb-lbl { font-size: 10px; color: var(--text-muted); white-space: nowrap; }

  .shape-types { display: flex; gap: 2px; }
  .shape-btn {
    padding: 2px 7px; border-radius: var(--radius-sm);
    font-size: 14px; color: var(--text-muted);
    background: transparent; border: 1px solid transparent;
    transition: all var(--transition);
  }
  .shape-btn:hover { color: var(--text-primary); background: var(--bg-hover); }
  .shape-btn.active { color: var(--accent); background: var(--accent-dim); border-color: var(--accent); }

  .color-group { display: flex; align-items: center; gap: 4px; }
  .swatches { display: flex; gap: 3px; align-items: center; }
  .swatch {
    width: 14px; height: 14px; border-radius: 50%;
    border: 2px solid transparent; transition: transform var(--transition), outline var(--transition);
    flex-shrink: 0;
  }
  .swatch:hover { transform: scale(1.25); }
  .swatch.active { outline: 2px solid var(--text-primary); outline-offset: 1px; }

  .color-input {
    width: 18px; height: 18px; border-radius: 50%; border: 1px solid var(--border);
    padding: 0; cursor: pointer; background: none; overflow: hidden;
  }
  .color-input::-webkit-color-swatch-wrapper { padding: 0; }
  .color-input::-webkit-color-swatch { border: none; border-radius: 50%; }

  /* Shape canvas */
  .shape-canvas {
    flex: 1; position: relative;
    display: flex; align-items: center; justify-content: center;
    padding: 12px; overflow: hidden; cursor: default;
  }

  .shape-svg {
    width: 100%; height: 100%;
    max-width: 100%; max-height: 100%;
  }

  /* Text */
  .text-overlay {
    position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
    pointer-events: none;
  }
  .text-overlay.editing { pointer-events: all; }

  .text-display {
    max-width: 70%; text-align: center; word-break: break-word;
    font-size: clamp(10px, 2.5vw, 15px); font-weight: 600;
    color: #fff; text-shadow: 0 1px 3px rgba(0,0,0,0.6);
    line-height: 1.3; pointer-events: none;
  }

  .text-editor {
    width: 60%; min-height: 48px; resize: none;
    background: rgba(0,0,0,0.45); color: #fff;
    border: 1px solid rgba(255,255,255,0.4); border-radius: 4px;
    padding: 5px 8px; font-size: 13px; font-weight: 600;
    text-align: center; line-height: 1.4;
    outline: none; backdrop-filter: blur(4px);
  }
  .text-editor::placeholder { color: rgba(255,255,255,0.5); }

  .text-hint {
    position: absolute; bottom: 8px; left: 50%; transform: translateX(-50%);
    font-size: 10px; color: var(--text-muted); pointer-events: none;
    white-space: nowrap; opacity: 0;
    transition: opacity var(--transition);
  }
  .shape-canvas:hover .text-hint { opacity: 1; }
</style>
