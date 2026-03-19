<script lang="ts">
  import { onMount } from "svelte";
  import { getLinks, createLink, deleteLink } from "../../api";
  import type { Link } from "../../types";

  export let spaceId: string;

  let links: Link[] = [];
  let selected: Link | null = null;
  let newUrl = "";
  let newTitle = "";
  let adding = false;
  let error = "";

  onMount(() => load());
  $: if (spaceId) load();

  async function load() {
    links = await getLinks(spaceId);
    selected = links.length > 0 ? links[0] : null;
  }

  function getYoutubeEmbedUrl(url: string): string {
    // youtu.be/ID
    const shortMatch = url.match(/youtu\.be\/([^?&]+)/);
    if (shortMatch) return `https://www.youtube.com/embed/${shortMatch[1]}`;
    // youtube.com/watch?v=ID
    const longMatch = url.match(/[?&]v=([^&]+)/);
    if (longMatch) return `https://www.youtube.com/embed/${longMatch[1]}`;
    return "";
  }

  function isValidUrl(url: string): boolean {
    try { new URL(url); return true; } catch { return false; }
  }

  async function handleAdd() {
    error = "";
    if (!newUrl.trim()) { error = "Ingresa una URL"; return; }
    if (!isValidUrl(newUrl.trim())) { error = "URL no válida"; return; }
    const link = await createLink(spaceId, newTitle.trim() || newUrl.trim(), newUrl.trim());
    links = [link, ...links];
    selected = link;
    newUrl = "";
    newTitle = "";
    adding = false;
  }

  async function handleDelete(id: string) {
    await deleteLink(id);
    links = links.filter((l) => l.id !== id);
    if (selected?.id === id) selected = links.length > 0 ? links[0] : null;
  }
</script>

<div class="links-layout">
  <!-- Link list -->
  <div class="link-sidebar">
    <div class="link-toolbar">
      {#if !adding}
        <button class="add-btn" on:click={() => adding = true}>+ Añadir enlace</button>
      {:else}
        <div class="add-form">
          <input bind:value={newTitle} placeholder="Título (opcional)" />
          <input bind:value={newUrl}   placeholder="https://..." on:keydown={(e) => e.key === "Enter" && handleAdd()} />
          {#if error}<p class="err">{error}</p>{/if}
          <div class="form-actions">
            <button class="btn-accent" on:click={handleAdd}>Guardar</button>
            <button class="btn-ghost" on:click={() => { adding = false; error = ""; }}>Cancelar</button>
          </div>
        </div>
      {/if}
    </div>

    <div class="link-list">
      {#each links as link (link.id)}
        <div
          class="link-item"
          class:active={selected?.id === link.id}
          role="button"
          tabindex="0"
          on:click={() => selected = link}
          on:keypress={(e) => e.key === "Enter" && (selected = link)}
        >
          <span class="link-type-badge" class:yt={link.link_type === "youtube"}>
            {link.link_type === "youtube" ? "▶" : "🔗"}
          </span>
          <div class="link-info">
            <span class="link-title truncate">{link.title}</span>
            <span class="link-url truncate">{link.url}</span>
          </div>
          <button class="link-del" on:click|stopPropagation={() => handleDelete(link.id)}>×</button>
        </div>
      {/each}

      {#if links.length === 0}
        <div class="empty-links">
          <span>🔗</span>
          <p>Sin enlaces guardados</p>
        </div>
      {/if}
    </div>
  </div>

  <!-- Preview area -->
  <div class="link-preview">
    {#if !selected}
      <div class="no-preview">
        <span>🔗</span>
        <p>Selecciona un enlace para verlo</p>
      </div>
    {:else if selected.link_type === "youtube"}
      {@const embedUrl = getYoutubeEmbedUrl(selected.url)}
      {#if embedUrl}
        <div class="yt-wrapper">
          <h3>{selected.title}</h3>
          <div class="yt-embed-container">
            <iframe
              src={embedUrl}
              title={selected.title}
              frameborder="0"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
              allowfullscreen
            ></iframe>
          </div>
        </div>
      {/if}
    {:else}
      <div class="external-link">
        <span>🔗</span>
        <h3>{selected.title}</h3>
        <p class="url-text">{selected.url}</p>
        <p class="hint">Los enlaces externos se abren en el navegador.</p>
        <a href={selected.url} target="_blank" rel="noopener" class="open-btn">
          Abrir en navegador ↗
        </a>
      </div>
    {/if}
  </div>
</div>

<style>
  .links-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .link-sidebar {
    width: 280px;
    min-width: 280px;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
  }

  .link-toolbar { padding: 12px; border-bottom: 1px solid var(--border); }

  .add-btn {
    width: 100%;
    padding: 8px;
    background: var(--accent);
    color: #fff;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 500;
    transition: background var(--transition);
  }

  .add-btn:hover { background: var(--accent-hover); }

  .add-form {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .add-form input {
    width: 100%;
    padding: 7px 10px;
    font-size: 13px;
  }

  .err { color: var(--red); font-size: 11px; }

  .form-actions { display: flex; gap: 6px; }

  .btn-accent {
    flex: 1;
    padding: 7px;
    background: var(--accent);
    color: #fff;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 500;
  }

  .btn-ghost {
    flex: 1;
    padding: 7px;
    background: var(--bg-active);
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    font-size: 13px;
  }

  .link-list { flex: 1; overflow-y: auto; padding: 8px; }

  .link-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    position: relative;
    transition: background var(--transition);
    margin-bottom: 2px;
  }

  .link-item:hover { background: var(--bg-hover); }
  .link-item.active { background: var(--accent-dim); }

  .link-type-badge {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--bg-active);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    flex-shrink: 0;
  }

  .link-type-badge.yt { background: #ff0000; color: #fff; }

  .link-info { flex: 1; overflow: hidden; }

  .link-title { display: block; font-size: 13px; color: var(--text-primary); }
  .link-url   { display: block; font-size: 11px; color: var(--text-muted); }

  .link-del {
    position: absolute;
    top: 6px; right: 4px;
    font-size: 14px;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity var(--transition), color var(--transition);
    padding: 2px 4px;
  }

  .link-item:hover .link-del { opacity: 1; }
  .link-del:hover { color: var(--red); }

  .empty-links {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 40px 16px;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }

  .empty-links span { font-size: 36px; }

  /* Preview */
  .link-preview {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    background: var(--bg-base);
    padding: 24px;
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

  .yt-wrapper {
    width: 100%;
    max-width: 800px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .yt-wrapper h3 { font-size: 16px; color: var(--text-primary); }

  .yt-embed-container {
    position: relative;
    width: 100%;
    padding-top: 56.25%; /* 16:9 */
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0,0,0,0.5);
  }

  .yt-embed-container iframe {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border: none;
  }

  .external-link {
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    color: var(--text-secondary);
  }

  .external-link span { font-size: 52px; }
  .external-link h3 { color: var(--text-primary); font-size: 18px; }
  .url-text { font-size: 12px; color: var(--text-muted); word-break: break-all; max-width: 400px; }
  .hint { font-size: 12px; color: var(--text-muted); }

  .open-btn {
    margin-top: 8px;
    padding: 10px 24px;
    background: var(--accent);
    color: #fff;
    border-radius: var(--radius-md);
    font-size: 14px;
    font-weight: 500;
    transition: background var(--transition);
    display: inline-block;
  }

  .open-btn:hover { background: var(--accent-hover); }
</style>
