<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getFiles, importFile, deleteFile, readFileAsBase64 } from "../../api";
  import type { AppFile } from "../../types";

  export let spaceId: string;

  let files: AppFile[] = [];
  let selected: AppFile | null = null;
  let mediaData: string = "";
  let loading = false;

  onMount(() => load());
  $: if (spaceId) load();

  async function load() {
    files = await getFiles(spaceId);
    selected = null;
    mediaData = "";
  }

  async function handleImport() {
    const path = await open({
      multiple: false,
      filters: [
        { name: "Archivos", extensions: ["jpg","jpeg","png","gif","webp","bmp","svg","pdf","mp4","webm","mkv","avi","mov"] },
      ],
    });
    if (!path || typeof path !== "string") return;
    loading = true;
    try {
      const file = await importFile(spaceId, path);
      files = [file, ...files];
    } finally {
      loading = false;
    }
  }

  async function selectFile(file: AppFile) {
    selected = file;
    mediaData = "";
    if (file.file_type === "image") {
      const b64 = await readFileAsBase64(file.stored_path);
      const ext = file.name.split(".").pop()?.toLowerCase() || "png";
      const mime = ext === "svg" ? "image/svg+xml" : `image/${ext === "jpg" ? "jpeg" : ext}`;
      mediaData = `data:${mime};base64,${b64}`;
    } else if (file.file_type === "video") {
      const b64 = await readFileAsBase64(file.stored_path);
      mediaData = `data:video/mp4;base64,${b64}`;
    }
  }

  async function handleDelete(id: string) {
    await deleteFile(id);
    files = files.filter((f) => f.id !== id);
    if (selected?.id === id) { selected = null; mediaData = ""; }
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function fileIcon(type: string): string {
    if (type === "image") return "🖼️";
    if (type === "pdf")   return "📄";
    if (type === "video") return "🎬";
    return "📎";
  }
</script>

<div class="files-layout">
  <!-- File grid -->
  <div class="file-sidebar">
    <div class="file-toolbar">
      <button class="import-btn" on:click={handleImport} disabled={loading}>
        {loading ? "Importando…" : "+ Adjuntar"}
      </button>
    </div>

    <div class="file-grid">
      {#each files as file (file.id)}
        <div
          class="file-card"
          class:active={selected?.id === file.id}
          role="button"
          tabindex="0"
          on:click={() => selectFile(file)}
          on:keypress={(e) => e.key === "Enter" && selectFile(file)}
        >
          <span class="file-icon">{fileIcon(file.file_type)}</span>
          <span class="file-name truncate">{file.name}</span>
          <span class="file-size">{formatSize(file.size)}</span>
          <button
            class="file-delete"
            on:click|stopPropagation={() => handleDelete(file.id)}
          >×</button>
        </div>
      {/each}

      {#if files.length === 0}
        <div class="empty-files">
          <span>📎</span>
          <p>Sin archivos adjuntos</p>
          <p style="font-size:12px">Haz clic en "+ Adjuntar"</p>
        </div>
      {/if}
    </div>
  </div>

  <!-- Preview area -->
  <div class="preview-area">
    {#if !selected}
      <div class="no-preview">
        <span>🖼️</span>
        <p>Selecciona un archivo para previsualizar</p>
      </div>
    {:else if selected.file_type === "image" && mediaData}
      <img src={mediaData} alt={selected.name} class="img-preview" />
    {:else if selected.file_type === "video" && mediaData}
      <video controls class="video-preview">
        <source src={mediaData} />
        Tu navegador no soporta este formato.
      </video>
    {:else if selected.file_type === "pdf"}
      <div class="pdf-info">
        <span class="pdf-icon">📄</span>
        <h3>{selected.name}</h3>
        <p class="muted">{formatSize(selected.size)}</p>
        <p class="muted" style="margin-top:8px; font-size:12px">
          Los PDF se visualizan mediante el visor del sistema.<br/>
          Ruta: <code>{selected.stored_path}</code>
        </p>
      </div>
    {:else}
      <div class="no-preview">
        <span>{fileIcon(selected.file_type)}</span>
        <p>{selected.name}</p>
        <p class="muted">{formatSize(selected.size)}</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .files-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .file-sidebar {
    width: 240px;
    min-width: 240px;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
  }

  .file-toolbar {
    padding: 12px;
    border-bottom: 1px solid var(--border);
  }

  .import-btn {
    width: 100%;
    padding: 8px;
    background: var(--accent);
    color: #fff;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 500;
    transition: background var(--transition);
  }

  .import-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .import-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .file-grid {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .file-card {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    position: relative;
    transition: background var(--transition);
  }

  .file-card:hover { background: var(--bg-hover); }
  .file-card.active { background: var(--accent-dim); }

  .file-icon { font-size: 18px; flex-shrink: 0; }

  .file-name { flex: 1; font-size: 13px; color: var(--text-primary); }

  .file-size { font-size: 10px; color: var(--text-muted); flex-shrink: 0; }

  .file-delete {
    position: absolute;
    top: 6px; right: 4px;
    font-size: 14px;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity var(--transition), color var(--transition);
    padding: 2px 4px;
  }

  .file-card:hover .file-delete { opacity: 1; }
  .file-delete:hover { color: var(--red); }

  .empty-files {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 40px 16px;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }

  .empty-files span { font-size: 36px; }

  /* Preview */
  .preview-area {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: auto;
    background: var(--bg-base);
    padding: 20px;
  }

  .no-preview {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 14px;
  }

  .no-preview span { font-size: 48px; }

  .img-preview {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: var(--radius-md);
    box-shadow: 0 8px 32px rgba(0,0,0,0.4);
  }

  .video-preview {
    max-width: 100%;
    max-height: 100%;
    border-radius: var(--radius-md);
    box-shadow: 0 8px 32px rgba(0,0,0,0.4);
  }

  .pdf-info {
    text-align: center;
    color: var(--text-secondary);
  }

  .pdf-icon { font-size: 64px; display: block; margin-bottom: 12px; }
  .pdf-info h3 { color: var(--text-primary); font-size: 16px; }
  .muted { color: var(--text-muted); font-size: 13px; margin-top: 4px; }
  .pdf-info code { font-family: var(--font-mono); font-size: 11px; word-break: break-all; }
</style>
