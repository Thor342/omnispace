<script lang="ts">
  import { pomodoro } from "../../stores/pomodoro";
  import type { PomodoroMode } from "../../stores/pomodoro";

  const { mode, running, remaining, sessions, start, stop, reset, setMode } = pomodoro;

  const modes: { id: PomodoroMode; label: string }[] = [
    { id: "work",  label: "Foco" },
    { id: "short", label: "Descanso" },
    { id: "long",  label: "Largo" },
  ];

  function fmt(s: number): string {
    const m = Math.floor(s / 60).toString().padStart(2, "0");
    const sec = (s % 60).toString().padStart(2, "0");
    return `${m}:${sec}`;
  }

  $: pct = $mode === "work"
    ? (1 - $remaining / (25 * 60)) * 100
    : $mode === "short"
    ? (1 - $remaining / (5 * 60)) * 100
    : (1 - $remaining / (15 * 60)) * 100;
</script>

<div class="pomodoro">
  <!-- Mode tabs -->
  <div class="pom-modes">
    {#each modes as m}
      <button
        class="mode-btn"
        class:active={$mode === m.id}
        on:click={() => setMode(m.id)}
      >{m.label}</button>
    {/each}
  </div>

  <!-- Timer display -->
  <div class="timer-wrap">
    <svg class="timer-ring" viewBox="0 0 48 48">
      <circle cx="24" cy="24" r="20" fill="none" stroke="var(--bg-active)" stroke-width="3" />
      <circle
        cx="24" cy="24" r="20"
        fill="none"
        stroke="var(--accent)"
        stroke-width="3"
        stroke-dasharray="125.6"
        stroke-dashoffset={125.6 * (1 - pct / 100)}
        stroke-linecap="round"
        transform="rotate(-90 24 24)"
      />
    </svg>
    <span class="timer-text">{fmt($remaining)}</span>
  </div>

  <!-- Controls -->
  <div class="pom-controls">
    {#if $running}
      <button class="ctrl-btn pause" on:click={stop} title="Pausar">⏸</button>
    {:else}
      <button class="ctrl-btn play" on:click={start} title="Iniciar">▶</button>
    {/if}
    <button class="ctrl-btn reset" on:click={reset} title="Reiniciar">↺</button>
  </div>

  {#if $sessions > 0}
    <span class="sessions-badge" title="Sesiones completadas">🍅 {$sessions}</span>
  {/if}
</div>

<style>
  .pomodoro {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 6px 14px;
  }

  .pom-modes { display: flex; gap: 2px; }

  .mode-btn {
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    font-size: 11px;
    color: var(--text-muted);
    transition: all var(--transition);
  }

  .mode-btn:hover { color: var(--text-primary); }
  .mode-btn.active { background: var(--accent-dim); color: var(--accent); font-weight: 600; }

  .timer-wrap {
    position: relative;
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .timer-ring {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    transition: stroke-dashoffset 0.5s ease;
  }

  .timer-text {
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--text-primary);
    position: relative;
    z-index: 1;
  }

  .pom-controls { display: flex; gap: 4px; }

  .ctrl-btn {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    font-size: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background var(--transition);
  }

  .ctrl-btn.play  { background: var(--accent); color: #fff; }
  .ctrl-btn.pause { background: var(--yellow); color: #000; }
  .ctrl-btn.reset { background: var(--bg-active); color: var(--text-secondary); }

  .ctrl-btn:hover { opacity: 0.85; }

  .sessions-badge {
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
