<script lang="ts">
  import { fetchOgMeta } from "../../api";
  import type { LinkContent, LinkType } from "../../types";

  export let content: string;
  export let onContentChange: (c: string) => void;

  let data: LinkContent = JSON.parse(content || '{"url":"","title":"","link_type":"general"}');
  let editing = !data.url;
  let inputUrl = data.url;
  let inputTitle = data.title;
  let error = "";
  let loadingOg = false;

  // ── Embed detection ─────────────────────────────────────

  function detectType(url: string): LinkType {
    if (url.includes("youtube.com/watch") || url.includes("youtu.be/")) return "youtube";
    if (url.includes("figma.com/design") || url.includes("figma.com/file") || url.includes("figma.com/proto")) return "figma";
    if (url.includes("docs.google.com/presentation")) return "gslides";
    if (url.includes("docs.google.com/document")) return "gdocs";
    if (url.includes("miro.com/app/board")) return "miro";
    if (url.includes("loom.com/share")) return "loom";
    return "general";
  }

  function getEmbedUrl(url: string, type: LinkType): string {
    try {
      switch (type) {
        case "youtube": {
          const short = url.match(/youtu\.be\/([^?&]+)/);
          if (short) return `https://www.youtube.com/embed/${short[1]}`;
          const long  = url.match(/[?&]v=([^&]+)/);
          if (long)  return `https://www.youtube.com/embed/${long[1]}`;
          return "";
        }
        case "canva": {
          // https://www.canva.com/design/ID/slug/view → add ?embed
          const u = new URL(url);
          u.searchParams.set("embed", "");
          return u.toString().replace("embed=", "embed");
        }
        case "figma":
          return `https://www.figma.com/embed?embed_host=share&url=${encodeURIComponent(url)}`;
        case "gslides": {
          // Replace /edit, /pub, /preview with /embed
          return url.replace(/\/(edit|pub|preview|view)(.*)?$/, "/embed");
        }
        case "gdocs": {
          return url.replace(/\/(edit|preview)(.*)?$/, "/pub?embedded=true");
        }
        case "miro": {
          const id = url.match(/miro\.com\/app\/board\/([^/?]+)/)?.[1];
          return id ? `https://miro.com/app/live-embed/${id}/` : "";
        }
        case "loom": {
          return url.replace("loom.com/share/", "loom.com/embed/");
        }
        default: return "";
      }
    } catch { return ""; }
  }

  $: embedUrl = data.link_type !== "general" ? getEmbedUrl(data.url, data.link_type) : "";
  $: isEmbeddable = embedUrl.length > 0;

  // ── Platform labels & colors ────────────────────────────
  const PLATFORM: Record<string, { label: string; color: string; icon: string }> = {
    youtube:  { label: "YouTube",       color: "#ff0000", icon: "▶" },
    canva:    { label: "Canva",         color: "#7d2ae8", icon: "✦" },
    figma:    { label: "Figma",         color: "#f24e1e", icon: "◈" },
    gslides:  { label: "Google Slides", color: "#fbbc04", icon: "▣" },
    gdocs:    { label: "Google Docs",   color: "#4285f4", icon: "📄" },
    miro:     { label: "Miro",          color: "#ffdd00", icon: "◻" },
    loom:     { label: "Loom",          color: "#625DF5", icon: "⏺" },
    general:  { label: "",              color: "var(--accent)", icon: "🔗" },
  };
  $: platform = PLATFORM[data.link_type] ?? PLATFORM.general;

  // ── Save ───────────────────────────────────────────────
  function isValid(url: string) {
    try { new URL(url); return true; } catch { return false; }
  }

  async function save() {
    error = "";
    const url = inputUrl.trim();
    if (!url) { error = "Ingresa una URL"; return; }
    if (!isValid(url)) { error = "URL no válida"; return; }

    const type = detectType(url);
    const title = inputTitle.trim() || url;

    // Base data first (show immediately)
    data = { url, title, link_type: type };
    onContentChange(JSON.stringify(data));
    editing = false;

    // Fetch OG only for general links
    if (type === "general") {
      loadingOg = true;
      try {
        const og = await fetchOgMeta(url);
        data = {
          ...data,
          og_title: og.title || title,
          og_description: og.description,
          og_image: og.image,
          og_site: og.site_name,
        };
        onContentChange(JSON.stringify(data));
      } catch { /* OG fetch failed silently */ }
      finally { loadingOg = false; }
    }
  }

  function startEdit() {
    inputUrl = data.url;
    inputTitle = data.title;
    editing = true;
  }
</script>

