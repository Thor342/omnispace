<script lang="ts">
  import type { NoteContent } from "../../types";

  export let content: string;
  export let onContentChange: (c: string) => void;

  function parse(c: string): NoteContent {
    try { return JSON.parse(c || '{"title":"","text":""}'); }
    catch { return { title: "", text: "" }; }
  }

  let data: NoteContent = parse(content);
  let isDirty = false;
  let timer: ReturnType<typeof setTimeout>;

  // Sync from prop changes (e.g., expand ↔ card switch) but only when not mid-edit
  $: if (!isDirty) { data = parse(content); }

  function schedule() {
    isDirty = true;
    clearTimeout(timer);
    timer = setTimeout(() => {
      onContentChange(JSON.stringify(data));
      isDirty = false;
    }, 800);
  }
</script>

<div class="note-block">
  <input
    class="note-title"
    bind:value={data.title}
    placeholder="Título…"
    on:input={schedule}
  />
  <textarea
    class="note-editor"
    bind:value={data.text}
    placeholder="Escribe aquí…"
    on:input={schedule}
    spellcheck="false"
  />
</div>

<style>
  .note-block {
    display: flex; flex-direction: column; flex: 1; overflow: hidden;
    background: #FFFBEB;
    position: relative;
  }
  .note-block::after {
    content: "";
    position: absolute; bottom: 0; left: 0; right: 0; height: 24px;
    background: linear-gradient(to bottom, transparent, rgba(200,160,0,0.06));
    pointer-events: none;
  }

  .note-title {
    background: transparent; border: none;
    font-size: 13.5px; font-weight: 700; color: #5C4400;
    padding: 9px 16px 7px;
    flex-shrink: 0; font-family: inherit;
    border-bottom: 1.5px solid rgba(180,140,0,0.22);
    line-height: 24px;
  }
  .note-title::placeholder { color: rgba(92,68,0,0.28); }
  .note-title:focus { outline: none; }

  .note-editor {
    flex: 1; resize: none; border: none; border-radius: 0;
    background: transparent;
    padding: 5px 16px 10px;
    font-size: 13px;
    line-height: 24px;
    color: #4A3800;
    font-family: inherit;
    background-image: repeating-linear-gradient(
      transparent 0px,
      transparent 23px,
      rgba(180,140,0,0.18) 23px,
      rgba(180,140,0,0.18) 24px
    );
    background-size: 100% 24px;
    background-attachment: local;
    background-position-y: 5px;
  }
  .note-editor::placeholder { color: rgba(74,56,0,0.28); }
  .note-editor:focus { outline: none; }
</style>
