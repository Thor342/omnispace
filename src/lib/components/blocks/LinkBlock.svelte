<script lang="ts">
  import { fetchOgMeta } from "../../api";
  import type { LinkContent, LinkType } from "../../types";

  export let content: string;
  export let onContentChange: (c: string) => void;

  let data: LinkContent = JSON.parse(content || '{"url":"","title":"","link_type":"general"}');

  // Auto-fix: if saved as "general" but URL matches a known embeddable service, upgrade it
  if (data.url && data.link_type === "general") {
    const corrected = detectType(data.url);
    if (corrected !== "general") {
      data = { ...data, link_type: corrected, og_title: undefined, og_description: undefined, og_image: undefined, og_site: undefined };
      onContentChange(JSON.stringify(data));
    }
  }

  let editing = !data.url;
  let inputUrl = data.url;
  let inputTitle = data.title;
  let error = "";
  let loadingOg = false;

  // ── Embed detection ─────────────────────────────────────

  function detectType(url: string): LinkType {
    if (url.includes("youtube.com/watch") || url.includes("youtu.be/")) return "youtube";
    if (url.includes("canva.com/design/")) return "canva";
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
          // Strip query params, replace /edit with /view, append ?embed
          const u = new URL(url);
          const cleanPath = u.pathname.replace(/\/(edit|view|present)(\/.*)?$/, "/view");
          return `https://www.canva.com${cleanPath}?embed`;
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

  // ── Helpers ────────────────────────────────────────────
  function isValid(url: string) {
    try { new URL(url); return true; } catch { return false; }
  }

  function domainOf(url: string): string {
    try { return new URL(url).hostname.replace(/^www\./, ""); } catch { return url; }
  }

  // Bad fallback values that should be ignored when displaying
  const BAD_TITLES = ["error", "403 forbidden", "access denied", "just a moment", "attention required"];
  function isBadTitle(t: string) { return BAD_TITLES.some(b => t.trim().toLowerCase() === b); }

  $: displayTitle = (data.og_title && !isBadTitle(data.og_title))
    ? data.og_title
    : (data.title && !isBadTitle(data.title) && data.title !== data.url)
      ? data.title
      : domainOf(data.url);

  $: displaySite = data.og_site || domainOf(data.url).split(".")[0].toUpperCase();

  // ── Save ───────────────────────────────────────────────
  async function save() {
    error = "";
    const url = inputUrl.trim();
    if (!url) { error = "Ingresa una URL"; return; }
    if (!isValid(url)) { error = "URL no válida"; return; }

    const type = detectType(url);
    const title = inputTitle.trim() || domainOf(url);

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
          og_title: og.title || undefined,
          og_description: og.description || undefined,
          og_image: og.image || undefined,
          og_site: og.site_name || undefined,
        };
        onContentChange(JSON.stringify(data));
      } catch { /* OG fetch failed — fallback to domain display */ }
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
      {#if data.link_type === "youtube"}
        <div class="form-hint">Pega el enlace del video de YouTube</div>
        <input
          bind:value={inputUrl}
          placeholder="https://www.youtube.com/watch?v=… o youtu.be/…"
          on:keydown={e => e.key === "Enter" && save()}
        />
      {:else if data.link_type === "canva"}
        <div class="form-hint">Pega el enlace de tu diseño de Canva</div>
        <input
          bind:value={inputUrl}
          placeholder="https://www.canva.com/design/…"
          on:keydown={e => e.key === "Enter" && save()}
        />
      {:else}
        <input bind:value={inputTitle} placeholder="Nombre del enlace" />
        <input
          bind:value={inputUrl}
          placeholder="https://…"
          on:keydown={e => e.key === "Enter" && save()}
        />
      {/if}
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
    <!-- ── Simple link card ── -->
    <div class="simple-card">
      <div class="simple-icon">{platform.icon}</div>
      <div class="simple-info">
        <span class="simple-site">{loadingOg ? "…" : displaySite}</span>
        <p class="simple-title">{displayTitle}</p>
      </div>
      <div class="simple-actions">
        <a href={data.url} target="_blank" rel="noopener" class="open-btn">Abrir ↗</a>
        <button class="edit-btn" on:click={startEdit}>✏</button>
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

  /* ── Simple card ── */
  .simple-card {
    display: flex; align-items: center; gap: 10px;
    padding: 12px 14px; flex: 1;
  }
  .simple-icon { font-size: 22px; flex-shrink: 0; }
  .simple-info { flex: 1; min-width: 0; }
  .simple-site { font-size: 10px; color: var(--accent); font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; }
  .simple-title {
    font-size: 13px; font-weight: 600; color: var(--text-primary); margin: 2px 0 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .simple-actions { display: flex; gap: 6px; flex-shrink: 0; }
  .open-btn {
    padding: 6px 12px; background: var(--accent); color: #fff;
    border-radius: 8px; font-size: 12px; font-weight: 600;
    text-decoration: none; transition: opacity 0.15s; white-space: nowrap;
  }
  .open-btn:hover { opacity: 0.88; }
  .edit-btn {
    padding: 6px 10px; background: var(--bg-active);
    border: 1px solid var(--border); border-radius: 8px;
    font-size: 12px; color: var(--text-secondary); transition: background 0.15s;
  }
  .edit-btn:hover { background: var(--bg-hover); }

  .truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