<div class="link-block">
  {#if editing}
    <!-- ── Form ── -->
    <div class="link-form">
      <div class="form-hint">Pega un enlace de YouTube, Canva, Figma, Google Slides, Miro…</div>
      <input bind:value={inputTitle} placeholder="Título (opcional)" />
      <input
        bind:value={inputUrl}
        placeholder="https://…"
        on:keydown={e => e.key === "Enter" && save()}
      />
      {#if error}<p class="err">{error}</p>{/if}
      <div class="form-row">
        <button class="btn-save" on:click={save}>Guardar</button>
        {#if data.url}<button class="btn-cancel" on:click={() => editing = false}>Cancelar</button>{/if}
      </div>
    </div>

  {:else if isEmbeddable}
    <!-- ── Embed iframe ── -->
    <div class="embed-wrapper">
      <div class="embed-bar" style="--plat-color:{platform.color}">
        <span class="plat-badge" style="background:{platform.color}20;color:{platform.color}">
          {platform.icon} {platform.label}
        </span>
        <span class="embed-title truncate">{data.title}</span>
        <div class="embed-actions">
          <a href={data.url} target="_blank" rel="noopener" class="bar-btn" title="Abrir en nueva pestaña">↗</a>
          <button class="bar-btn" on:click={startEdit} title="Editar">✏</button>
        </div>
      </div>
      <div class="embed-frame">
        <iframe
          src={embedUrl}
          title={data.title}
          frameborder="0"
          allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; fullscreen"
          allowfullscreen
        />
      </div>
    </div>

  {:else}
    <!-- ── OG preview card ── -->
    <div class="og-card">
      {#if data.og_image}
        <div class="og-image-wrap">
          <img src={data.og_image} alt={data.og_title || data.title} class="og-image" />
        </div>
      {/if}
      <div class="og-body">
        {#if data.og_site || loadingOg}
          <span class="og-site">{loadingOg ? "Cargando vista previa…" : data.og_site}</span>
        {/if}
        <p class="og-title">{data.og_title || data.title}</p>
        {#if data.og_description}
          <p class="og-desc">{data.og_description}</p>
        {/if}
        <p class="og-url">{data.url}</p>
      </div>
      <div class="og-footer">
        <a href={data.url} target="_blank" rel="noopener" class="open-btn">Abrir ↗</a>
        <button class="edit-btn" on:click={startEdit}>✏ Editar</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .link-block { display: flex; flex-direction: column; flex: 1; overflow: hidden; }

  /* ── Form ── */
  .link-form {
    display: flex; flex-direction: column; gap: 10px;
    padding: 16px; flex: 1;
  }
  .form-hint {
    font-size: 11px; color: var(--text-muted);
    background: var(--bg-overlay); border-radius: 6px;
    padding: 6px 10px; line-height: 1.4;
  }
  .link-form input {
    padding: 9px 13px; font-size: 13px; width: 100%; box-sizing: border-box;
    background: var(--bg-overlay); border: 1.5px solid var(--border);
    border-radius: 9px; color: var(--text-primary); font-family: inherit;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .link-form input:focus {
    border-color: var(--accent); outline: none;
    box-shadow: 0 0 0 3px var(--accent-dim);
  }
  .link-form input::placeholder { color: var(--text-muted); }
  .err { color: #ef4444; font-size: 11px; margin-top: -4px; }
  .form-row { display: flex; gap: 8px; }
  .btn-save {
    flex: 1; padding: 9px; background: var(--accent); color: #fff;
    border-radius: 9px; font-size: 13px; font-weight: 600; transition: opacity 0.15s;
  }
  .btn-save:hover { opacity: 0.88; }
  .btn-cancel {
    flex: 1; padding: 9px; background: var(--bg-active);
    color: var(--text-secondary); border-radius: 9px; font-size: 13px;
    transition: background 0.15s;
  }
  .btn-cancel:hover { background: var(--bg-hover); }

  /* ── Embed ── */
  .embed-wrapper { display: flex; flex-direction: column; flex: 1; overflow: hidden; }
  .embed-bar {
    display: flex; align-items: center; gap: 6px; padding: 4px 8px;
    background: var(--bg-surface); border-bottom: 1px solid var(--border);
    flex-shrink: 0; min-width: 0;
  }
  .plat-badge {
    font-size: 10px; font-weight: 700; padding: 2px 7px; border-radius: 20px;
    white-space: nowrap; flex-shrink: 0; letter-spacing: 0.3px;
  }
  .embed-title { flex: 1; font-size: 11px; color: var(--text-muted); font-weight: 500; }
  .embed-actions { display: flex; gap: 2px; flex-shrink: 0; }
  .bar-btn {
    padding: 3px 7px; background: var(--bg-overlay);
    border: 1px solid var(--border); border-radius: 5px;
    font-size: 11px; color: var(--text-muted); cursor: pointer;
    transition: all 0.15s; text-decoration: none;
  }
  .bar-btn:hover { color: var(--text-primary); background: var(--bg-hover); }
  .embed-frame { flex: 1; overflow: hidden; }
  .embed-frame iframe { width: 100%; height: 100%; border: none; display: block; }

  /* ── OG Card ── */
  .og-card {
    display: flex; flex-direction: column; flex: 1; overflow: hidden;
  }
  .og-image-wrap {
    flex-shrink: 0; max-height: 55%; overflow: hidden;
    background: var(--bg-overlay);
  }
  .og-image { width: 100%; height: 100%; object-fit: cover; object-position: top; display: block; }
  .og-body {
    flex: 1; overflow: hidden; padding: 12px 14px 6px;
    display: flex; flex-direction: column; gap: 4px; min-height: 0;
  }
  .og-site { font-size: 10px; color: var(--accent); font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; }
  .og-title {
    font-size: 13px; font-weight: 700; color: var(--text-primary);
    line-height: 1.35; margin: 0;
    overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical;
  }
  .og-desc {
    font-size: 11px; color: var(--text-muted); line-height: 1.45; margin: 0;
    overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical;
  }
  .og-url {
    font-size: 10px; color: var(--text-muted); margin: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    opacity: 0.7;
  }
  .og-footer {
    display: flex; gap: 8px; padding: 8px 12px;
    border-top: 1px solid var(--border); flex-shrink: 0;
  }
  .open-btn {
    flex: 1; padding: 7px; background: var(--accent); color: #fff;
    border-radius: 8px; font-size: 12px; font-weight: 600;
    text-align: center; text-decoration: none; transition: opacity 0.15s;
  }
  .open-btn:hover { opacity: 0.88; }
  .edit-btn {
    padding: 7px 12px; background: var(--bg-active);
    border: 1px solid var(--border); border-radius: 8px;
    font-size: 12px; color: var(--text-secondary); transition: background 0.15s;
  }
  .edit-btn:hover { background: var(--bg-hover); }

  .truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
