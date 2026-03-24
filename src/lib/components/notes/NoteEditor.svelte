<script lang="ts">
  import { onMount } from "svelte";
  import { marked } from "marked";
  import DOMPurify from "dompurify";
  import { getNotes, createNote, updateNote, deleteNote } from "../../api";
  import type { Note } from "../../types";

  export let spaceId: string;

  let notes: Note[] = [];
  let activeNote: Note | null = null;
  let editContent = "";
  let editTitle = "";
  let preview = false;
  let saving = false;
  let saveTimer: ReturnType<typeof setTimeout>;

  onMount(() => load());

  $: if (spaceId) load();

  async function load() {
    notes = await getNotes(spaceId);
    if (notes.length > 0 && !activeNote) selectNote(notes[0]);
    else if (notes.length === 0) activeNote = null;
  }

  function selectNote(note: Note) {
    activeNote = note;
    editTitle = note.title;
    editContent = note.content;
    preview = false;
  }

  async function handleNew() {
    const note = await createNote(spaceId, "Nueva nota", "");
    notes = [note, ...notes];
    selectNote(note);
  }

  function scheduleAutoSave() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 1200);
  }

  async function save() {
    if (!activeNote) return;
    saving = true;
    await updateNote(activeNote.id, editTitle, editContent);
    notes = notes.map((n) =>
      n.id === activeNote!.id ? { ...n, title: editTitle, content: editContent } : n
    );
    activeNote = { ...activeNote, title: editTitle, content: editContent };
    saving = false;
  }

  async function handleDelete(id: string) {
    await deleteNote(id);
    notes = notes.filter((n) => n.id !== id);
    if (activeNote?.id === id) {
      activeNote = notes.length > 0 ? notes[0] : null;
      if (activeNote) selectNote(activeNote);
    }
  }

  $: renderedMd = preview && editContent
    ? DOMPurify.sanitize(marked.parse(editContent) as string)
    : "";
</script>

<div class="note-layout">
  <!-- Note List -->
  <div class="note-list">
    <div class="note-list-header">
      <span class="count">{notes.length} notas</span>
      <button class="add-btn" on:click={handleNew} title="Nueva nota">+</button>
    </div>
    <div class="note-items">
      {#each notes as note (note.id)}
        <div
          class="note-item"
          class:active={activeNote?.id === note.id}
          role="button"
          tabindex="0"
          on:click={() => selectNote(note)}
          on:keypress={(e) => e.key === "Enter" && selectNote(note)}
        >
          <div class="note-item-title truncate">{note.title || "Sin título"}</div>
          <div class="note-item-preview truncate">
            {note.content.replace(/[#*`_>]/g, "").slice(0, 60) || "Vacía"}
          </div>
          <button
            class="note-delete"
            on:click|stopPropagation={() => handleDelete(note.id)}
          >×</button>
        </div>
      {/each}
      {#if notes.length === 0}
        <p class="empty-hint">Sin notas aún. Crea una ↑</p>
      {/if}
    </div>
  </div>

  <!-- Editor -->
  <div class="editor-area">
    {#if activeNote}
      <div class="editor-toolbar">
        <input
          class="title-input"
          bind:value={editTitle}
          placeholder="Título..."
          on:input={scheduleAutoSave}
        />
        <div class="toolbar-actions">
          {#if saving}<span class="saving-badge">Guardando…</span>{/if}
          <button
            class="toolbar-btn"
            class:active={preview}
            on:click={() => preview = !preview}
            title={preview ? "Editar" : "Vista previa"}
          >
            {preview ? "✏️ Editar" : "👁 Preview"}
          </button>
          <button class="toolbar-btn" on:click={save} title="Guardar ahora">💾</button>
        </div>
      </div>

      {#if preview}
        <div class="preview-pane md-content">
          {@html renderedMd}
        </div>
      {:else}
        <textarea
          class="md-editor"
          bind:value={editContent}
          placeholder="Escribe en Markdown... (# Título, **negrita**, - lista)"
          on:input={scheduleAutoSave}
          spellcheck="false"
        />
      {/if}
    {:else}
      <div class="no-note">
        <span>📝</span>
        <p>Selecciona o crea una nota</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .note-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  /* ── Note list ── */
  .note-list {
    width: 220px;
    min-width: 220px;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
  }

  .note-list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 8px;
  }

  .count { font-size: 11px; color: var(--text-muted); }

  .add-btn {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    background: var(--accent-dim);
    color: var(--accent);
    font-size: 18px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background var(--transition);
  }

  .add-btn:hover { background: var(--accent); color: #fff; }

  .note-items { flex: 1; overflow-y: auto; padding: 4px 8px; }

  .note-item {
    padding: 9px 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    position: relative;
    transition: background var(--transition);
    margin-bottom: 2px;
  }

  .note-item:hover { background: var(--bg-hover); }
  .note-item.active { background: var(--accent-dim); }

  .note-item-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .note-item-preview {
    font-size: 11px;
    color: var(--text-muted);
  }

  .note-delete {
    position: absolute;
    top: 6px; right: 6px;
    font-size: 14px;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity var(--transition), color var(--transition);
    line-height: 1;
    padding: 2px 4px;
  }

  .note-item:hover .note-delete { opacity: 1; }
  .note-delete:hover { color: var(--red); }

  .empty-hint {
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
    padding: 20px 8px;
  }

  /* ── Editor ── */
  .editor-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-surface);
  }

  .title-input {
    flex: 1;
    background: transparent;
    border: none;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    padding: 4px 0;
  }

  .title-input:focus { border-color: transparent; }

  .toolbar-actions { display: flex; align-items: center; gap: 8px; }

  .saving-badge {
    font-size: 11px;
    color: var(--text-muted);
    animation: pulse 1s infinite;
  }

  @keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.4; } }

  .toolbar-btn {
    padding: 5px 10px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    transition: all var(--transition);
  }

  .toolbar-btn:hover, .toolbar-btn.active {
    background: var(--accent-dim);
    color: var(--accent);
    border-color: var(--accent);
  }

  .md-editor {
    flex: 1;
    resize: none;
    border: none;
    border-radius: 0;
    background: var(--bg-base);
    padding: 20px 24px;
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 1.7;
    color: var(--text-primary);
    width: 100%;
  }

  .preview-pane {
    flex: 1;
    overflow-y: auto;
    padding: 20px 32px;
    max-width: 760px;
  }

  .no-note {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 14px;
  }

  .no-note span { font-size: 40px; }
</style>
