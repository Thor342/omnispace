<script lang="ts">
  import { onMount } from "svelte";
  import { emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getPages } from "../api";
  import PageCanvas from "./canvas/PageCanvas.svelte";

  export let pageId: string;
  export let spaceId: string;

  let pageTitle = "";

  onMount(async () => {
    try {
      const pages = await getPages(spaceId);
      const page = pages.find(p => p.id === pageId);
      if (page) pageTitle = page.title;
    } catch {}
  });

  async function reattach() {
    await emit("reattach-tab", { pageId, spaceId });
    const win = getCurrentWindow();
    await win.close();
  }
</script>

<div class="detached-root">
  <header class="detached-header">
    <div class="drag-hint" data-tauri-drag-region>
      <span class="page-icon">📄</span>
      <span class="page-title">{pageTitle || "…"}</span>
    </div>
    <button class="reunir-btn" on:click={reattach} title="Reunir con la ventana principal">
      ↩ Reunir
    </button>
  </header>
  <div class="canvas-wrap">
    <PageCanvas {pageId} {spaceId} />
  </div>
</div>

<style>
  .detached-root {
    display: flex; flex-direction: column;
    height: 100vh; width: 100vw;
    background: var(--bg-base);
    overflow: hidden;
  }

  .detached-header {
    display: flex; align-items: center;
    padding: 0 12px;
    height: 42px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
    gap: 8px;
  }

  .drag-hint {
    flex: 1; display: flex; align-items: center; gap: 8px;
    min-width: 0; cursor: move;
  }

  .page-icon { font-size: 15px; flex-shrink: 0; }

  .page-title {
    font-size: 13px; font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }

  .reunir-btn {
    display: flex; align-items: center; gap: 5px;
    padding: 5px 12px; border-radius: var(--radius-sm);
    background: var(--accent); color: #fff;
    font-size: 12px; font-weight: 600;
    flex-shrink: 0;
    transition: opacity 0.15s;
  }
  .reunir-btn:hover { opacity: 0.82; }

  .canvas-wrap {
    flex: 1; overflow: hidden;
    display: flex; flex-direction: column;
  }
</style>
