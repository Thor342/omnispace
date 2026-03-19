<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { spaces, pagesMap, activeSpaceId, activePageId } from "../stores/spaces";
  import { getPages, getBlocks } from "../api";

  export let onClose: () => void;

  let query = "";
  let inputEl: HTMLInputElement;
  let results: { spaceId: string; spaceName: string; spaceIcon: string; pageId: string; pageTitle: string; blockType: string; label: string; preview: string }[] = [];
  let searching = false;
  let selectedIdx = 0;

  onMount(async () => {
    await tick();
    inputEl?.focus();
    window.addEventListener("keydown", onKey);
  });
  onDestroy(() => window.removeEventListener("keydown", onKey));

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") { onClose(); return; }
    if (e.key === "ArrowDown") { e.preventDefault(); selectedIdx = Math.min(selectedIdx + 1, results.length - 1); }
    if (e.key === "ArrowUp")   { e.preventDefault(); selectedIdx = Math.max(selectedIdx - 1, 0); }
    if (e.key === "Enter" && results[selectedIdx]) navigate(results[selectedIdx]);
  }

  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  $: if (query.trim().length >= 2) {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => runSearch(query.trim()), 250);
  } else {
    results = [];
  }

  function extractLabel(blockType: string, content: string): { label: string; preview: string } {
    try {
      const d = JSON.parse(content || "{}");
      if (blockType === "note") return { label: d.title || "Nota", preview: d.text?.slice(0, 80) || "" };
      if (blockType === "link") return { label: d.title || d.url || "Enlace", preview: d.url || "" };
      if (blockType === "task") {
        const done = d.tasks?.filter((t: any) => t.completed).length ?? 0;
        const total = d.tasks?.length ?? 0;
        return { label: "Tareas", preview: `${done}/${total} completadas` };
      }
      if (blockType === "file") return { label: d.name || "Documento", preview: d.file_type || "" };
    } catch {}
    return { label: blockType, preview: "" };
  }

  async function runSearch(q: string) {
    searching = true;
    results = [];
    selectedIdx = 0;
    const lower = q.toLowerCase();
    const allSpaces = $spaces;

    for (const space of allSpaces) {
      // Asegurar que las páginas del espacio están cargadas
      let pages = $pagesMap[space.id];
      if (!pages) {
        try {
          pages = await getPages(space.id);
          pagesMap.update(m => ({ ...m, [space.id]: pages }));
        } catch { continue; }
      }

      for (const page of pages) {
        // Título de página coincide
        if (page.title.toLowerCase().includes(lower)) {
          results.push({
            spaceId: space.id, spaceName: space.name, spaceIcon: space.icon,
            pageId: page.id, pageTitle: page.title,
            blockType: "page", label: page.title, preview: "Página"
          });
        }

        // Buscar en bloques de la página
        try {
          const blocks = await getBlocks(page.id);
          for (const b of blocks) {
            if (b.block_type === "shape" || b.block_type === "clock" || b.block_type === "calendar") continue;
            const { label, preview } = extractLabel(b.block_type, b.content);
            const searchable = (label + " " + preview + " " + b.content).toLowerCase();
            if (searchable.includes(lower)) {
              results.push({
                spaceId: space.id, spaceName: space.name, spaceIcon: space.icon,
                pageId: page.id, pageTitle: page.title,
                blockType: b.block_type, label, preview: preview.slice(0, 80)
              });
            }
          }
        } catch {}
      }
    }
    searching = false;
  }

  function navigate(r: typeof results[0]) {
    activeSpaceId.set(r.spaceId);
    // Esperar que las páginas del espacio carguen y luego activar la página
    setTimeout(() => activePageId.set(r.pageId), 100);
    onClose();
  }

  function blockIcon(type: string): string {
    if (type === "note")     return "📝";
    if (type === "link")     return "🔗";
    if (type === "task")     return "✅";
    if (type === "file")     return "📄";
    if (type === "calendar") return "📅";
    if (type === "page")     return "📋";
    return "◻";
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="overlay-backdrop" on:click={e => e.target === e.currentTarget && onClose()}>
  <div class="search-panel" role="dialog" aria-label="Búsqueda global">
    <div class="search-bar">
      <span class="search-ico">🔍</span>
      <input
        bind:this={inputEl}
        bind:value={query}
        class="search-input"
        placeholder="Buscar en todos los espacios…"
        autocomplete="off"
      />
      {#if searching}
        <span class="search-spin">⏳</span>
      {:else if query}
        <button class="search-clear" on:click={() => { query = ""; results = []; inputEl?.focus(); }}>✕</button>
      {/if}
    </div>

    {#if query.length >= 2}
      <div class="results-list">
        {#if results.length === 0 && !searching}
          <div class="no-results">Sin resultados para "<strong>{query}</strong>"</div>
        {:else}
          {#each results as r, i}
            <!-- svelte-ignore a11y-no-static-element-interactions -->
            <div
              class="result-item"
              class:selected={selectedIdx === i}
              on:click={() => navigate(r)}
              on:mouseenter={() => selectedIdx = i}
            >
              <span class="result-block-icon">{blockIcon(r.blockType)}</span>
              <div class="result-text">
                <span class="result-label">{r.label}</span>
                {#if r.preview}
                  <span class="result-preview">{r.preview}</span>
                {/if}
              </div>
              <div class="result-path">
                <span class="result-space">{r.spaceIcon} {r.spaceName}</span>
                <span class="result-sep">›</span>
                <span class="result-page">{r.pageTitle}</span>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    {:else}
      <div class="search-hint">Escribe al menos 2 caracteres para buscar</div>
    {/if}

    <div class="search-footer">
      <span>↑↓ navegar</span><span>↵ abrir</span><span>Esc cerrar</span>
    </div>
  </div>
</div>

<style>
  .overlay-backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex; align-items: flex-start; justify-content: center;
    padding-top: 12vh;
    z-index: 99990;
    backdrop-filter: blur(3px);
    animation: fade-in 0.1s ease;
  }
  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }

  .search-panel {
    width: 580px; max-width: 94vw;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 24px 60px rgba(0,0,0,0.5);
    overflow: hidden;
    animation: slide-in 0.12s ease;
  }
  @keyframes slide-in {
    from { opacity: 0; transform: translateY(-12px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .search-bar {
    display: flex; align-items: center; gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border);
  }
  .search-ico { font-size: 16px; color: var(--text-muted); flex-shrink: 0; }
  .search-input {
    flex: 1; background: transparent; border: none; outline: none;
    font-size: 16px; color: var(--text-primary);
    caret-color: var(--accent);
  }
  .search-input::placeholder { color: var(--text-muted); }
  .search-spin { font-size: 14px; }
  .search-clear {
    font-size: 14px; color: var(--text-muted); padding: 2px 5px;
    border-radius: 4px; transition: all 0.1s;
  }
  .search-clear:hover { color: var(--text-primary); background: var(--bg-hover); }

  .results-list { max-height: 400px; overflow-y: auto; }

  .result-item {
    display: flex; align-items: center; gap: 10px;
    padding: 9px 16px; cursor: pointer;
    transition: background 0.1s;
    border-bottom: 1px solid var(--border);
  }
  .result-item:last-child { border-bottom: none; }
  .result-item.selected, .result-item:hover { background: var(--accent-dim); }
  .result-item.selected .result-label { color: var(--accent); }

  .result-block-icon { font-size: 15px; flex-shrink: 0; }

  .result-text {
    flex: 1; min-width: 0;
    display: flex; flex-direction: column; gap: 1px;
  }
  .result-label {
    font-size: 13px; font-weight: 500; color: var(--text-primary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .result-preview {
    font-size: 11px; color: var(--text-muted);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  .result-path {
    display: flex; align-items: center; gap: 4px;
    font-size: 10px; color: var(--text-muted);
    flex-shrink: 0; white-space: nowrap;
  }
  .result-sep { opacity: 0.5; }

  .no-results {
    padding: 24px 16px; text-align: center;
    font-size: 13px; color: var(--text-muted);
  }
  .no-results strong { color: var(--text-secondary); }

  .search-hint {
    padding: 20px 16px; text-align: center;
    font-size: 12px; color: var(--text-muted);
  }

  .search-footer {
    display: flex; gap: 16px; padding: 8px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-overlay);
    font-size: 11px; color: var(--text-muted);
  }
</style>
