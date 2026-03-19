import { writable, derived } from "svelte/store";

export type PomodoroMode = "work" | "short" | "long";

const DURATIONS: Record<PomodoroMode, number> = {
  work: 25 * 60,
  short: 5 * 60,
  long: 15 * 60,
};

function createPomodoro() {
  const mode = writable<PomodoroMode>("work");
  const running = writable(false);
  const remaining = writable(DURATIONS.work);
  const sessions = writable(0);

  let interval: ReturnType<typeof setInterval> | null = null;

  function start() {
    if (interval) return;
    running.set(true);
    interval = setInterval(() => {
      remaining.update((r) => {
        if (r <= 1) {
          stop();
          sessions.update((s) => s + 1);
          return 0;
        }
        return r - 1;
      });
    }, 1000);
  }

  function stop() {
    if (interval) { clearInterval(interval); interval = null; }
    running.set(false);
  }

  function reset() {
    stop();
    mode.subscribe((m) => remaining.set(DURATIONS[m]))();
  }

  function setMode(m: PomodoroMode) {
    stop();
    mode.set(m);
    remaining.set(DURATIONS[m]);
  }

  return { mode, running, remaining, sessions, start, stop, reset, setMode };
}

export const pomodoro = createPomodoro();
