<script lang="ts">
  import { createEventDispatcher } from "svelte";
  const dispatch = createEventDispatcher();

  export let title: string;
  export let countdown: number = 0;
  export let confirmLabel: string = "Sí, eliminar";
  export let shake: boolean = false;
  export let onConfirm: () => void;
  export let onCancel: () => void;
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="modal-backdrop"
  role="button" tabindex="0"
  aria-label="Cancelar"
  on:click={e => { if (e.target === e.currentTarget) onCancel(); }}
  on:keydown={e => e.key === "Escape" && onCancel()}
>
  <div
    class="del-modal"
    class:shake
    on:animationend={() => dispatch("shakeend")}
    on:click|stopPropagation
    role="dialog"
    aria-modal="true"
    aria-label="Confirmar eliminación"
  >
    <div class="del-icon">⚠️</div>
    <h3>{title}</h3>
    <div class="del-warning"><slot /></div>

    {#if countdown > 0}
      <div class="countdown">
        Lee el mensaje — podrás confirmar en <strong>{countdown}s</strong>
      </div>
      <div class="countdown-bar">
        <div class="countdown-fill" style="width:{((5 - countdown) / 5) * 100}%" />
      </div>
    {/if}

    <div class="del-actions">
      <button
        class="btn-danger"
        disabled={countdown > 0}
        on:click={onConfirm}
      >
        {countdown > 0 ? `Espera ${countdown}s…` : confirmLabel}
      </button>
      <button class="btn-ghost" on:click={onCancel}>Cancelar</button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.65);
    display: flex; align-items: center; justify-content: center;
    z-index: 300; backdrop-filter: blur(3px);
  }
  .del-modal {
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius-lg); padding: 28px;
    max-width: 420px; width: 92%;
    box-shadow: 0 24px 60px rgba(0,0,0,0.5);
    display: flex; flex-direction: column; gap: 14px;
  }
  .del-modal.shake { animation: modal-shake 0.35s ease; }
  @keyframes modal-shake {
    0%,100% { transform: translateX(0); }
    20%      { transform: translateX(-6px); }
    40%      { transform: translateX(6px); }
    60%      { transform: translateX(-4px); }
    80%      { transform: translateX(4px); }
  }
  .del-icon { font-size: 36px; text-align: center; }
  .del-modal h3 { font-size: 17px; font-weight: 700; color: var(--text-primary); text-align: center; }
  .del-warning { font-size: 13px; color: var(--text-secondary); text-align: center; line-height: 1.6; }
  :global(.del-warning strong) { color: var(--red); }
  .countdown {
    text-align: center; font-size: 12px; color: var(--text-muted);
    background: var(--bg-overlay); border-radius: var(--radius-sm); padding: 8px;
  }
  .countdown strong { color: var(--accent); }
  .countdown-bar { height: 4px; background: var(--bg-overlay); border-radius: 2px; overflow: hidden; }
  .countdown-fill { height: 100%; background: var(--accent); transition: width 1s linear; }
  .del-actions { display: flex; gap: 8px; }
  .btn-danger {
    flex: 1; padding: 10px; background: var(--red); color: #fff;
    border-radius: var(--radius-sm); font-size: 13px; font-weight: 600;
    transition: opacity var(--transition); outline: none;
  }
  .btn-danger:hover:not(:disabled) { opacity: 0.85; }
  .btn-danger:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-ghost {
    flex: 1; padding: 10px; background: var(--bg-active);
    color: var(--text-secondary); border-radius: var(--radius-sm);
    font-size: 13px; outline: none;
  }
  .btn-ghost:hover { background: var(--bg-hover); color: var(--text-primary); }
</style>
