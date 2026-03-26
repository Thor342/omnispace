<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../../stores/language";

  export let content: string;
  export let onContentChange: (c: string) => void;
  export let zoom: number = 1;

  // ── Date constants (i18n) ──────────────────────────────
  $: MONTH_NAMES = $t.calendarBlock.months;
  $: DAY_NAMES   = $t.calendarBlock.dayLetters;
  $: DAY_FULL    = $t.calendarBlock.dayShort;
  $: DAY_LONG    = $t.calendarBlock.dayLong;

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

  // ── Scale factor: counteracts canvas zoom so content is
  //    always readable regardless of zoom level.
  //    s = 1/zoom → at 20% zoom s=5, at 100% zoom s=1.
  $: s = 1 / Math.max(zoom, 0.05);

  // ── View mode uses SCREEN-SPACE dimensions ─────────────
  // so "month" only shows when the block is large enough on screen.
  $: sw = cw * zoom;
  $: sh = ch * zoom;
  $: viewMode = (() => {
    if (sw >= 230 && sh >= 210) return "month";
    if (sw >= 180 && sh >= 170) return "week";
    return "day";
  })();

  // ── Day view font sizes (screen-space targets) ─────────
  $: dayNumPx   = Math.max(28, Math.min(sw * 0.42, sh * 0.48)) * s;
  $: dayNamePx  = Math.max(10, Math.min(sw * 0.10, sh * 0.11)) * s;
  $: dayMonthPx = Math.max(9,  Math.min(sw * 0.07, sh * 0.08)) * s;

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

