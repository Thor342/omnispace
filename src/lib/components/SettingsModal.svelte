<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { theme } from "../stores/theme";
  import appIcon from "../../assets/icon.png";

  export let onClose: () => void;

  let appVersion = "";
  onMount(async () => { appVersion = await getVersion(); });
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="modal-backdrop"
  role="button" tabindex="0"
  aria-label="Cerrar configuración"
  on:click={e => { if (e.target === e.currentTarget) onClose(); }}
  on:keydown={e => e.key === "Escape" && onClose()}
>
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="settings-modal" on:click|stopPropagation role="dialog" aria-modal="true">
    <div class="settings-modal-header">
      <div class="settings-modal-title-row">
        <img src={appIcon} alt="OmniSpace" class="settings-modal-icon" />
        <div>
          <p class="settings-modal-name">OmniSpace</p>
          <p class="settings-modal-version">Versión {appVersion}</p>
        </div>
      </div>
      <button class="settings-modal-close" on:click={onClose}>✕</button>
    </div>

    <div class="settings-modal-section">
      <p class="settings-section-label">Apariencia</p>
      <div class="theme-row">
        <button class="theme-option" class:active={$theme === "dark"} on:click={() => theme.set("dark")}>
          <span>🌙</span><span>Oscuro</span>
        </button>
        <button class="theme-option" class:active={$theme === "light"} on:click={() => theme.set("light")}>
          <span>☀️</span><span>Claro</span>
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.65);
    display: flex; align-items: center; justify-content: center;
    z-index: 200; backdrop-filter: blur(3px);
    pointer-events: auto;
  }
  .settings-modal {
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius-lg); padding: 24px;
    width: 340px; max-width: 92%;
    box-shadow: 0 24px 60px rgba(0,0,0,0.5);
    display: flex; flex-direction: column; gap: 20px;
  }
  .settings-modal-header { display: flex; align-items: flex-start; justify-content: space-between; }
  .settings-modal-title-row { display: flex; align-items: center; gap: 12px; }
  .settings-modal-icon { width: 40px; height: 40px; object-fit: contain; }
  .settings-modal-name { font-size: 16px; font-weight: 700; color: var(--text-primary); margin: 0; }
  .settings-modal-version { font-size: 11px; color: var(--text-muted); margin: 2px 0 0; }
  .settings-modal-close {
    font-size: 14px; color: var(--text-muted);
    padding: 4px 7px; border-radius: 6px; transition: all var(--transition);
  }
  .settings-modal-close:hover { background: var(--bg-hover); color: var(--text-primary); }
  .settings-modal-section { display: flex; flex-direction: column; gap: 10px; }
  .settings-section-label {
    font-size: 10px; font-weight: 700; letter-spacing: 1px;
    text-transform: uppercase; color: var(--text-muted); margin: 0;
  }
  .theme-row { display: flex; gap: 6px; }
  .theme-option {
    flex: 1; display: flex; flex-direction: column; align-items: center; gap: 4px;
    padding: 10px 6px; border-radius: var(--radius-sm); font-size: 12px;
    color: var(--text-secondary); background: var(--bg-overlay);
    border: 1px solid var(--border); transition: all var(--transition);
  }
  .theme-option span:first-child { font-size: 18px; }
  .theme-option:hover { border-color: var(--accent); }
  .theme-option.active { background: var(--accent-dim); border-color: var(--accent); color: var(--accent); font-weight: 600; }
</style>
