<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { t } from "../../stores/language";

  export let content: string;
  export let onContentChange: (c: string) => void;

  type Mode = "clock" | "timer" | "stopwatch";
  let mode: Mode = (() => {
    try { return (JSON.parse(content || "{}").mode as Mode) ?? "clock"; }
    catch { return "clock"; }
  })();

  function saveMode(m: Mode) { mode = m; onContentChange(JSON.stringify({ mode: m })); }

  // ── Clock ──────────────────────────────────────────────
  let now = new Date();
  $: clockHour12 = now.getHours() % 12 || 12;
  $: clockAmPm   = now.getHours() >= 12 ? "PM" : "AM";
  $: clockStr = `${pad(clockHour12)}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;
  $: dateStr  = $t.clockBlock.dateStr($t.clockBlock.days[now.getDay()], now.getDate(), $t.clockBlock.months[now.getMonth()], now.getFullYear());

  // ── Stopwatch ─────────────────────────────────────────
  let swRunning = false;
  let swMs = 0;
  let swBase = 0;
  let swStart = 0;
  let swInterval: ReturnType<typeof setInterval>;
  let laps: string[] = [];

  $: swTotalS = Math.floor(swMs / 1000);
  $: swM = Math.floor(swTotalS / 60);
  $: swS = swTotalS % 60;
  $: swCs = Math.floor((swMs % 1000) / 10);
  $: swStr = `${pad(swM)}:${pad(swS)}.${pad(swCs)}`;

  function swStart_() {
    swRunning = true;
    swStart = Date.now();
    swInterval = setInterval(() => { swMs = swBase + (Date.now() - swStart); }, 33);
  }
  function swPause() {
    swRunning = false;
    swBase = swMs;
    clearInterval(swInterval);
  }
  function swReset() {
    clearInterval(swInterval);
    swRunning = false; swMs = 0; swBase = 0; laps = [];
  }
  function swLap() {
    laps = [...laps, swStr];
  }

  // ── Timer ──────────────────────────────────────────────
  let timerInput = "05:00"; // mm:ss string
  let timerMs = 0;
  let timerTotalMs = 0;
  let timerRunning = false;
  let timerDone = false;
  let timerInterval: ReturnType<typeof setInterval>;

  function parseTimerInput(): number {
    const [m, s] = timerInput.split(":").map(Number);
    return ((m || 0) * 60 + (s || 0)) * 1000;
  }
  $: timerRemS = Math.ceil(timerMs / 1000);
  $: timerM = Math.floor(timerRemS / 60);
  $: timerS = timerRemS % 60;
  $: timerStr = `${pad(timerM)}:${pad(timerS)}`;
  $: timerProgress = timerTotalMs > 0 ? (timerMs / timerTotalMs) * 100 : 100;
  $: circumference = 2 * Math.PI * 42; // r=42 in SVG

  function timerStart() {
    if (timerMs <= 0) {
      timerTotalMs = parseTimerInput();
      timerMs = timerTotalMs;
    }
    if (timerMs <= 0) return;
    timerRunning = true;
    timerDone = false;
    const start = Date.now();
    const base = timerMs;
    clearInterval(timerInterval);
    timerInterval = setInterval(() => {
      timerMs = Math.max(0, base - (Date.now() - start));
      if (timerMs <= 0) {
        clearInterval(timerInterval);
        timerRunning = false;
        timerDone = true;
      }
    }, 100);
  }
  function timerPause() { clearInterval(timerInterval); timerRunning = false; }
  function timerReset() { clearInterval(timerInterval); timerRunning = false; timerMs = 0; timerTotalMs = 0; timerDone = false; }

  // ── Shared ─────────────────────────────────────────────
  function pad(n: number) { return String(Math.floor(n)).padStart(2, "0"); }

  let clockInterval: ReturnType<typeof setInterval>;
  onMount(() => { clockInterval = setInterval(() => { now = new Date(); }, 1000); });
  onDestroy(() => { clearInterval(clockInterval); clearInterval(swInterval); clearInterval(timerInterval); });
</script>

<div class="clock-block">

  <!-- ── Clock ── -->
  {#if mode === "clock"}
    <div class="face">
      <div class="big-time">{clockStr}<span class="ampm">{clockAmPm}</span></div>
      <div class="sub-date">{dateStr}</div>
    </div>

  <!-- ── Timer ── -->
  {:else if mode === "timer"}
    <div class="face">
      <div class="timer-arc">
        <svg viewBox="0 0 100 100">
          <circle class="arc-bg"   cx="50" cy="50" r="42" />
          <circle class="arc-fill" cx="50" cy="50" r="42"
            stroke-dasharray={circumference}
            stroke-dashoffset={circumference * (1 - timerProgress / 100)}
          />
        </svg>
        <div class="arc-num" class:done={timerDone}>
          {#if !timerRunning && timerMs <= 0 && !timerDone}
            <input class="bare-input" bind:value={timerInput} placeholder="mm:ss" />
            <button class="arc-play" on:click={timerStart}>▶</button>
          {:else}
            <span>{timerStr}</span>
            <div class="arc-btns">
              <button class="arc-play" on:click={() => timerRunning ? timerPause() : timerStart()}>
                {timerRunning ? "⏸" : "▶"}
              </button>
              <button class="arc-play arc-secondary" on:click={timerReset}>↺</button>
            </div>
          {/if}
        </div>
      </div>
      {#if timerDone}<div class="done-badge">{$t.clockBlock.timeUp}</div>{/if}
    </div>

  <!-- ── Stopwatch ── -->
  {:else}
    <div class="face">
      <div class="big-time mono">{swStr}</div>
      <div class="ctrl-row">
        {#if !swRunning}
          <button class="bare-btn accent" on:click={swStart_}>▶ {swMs > 0 ? $t.clockBlock.resume : $t.clockBlock.start}</button>
        {:else}
          <button class="bare-btn" on:click={swPause}>⏸ {$t.clockBlock.pause}</button>
        {/if}
        {#if swRunning}
          <button class="bare-btn" on:click={swLap}>◎ {$t.clockBlock.lap}</button>
        {/if}
        <button class="bare-btn" on:click={swReset}>↺ {$t.clockBlock.reset}</button>
      </div>
      {#if laps.length > 0}
        <div class="laps">
          {#each laps as lap, i}
            <div class="lap-row"><span class="lap-n">#{i + 1}</span><span>{lap}</span></div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <!-- ── Mode selector ── -->
  <div class="mode-bar">
    <button class="mode-opt" class:active={mode === "clock"}     on:click={() => saveMode("clock")}>{$t.clockBlock.clock}</button>
    <span class="mode-sep">·</span>
    <button class="mode-opt" class:active={mode === "stopwatch"} on:click={() => saveMode("stopwatch")}>{$t.clockBlock.stopwatch}</button>
    <span class="mode-sep">·</span>
    <button class="mode-opt" class:active={mode === "timer"}     on:click={() => saveMode("timer")}>{$t.clockBlock.timer}</button>
  </div>

</div>

<style>
  .clock-block {
    display: flex; flex-direction: column; flex: 1; overflow: hidden;
    background: transparent; container-type: size;
    align-items: center; justify-content: center; gap: 4px;
  }

  /* ── Face (main content area) ── */
  .face {
    display: flex; flex-direction: column;
    align-items: center;
    gap: 4px; padding: 2px 8px; overflow: hidden; min-height: 0;
  }

  /* ── Clock ── */
  .big-time {
    font-size: clamp(20px, min(13cqw, 20cqh), 10rem); font-weight: 400; letter-spacing: 2px;
    color: var(--text-primary); font-variant-numeric: tabular-nums;
    line-height: 1; white-space: nowrap;
  }
  .ampm {
    font-size: 0.35em; font-weight: 600; letter-spacing: 1px;
    color: var(--accent); vertical-align: super; margin-left: 3px;
  }
  .sub-date {
    font-size: clamp(9px, min(3cqw, 5cqh), 20px); color: var(--text-muted);
    text-align: center; letter-spacing: 0.2px; text-transform: capitalize;
  }

  /* ── Stopwatch ── */
  .mono { font-family: monospace; letter-spacing: 1px; }

  /* ── Timer arc ── */
  .timer-arc {
    position: relative; width: min(65cqw, 72cqh); aspect-ratio: 1; flex-shrink: 0;
    container-type: inline-size;
  }
  .timer-arc svg { width: 100%; height: 100%; transform: rotate(-90deg); display: block; }
  .arc-bg   { fill: none; stroke: var(--border); stroke-width: 5; }
  .arc-fill { fill: none; stroke: var(--accent); stroke-width: 5; stroke-linecap: round; transition: stroke-dashoffset 0.1s; }
  .arc-num {
    position: absolute; inset: 0; display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 3cqw;
    font-size: clamp(12px, 20cqw, 4rem); font-weight: 400;
    color: var(--text-primary); font-variant-numeric: tabular-nums; letter-spacing: 1px;
  }
  .arc-num.done { color: #f9a8d4; animation: pulse 0.6s ease infinite alternate; }
  .arc-play {
    font-size: clamp(10px, 14cqw, 2rem); color: var(--accent);
    background: var(--accent-dim); border: 1px solid var(--accent);
    border-radius: 50%; width: 1.8em; height: 1.8em;
    display: flex; align-items: center; justify-content: center;
    transition: background 0.15s, color 0.15s;
    line-height: 1;
  }
  .arc-play:hover { background: var(--accent); color: #fff; }
  .arc-btns { display: flex; gap: 6px; align-items: center; }
  .arc-secondary {
    background: var(--bg-overlay); color: var(--text-muted);
    border-color: var(--border);
  }
  .arc-secondary:hover { background: var(--bg-hover); color: var(--text-primary); }
  @keyframes pulse { to { opacity: 0.3; } }

  /* ── Bare input (timer set) ── */
  .bare-input {
    width: 70px; text-align: center; font-size: 15px; font-weight: 400;
    background: transparent; border: none; border-bottom: 1px solid var(--border);
    color: var(--text-primary); padding: 2px 4px; letter-spacing: 1px; outline: none;
    font-variant-numeric: tabular-nums;
  }
  .bare-input::placeholder { color: var(--text-muted); }
  .bare-input:focus { border-bottom-color: var(--accent); }

  /* ── Controls ── */
  .ctrl-row { display: flex; gap: 8px; flex-wrap: wrap; justify-content: center; }
  .bare-btn {
    padding: 3px 10px; border-radius: 20px; font-size: 10px; font-weight: 500;
    color: var(--text-secondary); background: var(--bg-overlay);
    border: 1px solid var(--border); transition: background 0.15s, color 0.15s;
    letter-spacing: 0.3px;
  }
  .bare-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .bare-btn.accent { background: var(--accent-dim); color: var(--accent); border-color: var(--accent); }
  .bare-btn.accent:hover { background: var(--accent); color: #fff; }

  /* ── Done badge ── */
  .done-badge {
    font-size: 12px; font-weight: 600; color: #f9a8d4;
    animation: pulse 0.6s ease infinite alternate; letter-spacing: 0.4px;
  }

  /* ── Laps ── */
  .laps {
    width: 100%; max-height: 70px; overflow-y: auto;
    border-top: 1px solid var(--border); scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }
  .lap-row {
    display: flex; justify-content: space-between; padding: 2px 8px;
    font-size: 10px; color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  .lap-n { color: var(--text-muted); }

  /* ── Mode selector ── */
  .mode-bar {
    display: flex; align-items: center; gap: 6px;
    justify-content: center; flex-shrink: 0;
    padding: 2px 0 4px; font-size: 10px;
  }
  .mode-opt {
    color: var(--text-muted); font-weight: 500; letter-spacing: 0.3px;
    padding: 1px 2px; border-bottom: 1.5px solid transparent;
    transition: color 0.15s, border-color 0.15s;
  }
  .mode-opt:hover { color: var(--text-secondary); }
  .mode-opt.active { color: var(--text-primary); font-weight: 700; border-bottom-color: var(--accent); }
  .mode-sep { color: var(--text-muted); opacity: 0.4; user-select: none; }
</style>