<div class="cal-block" bind:this={containerEl} style="--s:{s}">

  <!-- Navigation bar (hidden in day view) -->
  {#if viewMode !== "day"}
  <div class="cal-nav" on:pointerdown|stopPropagation>
    <div class="nav-center">
      <button class="nav-btn" on:click={prev}>‹</button>
      <span class="nav-label">{navLabel}</span>
      <button class="nav-btn" on:click={next}>›</button>
      <button class="nav-btn today-btn" on:click={goToday} title={$t.calendarBlock.today}>●</button>
    </div>
  </div>
  {/if}

  <!-- ── MONTH view ───────────────────────────────────── -->
  {#if viewMode === "month"}
    <div class="month-full">
      <div class="days-grid-full">
        {#each DAY_NAMES as d}
          <span class="day-hdr">{d}</span>
        {/each}
        {#each { length: monthData.start } as _}<span></span>{/each}
        {#each { length: monthData.days } as _, i}
          {@const day = i + 1}
          {@const key = dateKey(monthData.year, monthData.month, day)}
          <button
            class="day day-lg"
            class:today={isToday(monthData.year, monthData.month, day)}
            class:past={isPast(monthData.year, monthData.month, day)}
            class:marked={markedSet.has(key)}
            on:click={() => toggleMark(monthData.year, monthData.month, day)}
            title={markedSet.has(key) ? $t.calendarBlock.removeReminder : $t.calendarBlock.markReminder}
          >
            {day}
            {#if markedSet.has(key)}<span class="mark-dot"></span>{/if}
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
          title={markedSet.has(wkey) ? $t.calendarBlock.removeReminder : $t.calendarBlock.markReminder}
        >
          <span class="week-day-name">{DAY_FULL[date.getDay()]}</span>
          <span class="week-day-num" class:today={isT}>{date.getDate()}</span>
          {#if markedSet.has(wkey)}<span class="mark-dot"></span>{/if}
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
  /* --s is set via inline style as 1/zoom — scales all px values so content
     stays readable at any canvas zoom level. At zoom=1 s=1 (no change).
     At zoom=0.2 s=5, so 13px font becomes 65px canvas → 13px on screen. */

  .cal-block {
    display: flex; flex-direction: column; flex: 1; overflow: hidden;
    padding: calc(4px * var(--s, 1)) calc(6px * var(--s, 1)) calc(6px * var(--s, 1));
    min-width: 0; min-height: 0;
    background: transparent;
  }

  /* ── Nav bar ── */
  .cal-nav {
    display: flex; align-items: center; justify-content: center;
    flex-shrink: 0; margin-bottom: calc(8px * var(--s, 1));
    background: var(--bg-overlay);
    border-radius: calc(8px * var(--s, 1));
    padding: calc(2px * var(--s, 1));
    position: relative;
  }
  .nav-center {
    display: flex; align-items: center; gap: calc(2px * var(--s, 1));
    position: relative; z-index: 11;
  }
  .nav-label {
    font-size: calc(11px * var(--s, 1)); font-weight: 700;
    color: var(--text-primary); letter-spacing: 0.4px;
    white-space: nowrap;
  }
  .nav-btn {
    font-size: calc(14px * var(--s, 1)); color: var(--text-muted);
    padding: calc(3px * var(--s, 1)) calc(8px * var(--s, 1));
    border-radius: calc(6px * var(--s, 1)); line-height: 1;
    transition: color 0.15s, background 0.15s;
  }
  .nav-btn:hover { color: var(--accent); background: var(--accent-dim); }
  .today-btn { z-index: 11; }

  /* ── Full month view ── */
  .month-full { flex: 1; overflow: auto; scrollbar-width: none; }
  .month-full::-webkit-scrollbar { display: none; }
  .days-grid-full {
    display: grid; grid-template-columns: repeat(7, 1fr);
    gap: calc(3px * var(--s, 1)); text-align: center;
    padding: calc(2px * var(--s, 1));
  }
  .day-hdr {
    font-size: calc(8px * var(--s, 1)); color: var(--text-muted); font-weight: 700;
    padding: calc(1px * var(--s, 1)) 0; letter-spacing: 0.3px;
  }
  .day {
    font-size: calc(9px * var(--s, 1)); color: var(--text-secondary);
    padding: calc(1px * var(--s, 1)) 0; border-radius: 50%;
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
    background: rgba(251,191,36,0.15); color: var(--text-primary);
    box-shadow: inset 0 0 0 calc(1.5px * var(--s, 1)) rgba(251,191,36,0.5);
  }
  .day.marked.today { background: var(--accent); box-shadow: 0 1px 4px rgba(99,102,241,0.35); }
  .day-lg {
    font-size: calc(13px * var(--s, 1));
    padding: calc(2px * var(--s, 1)) calc(4px * var(--s, 1)) calc(6px * var(--s, 1));
    border-radius: calc(8px * var(--s, 1)); aspect-ratio: unset;
    min-height: calc(34px * var(--s, 1)); justify-content: flex-start;
    padding-top: calc(6px * var(--s, 1));
  }

  /* ── Marked dot ── */
  .mark-dot {
    display: block;
    width: calc(5px * var(--s, 1)); height: calc(5px * var(--s, 1));
    border-radius: 50%; background: #f59e0b; flex-shrink: 0;
    position: absolute; bottom: calc(4px * var(--s, 1));
    left: 50%; transform: translateX(-50%);
  }
  .day.today .mark-dot { background: rgba(255,255,255,0.85); }

  /* ── Week view ── */
  .week-grid {
    flex: 1; display: flex; gap: calc(5px * var(--s, 1)); align-items: stretch;
  }
  .week-day {
    flex: 1; display: flex; flex-direction: column; align-items: center;
    justify-content: center; gap: calc(4px * var(--s, 1));
    background: var(--bg-overlay);
    border-radius: calc(10px * var(--s, 1));
    padding: calc(6px * var(--s, 1)) calc(2px * var(--s, 1));
    transition: background 0.15s;
    border: calc(1px * var(--s, 1)) solid var(--border);
    cursor: pointer; position: relative;
  }
  .week-day:hover { background: var(--bg-hover); }
  .week-today { background: var(--accent-dim); border-color: var(--accent); }
  .week-marked { border-color: #f59e0b; background: rgba(251,191,36,0.08); }
  .week-marked.week-today { border-color: var(--accent); }
  .week-day-name {
    font-size: calc(9px * var(--s, 1)); font-weight: 700; color: var(--text-muted);
    text-transform: uppercase; letter-spacing: 0.5px;
  }
  .week-day-num {
    font-size: calc(16px * var(--s, 1)); font-weight: 700; color: var(--text-secondary);
    width: calc(30px * var(--s, 1)); height: calc(30px * var(--s, 1));
    border-radius: 50%;
    display: flex; align-items: center; justify-content: center;
    transition: background 0.15s, color 0.15s;
  }
  .week-day-num.today {
    background: var(--accent); color: #fff;
    box-shadow: 0 2px 8px rgba(99,102,241,0.3);
  }
  .week-day .mark-dot { bottom: calc(5px * var(--s, 1)); }

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
