<script lang="ts">
  import { onMount } from "svelte";
  import DOMPurify from "dompurify";
  import { t } from "../../stores/language";

  export let content: string;
  export let onContentChange: (c: string) => void;

  // ── Parse (backward-compatible con notas antiguas de texto plano) ──
  function parse(c: string): { title: string; html: string } {
    try {
      const d = JSON.parse(c || "{}");
      if (typeof d.html === "string") return { title: d.title ?? "", html: d.html };
      if (typeof d.text === "string") {
        // Convertir texto plano a HTML preservando saltos de línea
        const html = d.text
          .replace(/&/g, "&amp;")
          .replace(/</g, "&lt;")
          .replace(/>/g, "&gt;")
          .replace(/\n/g, "<br>");
        return { title: d.title ?? "", html };
      }
      return { title: "", html: "" };
    } catch { return { title: "", html: "" }; }
  }

  let parsed = parse(content);
  let noteTitle = parsed.title;
  let timer: ReturnType<typeof setTimeout>;
  let editorEl: HTMLDivElement;
  let editorFocused = false;

  onMount(() => {
    editorEl.innerHTML = parsed.html;
  });

  // Solo sincronizar desde afuera si el editor NO tiene el foco.
  // Si el usuario está escribiendo, nunca tocamos innerHTML (evita que el cursor salte).
  $: if (!editorFocused && editorEl) {
    const np = parse(content);
    noteTitle = np.title;
    editorEl.innerHTML = np.html;
  }

  function schedule() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      const html = DOMPurify.sanitize(editorEl.innerHTML);
      onContentChange(JSON.stringify({ title: noteTitle, html }));
    }, 800);
  }

  // ── Formateo ──────────────────────────────────────────────
  function fmt(cmd: string, value?: string) {
    editorEl.focus();
    document.execCommand(cmd, false, value);
    schedule();
  }

  function onTbMousedown(e: MouseEvent, cmd: string, value?: string) {
    e.preventDefault(); // evita que el editor pierda el foco/selección
    fmt(cmd, value);
  }

  // Atajos de teclado ya los maneja el navegador (Ctrl+B, Ctrl+I, Ctrl+U)
  function onEditorKeydown(e: KeyboardEvent) {
    if (e.key === "Tab") {
      e.preventDefault();
      document.execCommand("insertText", false, "    ");
    }
  }
</script>

<div class="note-block">
  <input
    class="note-title"
    bind:value={noteTitle}
    placeholder={$t.noteBlock.titlePlaceholder}
    on:input={schedule}
  />

  <!-- Toolbar de formato -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="toolbar" on:mousedown|stopPropagation>
    <button class="tb-btn heading" title={$t.noteBlock.h1} on:mousedown={e => onTbMousedown(e, "formatBlock", "H1")}>H1</button>
    <button class="tb-btn heading" title={$t.noteBlock.h2} on:mousedown={e => onTbMousedown(e, "formatBlock", "H2")}>H2</button>
    <button class="tb-btn heading" title={$t.noteBlock.h3} on:mousedown={e => onTbMousedown(e, "formatBlock", "H3")}>H3</button>
    <div class="tb-sep"></div>
    <button class="tb-btn" title={$t.noteBlock.bold} on:mousedown={e => onTbMousedown(e, "bold")}><b>B</b></button>
    <button class="tb-btn italic" title={$t.noteBlock.italic} on:mousedown={e => onTbMousedown(e, "italic")}><i>I</i></button>
    <button class="tb-btn underline" title={$t.noteBlock.underline} on:mousedown={e => onTbMousedown(e, "underline")}><u>U</u></button>
    <button class="tb-btn strike" title={$t.noteBlock.strike} on:mousedown={e => onTbMousedown(e, "strikeThrough")}><s>S</s></button>
    <div class="tb-sep"></div>
    <button class="tb-btn" title={$t.noteBlock.bulletList} on:mousedown={e => onTbMousedown(e, "insertUnorderedList")}>•≡</button>
    <button class="tb-btn" title={$t.noteBlock.numberedList} on:mousedown={e => onTbMousedown(e, "insertOrderedList")}>1≡</button>
    <div class="tb-sep"></div>
    <button class="tb-btn" title={$t.noteBlock.paragraph} on:mousedown={e => onTbMousedown(e, "formatBlock", "P")}>¶</button>
  </div>

  <!-- Editor rico -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <!-- svelte-ignore a11y-interactive-supports-focus -->
  <div
    class="note-editor"
    contenteditable="true"
    bind:this={editorEl}
    on:input={schedule}
    on:keydown={onEditorKeydown}
    on:focus={() => editorFocused = true}
    on:blur={() => editorFocused = false}
    spellcheck="false"
    role="textbox"
    aria-multiline="true"
    aria-label="Editor de nota"
  ></div>
