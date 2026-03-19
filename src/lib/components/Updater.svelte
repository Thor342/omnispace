<script lang="ts">
  import { onMount } from "svelte";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";

  let show = false;
  let version = "";
  let notes = "";
  let installing = false;
  let progress = 0;
  let error = "";

  let updateObj: Awaited<ReturnType<typeof check>> | null = null;

  onMount(async () => {
    // Esperar 3 segundos para no bloquear el arranque
    await new Promise(r => setTimeout(r, 3000));
    try {
      updateObj = await check();
      if (updateObj?.available) {
        version = updateObj.version;
        notes   = updateObj.body ?? "";
        show    = true;
      }
    } catch {
      // Sin internet o sin release publicado — silencioso
    }
  });

  async function install() {
    if (!updateObj) return;
    installing = true;
    error = "";
    try {
      let downloaded = 0;
      let total = 0;
      await updateObj.downloadAndInstall(event => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          progress = total > 0 ? Math.round((downloaded / total) * 100) : 0;
        }
      });
      await relaunch();
    } catch (e) {
      error = String(e);
      installing = false;
    }
  }

  function dismiss() {
    show = false;
  }
</script>

{#if show}
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="update-backdrop" on:click|self={dismiss} on:keydown={e => e.key === "Escape" && dismiss()}>
    <div class="update-card" role="dialog" aria-modal="true" aria-label="Actualización disponible">
      <div class="update-header">
        <span class="update-icon">🚀</span>
        <div class="update-titles">
          <p class="update-label">Actualización disponible</p>
          <p class="update-version">OmniSpace {version}</p>
        </div>
        {#if !installing}
          <button class="update-dismiss" on:click={dismiss} title="Recordar después">✕</button>
        {/if}
      </div>

      {#if notes}
        <div class="update-notes">
          <p class="notes-title">Novedades</p>
          <p class="notes-body">{notes}</p>
        </div>
      {/if}

      {#if error}
        <p class="update-error">{error}</p>
      {/if}

      {#if installing}
        <div class="update-progress-wrap">
          <div class="update-progress-bar" style="width:{progress}%"></div>
        </div>
        <p class="update-status">
          {progress < 100 ? `Descargando… ${progress}%` : "Instalando…"}
        </p>
      {:else}
        <div class="update-actions">
          <button class="btn-later" on:click={dismiss}>Después</button>
          <button class="btn-install" on:click={install}>Instalar ahora</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .update-backdrop {
    position: fixed; inset: 0; z-index: 9999;
    display: flex; align-items: flex-end; justify-content: flex-end;
    padding: 24px; pointer-events: none;
  }
  .update-card {
    pointer-events: all;
    width: 320px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    box-shadow: 0 8px 40px rgba(0,0,0,0.35), 0 2px 8px rgba(0,0,0,0.15);
    overflow: hidden;
    animation: slide-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  @keyframes slide-in {
    from { transform: translateY(20px); opacity: 0; }
    to   { transform: translateY(0);    opacity: 1; }
  }

  .update-header {
    display: flex; align-items: center; gap: 10px;
    padding: 16px 16px 12px;
  }
  .update-icon { font-size: 26px; flex-shrink: 0; line-height: 1; }
  .update-titles { flex: 1; min-width: 0; }
  .update-label {
    font-size: 10px; font-weight: 700; letter-spacing: 0.6px;
    text-transform: uppercase; color: var(--accent); margin: 0 0 2px;
  }
  .update-version {
    font-size: 14px; font-weight: 700; color: var(--text-primary); margin: 0;
  }
  .update-dismiss {
    font-size: 14px; color: var(--text-muted); padding: 4px 6px;
    border-radius: 6px; flex-shrink: 0; transition: all 0.15s;
  }
  .update-dismiss:hover { background: var(--bg-hover); color: var(--text-primary); }

  .update-notes {
    padding: 0 16px 12px;
    border-top: 1px solid var(--border);
    padding-top: 10px;
  }
  .notes-title {
    font-size: 10px; font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.5px; color: var(--text-muted); margin: 0 0 4px;
  }
  .notes-body {
    font-size: 12px; color: var(--text-secondary);
    line-height: 1.5; margin: 0;
    max-height: 80px; overflow-y: auto;
    white-space: pre-wrap;
  }

  .update-error {
    padding: 0 16px 10px;
    font-size: 11px; color: #ef4444;
  }

  .update-actions {
    display: flex; gap: 8px; padding: 0 16px 16px;
  }
  .btn-later {
    flex: 1; padding: 8px; font-size: 12px; font-weight: 600;
    border-radius: 8px; background: var(--bg-overlay);
    color: var(--text-secondary); border: 1px solid var(--border);
    transition: all 0.15s; cursor: pointer;
  }
  .btn-later:hover { background: var(--bg-hover); color: var(--text-primary); }
  .btn-install {
    flex: 2; padding: 8px; font-size: 12px; font-weight: 700;
    border-radius: 8px; background: var(--accent); color: #fff;
    border: none; transition: all 0.15s; cursor: pointer;
  }
  .btn-install:hover { opacity: 0.88; }

  .update-progress-wrap {
    margin: 0 16px 8px;
    height: 6px; background: var(--bg-overlay);
    border-radius: 3px; overflow: hidden;
  }
  .update-progress-bar {
    height: 100%; background: var(--accent);
    border-radius: 3px; transition: width 0.2s;
  }
  .update-status {
    padding: 0 16px 16px;
    font-size: 11px; color: var(--text-muted); text-align: center; margin: 0;
  }
</style>
