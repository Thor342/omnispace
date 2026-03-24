<script lang="ts">
  import { onMount } from "svelte";

  export let content: string;
  export let onContentChange: (c: string) => void;

  // ── Date constants ─────────────────────────────────────
  const MONTH_NAMES = ["Enero","Febrero","Marzo","Abril","Mayo","Junio",
                       "Julio","Agosto","Septiembre","Octubre","Noviembre","Diciembre"];
const DAY_NAMES   = ["D","L","M","X","J","V","S"];
  const DAY_FULL    = ["Dom","Lun","Mar","Mié","Jue","Vie","Sáb"];
  const DAY_LONG    = ["Domingo","Lunes","Martes","Miércoles","Jueves","Viernes","Sábado"];

  const now = new Date();

  // ── Marked days (persisted in content) ─────────────────
  let parsed: { markedDays?: string[] } = {};
  try { parsed = JSON.parse(content || "{}"); } catch { parsed = {}; }
  let markedDays: string[] = parsed.markedDays ?? [];

  function saveMarked() {
    onContentChange(JSON.stringify({ markedDays }));
  }

  function dateKey(y: number, m: number, d: number): string {
    return `${y}-${String(m + 1).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
  }

  $: markedSet = new Set(markedDays);

  function isMarked(y: number, m: number, d: number): boolean {
    return markedSet.has(dateKey(y, m, d));
  }

  function toggleMark(y: number, m: number, d: number) {
    const key = dateKey(y, m, d);
    if (markedSet.has(key)) {
      markedDays = markedDays.filter(k => k !== key);
    } else {
      markedDays = [...markedDays, key];
    }
    saveMarked();
  }

  // ── Navigation state ───────────────────────────────────
  let navYear  = now.getFullYear();
  let navMonth = now.getMonth(); // 0-11

  // ── Container size tracking ────────────────────────────
  let containerEl: HTMLDivElement;
  let cw = 400;
  let ch = 300;

  onMount(() => {
    const ro = new ResizeObserver(entries => {
      for (const e of entries) {
        cw = e.contentRect.width;
        ch = e.contentRect.height;
      }
    });
    ro.observe(containerEl);
    return () => ro.disconnect();
  });

  // ── Day view font sizes ────────────────────────────────
  $: dayNumPx   = Math.max(28, Math.min(cw * 0.42, ch * 0.48));
  $: dayNamePx  = Math.max(10, Math.min(cw * 0.10, ch * 0.11));
  $: dayMonthPx = Math.max(9,  Math.min(cw * 0.07, ch * 0.08));

  // ── View mode: month → week → day (max = month) ────────
  $: viewMode = (() => {
    if (cw >= 230 && ch >= 210) return "month";
    if (cw >= 180 && ch >= 170) return "week";
    return "day";
  })();

  // ── Helpers ────────────────────────────────────────────
  function monthDays(y: number, m: number) { return new Date(y, m + 1, 0).getDate(); }
  function monthStart(y: number, m: number) { return new Date(y, m, 1).getDay(); }

  function isToday(y: number, m: number, d: number) {
    return y === now.getFullYear() && m === now.getMonth() && d === now.getDate();
  }
  function isPast(y: number, m: number, d: number) {
    const t = new Date(y, m, d); const n = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    return t < n;
  }

  // ── Month view ─────────────────────────────────────────
  $: monthData = {
    year: navYear, month: navMonth,
    start: monthStart(navYear, navMonth),
    days:  monthDays(navYear, navMonth),
  };

  // ── Week view: current week of navMonth/navYear ─────────
  $: weekData = (() => {
    const base = (navYear === now.getFullYear() && navMonth === now.getMonth())
      ? now
      : new Date(navYear, navMonth, 1);
    const day = base.getDay();
    const monday = new Date(base);
    monday.setDate(base.getDate() - ((day + 6) % 7));
    return Array.from({ length: 7 }, (_, i) => {
      const d = new Date(monday);
      d.setDate(monday.getDate() + i);
      return d;
    });
  })();

  // ── Navigation ─────────────────────────────────────────
  function prev() {
    navMonth--;
    if (navMonth < 0) { navMonth = 11; navYear--; }
  }

  function next() {
    navMonth++;
    if (navMonth > 11) { navMonth = 0; navYear++; }
  }

  function goToday() { navYear = now.getFullYear(); navMonth = now.getMonth(); }

  $: navLabel = `${MONTH_NAMES[navMonth]} ${navYear}`;
</script>

<div class="cal-block" bind:this={containerEl}>

  <!-- Navigation bar (hidden in day view) -->
  {#if viewMode !== "day"}
  <div class="cal-nav">
    <button class="nav-btn" on:click={prev}>‹</button>
    <span class="nav-label">{navLabel}</span>
    <button class="nav-btn" on:click={goToday} title="Hoy">●</button>
    <button class="nav-btn" on:click={next}>›</button>
  </div>
  {/if}

  <!-- ── MONTH view ───────────────────────────────────── -->
  {#if viewMode === "month"}
    <div class="month-full">
      <div class="days-grid-full">
        {#each DAY_NAMES as d}
          <span class="day-hdr">{d}</span>
        {/each}
        {#each { length: monthData.start } as _}<span />{/each}
        {#each { length: monthData.days } as _, i}
          {@const day = i + 1}
          {@const key = dateKey(monthData.year, monthData.month, day)}
          <button
            class="day day-lg"
            class:today={isToday(monthData.year, monthData.month, day)}
            class:past={isPast(monthData.year, monthData.month, day)}
            class:marked={markedSet.has(key)}
            on:click={() => toggleMark(monthData.year, monthData.month, day)}
            title={markedSet.has(key) ? "Quitar recordatorio" : "Marcar como recordatorio"}
          >
            {day}
            {#if markedSet.has(key)}<span class="mark-dot" />{/if}
          </button>
        {/each}
      </div>
    </div>

  <!-- ── WEEK view ────────────────────────────────────── -->
  {:else if viewMode === "week"}
    <div class="week-grid">
      {#each weekData as date}
        {@const isT = isToday(date.getFullYear(), date.getMonth(), date.getDate())}
        {@const wkey = dateKey(date.getFullYear(), date.getMonth(), date.getDate())}
        <button
          class="week-day"
          class:week-today={isT}
          class:week-marked={markedSet.has(wkey)}
          on:click={() => toggleMark(date.getFullYear(), date.getMonth(), date.getDate())}
          title={markedSet.has(wkey) ? "Quitar recordatorio" : "Marcar como recordatorio"}
        >
          <span class="week-day-name">{DAY_FULL[date.getDay()]}</span>
          <span class="week-day-num" class:today={isT}>{date.getDate()}</span>
          {#if markedSet.has(wkey)}<span class="mark-dot" />{/if}
        </button>
      {/each}
    </div>

  <!-- ── DAY view ─────────────────────────────────────── -->
  {:else}
    <div class="day-view">
      <div class="day-view-num" style="font-size:{dayNumPx}px">{now.getDate()}</div>
      <div class="day-view-name" style="font-size:{dayNamePx}px">{DAY_LONG[now.getDay()]}</div>
      <div class="day-view-month" style="font-size:{dayMonthPx}px">{MONTH_NAMES[now.getMonth()]} {now.getFullYear()}</div>
    </div>
  {/if}

</div>

<style>
  .cal-block {
    display: flex; flex-direction: column; flex: 1; overflow: hidden;
    padding: 4px 6px 6px;
    min-width: 0; min-height: 0;
    background: transparent;
    container-type: size;
  }

  /* ── Nav bar ── */
  .cal-nav {
    display: flex; align-items: center; gap: 2px;
    flex-shrink: 0; margin-bottom: 8px;
    background: var(--bg-overlay); border-radius: 8px; padding: 2px;
  }
  .nav-label {
    flex: 1; text-align: center; font-size: 11px; font-weight: 700;
    color: var(--text-primary); letter-spacing: 0.4px;
  }
  .nav-btn {
    font-size: 14px; color: var(--text-muted); padding: 3px 8px;
    border-radius: 6px; line-height: 1;
    transition: color 0.15s, background 0.15s;
  }
  .nav-btn:hover { color: var(--accent); background: var(--accent-dim); }

  /* ── Full month view ── */
  .month-full { flex: 1; overflow: auto; scrollbar-width: none; }
  .month-full::-webkit-scrollbar { display: none; }
  .days-grid-full {
    display: grid; grid-template-columns: repeat(7, 1fr);
    gap: 3px; text-align: center; padding: 2px;
  }
  .day-hdr {
    font-size: 8px; color: var(--text-muted); font-weight: 700;
    padding: 1px 0; letter-spacing: 0.3px;
  }
  .day {
    font-size: 9px; color: var(--text-secondary);
    padding: 1px 0; border-radius: 50%;
    aspect-ratio: 1; display: flex; align-items: center; justify-content: center;
    transition: background 0.1s, color 0.1s;
    cursor: pointer; position: relative; flex-direction: column; gap: 0;
  }
  .day:hover { background: var(--bg-hover); color: var(--text-primary); }
  .day.today {
    background: var(--accent); color: #fff; font-weight: 800;
    box-shadow: 0 1px 4px rgba(99,102,241,0.35);
  }
  .day.past { opacity: 0.45; }
  .day.marked {
    background: rgba(251,191,36,0.15);
    color: var(--text-primary);
    box-shadow: inset 0 0 0 1.5px rgba(251,191,36,0.5);
  }
  .day.marked.today { background: var(--accent); box-shadow: 0 1px 4px rgba(99,102,241,0.35); }
  .day-lg {
    font-size: 13px; padding: 2px 4px 6px;
    border-radius: 8px; aspect-ratio: unset;
    min-height: 34px; justify-content: flex-start; padding-top: 6px;
  }

  /* ── Marked dot ── */
  .mark-dot {
    display: block; width: 5px; height: 5px; border-radius: 50%;
    background: #f59e0b; flex-shrink: 0;
    position: absolute; bottom: 4px; left: 50%; transform: translateX(-50%);
  }
  .day.today .mark-dot { background: rgba(255,255,255,0.85); }

  /* ── Week view ── */
  .week-grid {
    flex: 1; display: flex; gap: 5px; align-items: stretch;
  }
  .week-day {
    flex: 1; display: flex; flex-direction: column; align-items: center;
    justify-content: center; gap: 4px;
    background: var(--bg-overlay); border-radius: 10px; padding: 6px 2px;
    transition: background 0.15s; border: 1px solid var(--border);
    cursor: pointer; position: relative;
  }
  .week-day:hover { background: var(--bg-hover); }
  .week-today {
    background: var(--accent-dim); border-color: var(--accent);
  }
  .week-marked {
    border-color: #f59e0b;
    background: rgba(251,191,36,0.08);
  }
  .week-marked.week-today { border-color: var(--accent); }
  .week-day-name {
    font-size: 9px; font-weight: 700; color: var(--text-muted);
    text-transform: uppercase; letter-spacing: 0.5px;
  }
  .week-day-num {
    font-size: 16px; font-weight: 700; color: var(--text-secondary);
    width: 30px; height: 30px; border-radius: 50%;
    display: flex; align-items: center; justify-content: center;
    transition: background 0.15s, color 0.15s;
  }
  .week-day-num.today {
    background: var(--accent); color: #fff;
    box-shadow: 0 2px 8px rgba(99,102,241,0.3);
  }
  .week-day .mark-dot {
    bottom: 5px;
  }

  /* ── Day view ── */
  .day-view {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 0; padding: 0; overflow: hidden;
  }
  .day-view-num {
    font-weight: 800; color: var(--accent);
    line-height: 1; letter-spacing: -1px;
    text-shadow: 0 2px 12px rgba(99,102,241,0.2);
  }
  .day-view-name {
    font-weight: 700; color: var(--text-primary);
    text-transform: uppercase; letter-spacing: 1.5px;
  }
  .day-view-month { color: var(--text-muted); text-transform: capitalize; }
</style>