</div>

<style>
  .note-block {
    display: flex; flex-direction: column; flex: 1; overflow: hidden;
    background: #FFFBEB;
    position: relative;
  }

  .note-title {
    background: transparent; border: none;
    font-size: calc(13.5px * var(--s, 1)); font-weight: 700; color: #5C4400;
    padding: calc(9px * var(--s, 1)) calc(16px * var(--s, 1)) calc(7px * var(--s, 1));
    flex-shrink: 0; font-family: inherit;
    border-bottom: 1.5px solid rgba(180,140,0,0.22);
    line-height: calc(24px * var(--s, 1));
  }
  .note-title::placeholder { color: rgba(92,68,0,0.28); }
  .note-title:focus { outline: none; }

  /* ── Toolbar ── */
  .toolbar {
    display: flex; align-items: center; gap: 1px;
    padding: calc(4px * var(--s, 1)) calc(8px * var(--s, 1));
    border-bottom: 1px solid rgba(180,140,0,0.2);
    flex-shrink: 0;
    flex-wrap: wrap;
    background: rgba(255,251,220,0.8);
  }

  .tb-btn {
    padding: calc(3px * var(--s, 1)) calc(7px * var(--s, 1));
    border-radius: calc(4px * var(--s, 1));
    font-size: calc(11px * var(--s, 1)); font-weight: 600;
    color: #6B5000; background: transparent;
    cursor: pointer; line-height: 1.4;
    transition: background 0.15s;
  }
  .tb-btn:hover { background: rgba(180,140,0,0.15); }
  .tb-btn.heading { font-family: serif; font-size: calc(10px * var(--s, 1)); }
  .tb-btn.italic i { font-style: italic; }
  .tb-btn.underline u { text-decoration: underline; }
  .tb-btn.strike s { text-decoration: line-through; }

  .tb-sep {
    width: 1px; height: calc(14px * var(--s, 1));
    background: rgba(180,140,0,0.3);
    margin: 0 calc(3px * var(--s, 1));
  }

  /* ── Editor rico ── */
  .note-editor {
    flex: 1; overflow-y: auto;
    padding: calc(5px * var(--s, 1)) calc(16px * var(--s, 1)) calc(10px * var(--s, 1));
    font-size: calc(13px * var(--s, 1));
    line-height: calc(24px * var(--s, 1));
    color: #4A3800;
    font-family: inherit;
    outline: none;
    word-break: break-word;
    background-image: repeating-linear-gradient(
      transparent 0px, transparent calc(23px * var(--s, 1)),
      rgba(180,140,0,0.18) calc(23px * var(--s, 1)), rgba(180,140,0,0.18) calc(24px * var(--s, 1))
    );
    background-size: 100% calc(24px * var(--s, 1));
    background-attachment: local;
    background-position-y: calc(5px * var(--s, 1));
  }

  .note-editor :global(h1) { font-size: calc(16px * var(--s, 1)); font-weight: 800; color: #3A2600; margin: 0; line-height: calc(24px * var(--s, 1)); }
  .note-editor :global(h2) { font-size: calc(14px * var(--s, 1)); font-weight: 700; color: #3E2A00; margin: 0; line-height: calc(24px * var(--s, 1)); }
  .note-editor :global(h3) { font-size: calc(13px * var(--s, 1)); font-weight: 700; color: #4A3000; margin: 0; line-height: calc(24px * var(--s, 1)); }
  .note-editor :global(p)  { margin: 0; line-height: calc(24px * var(--s, 1)); }
  .note-editor :global(ul), .note-editor :global(ol) { padding-left: calc(20px * var(--s, 1)); margin: 0; }
  .note-editor :global(li) { margin: 0; line-height: calc(24px * var(--s, 1)); }
  .note-editor :global(b), .note-editor :global(strong) { color: #3A2800; }
  .note-editor :global(u) { text-decoration-color: rgba(100,70,0,0.5); }
</style>
