<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { t } from "../../stores/language";

  export let spaceId: string;
  export let onSave: (storedPath: string, name: string, size: number) => void;
  export let onClose: () => void;

  type RecordState = "idle" | "recording" | "stopped";
  type PermState  = "unknown" | "checking" | "granted" | "denied" | "unavailable";

  let state: RecordState = "idle";
  let permState: PermState = "unknown";
  let error = "";
  let seconds = 0;
  let ticker: ReturnType<typeof setInterval> | null = null;
  let saving = false;

  let mediaRecorder: MediaRecorder | null = null;
  let audioChunks: Blob[] = [];
  let audioBlob: Blob | null = null;
  let previewUrl = "";
  let stream: MediaStream | null = null;

  // Visualizer
  let canvasEl: HTMLCanvasElement;
  let analyser: AnalyserNode | null = null;
  let animFrame: number | null = null;

  function fmt(s: number) {
    const m  = Math.floor(s / 60).toString().padStart(2, "0");
    const ss = (s % 60).toString().padStart(2, "0");
    return `${m}:${ss}`;
  }

  // ── Permission check ──────────────────────────────────
  // Check permission state on mount so UI is accurate before user clicks Grabar
  onMount(async () => {
    permState = "checking";
    try {
      const result = await navigator.permissions.query({ name: "microphone" as PermissionName });
      permState = result.state === "granted" ? "granted"
                : result.state === "denied"  ? "denied"
                : "unknown"; // "prompt" → unknown, will ask on click

      // Listen for changes (user toggles permission in OS/browser settings)
      result.onchange = () => {
        permState = result.state === "granted" ? "granted"
                  : result.state === "denied"  ? "denied"
                  : "unknown";
        if (permState === "granted") error = "";
      };
    } catch {
      // Permissions API not available (unlikely in Tauri but handle it)
      permState = "unknown";
    }
  });

  // ── Open OS microphone settings ───────────────────────
  async function openMicSettings() {
    try { await invoke("open_mic_settings"); } catch {}
  }

  // ── Request microphone ────────────────────────────────
  async function requestMic(): Promise<boolean> {
    error = "";
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      permState = "granted";
      return true;
    } catch (e) {
      const name = (e as DOMException).name;
      if (name === "NotAllowedError" || name === "PermissionDeniedError") {
        permState = "denied";
        error = "";  // shown via perm UI, not error text
      } else if (name === "NotFoundError") {
        error = $t.recordAudio.micNotFound;
      } else {
        error = $t.recordAudio.micError(name);
      }
      return false;
    }
  }

  async function startRecording() {
    if (!(await requestMic())) return;

    // Waveform analyser
    const audioCtx = new AudioContext();
    const src = audioCtx.createMediaStreamSource(stream!);
    analyser = audioCtx.createAnalyser();
    analyser.fftSize = 256;
    src.connect(analyser);
    drawWave();

    const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
      ? "audio/webm;codecs=opus"
      : "audio/webm";

    mediaRecorder = new MediaRecorder(stream!, { mimeType });
    audioChunks = [];

    mediaRecorder.ondataavailable = e => { if (e.data.size > 0) audioChunks.push(e.data); };
    mediaRecorder.onstop = () => {
      audioBlob = new Blob(audioChunks, { type: mimeType });
      previewUrl = URL.createObjectURL(audioBlob);
      stopWave();
    };

    mediaRecorder.start(100);
    state = "recording";
    seconds = 0;
    ticker = setInterval(() => seconds++, 1000);
  }

  function stopRecording() {
    if (ticker) { clearInterval(ticker); ticker = null; }
    mediaRecorder?.stop();
    stream?.getTracks().forEach(t => t.stop());
    state = "stopped";
  }

  function drawWave() {
    if (!analyser || !canvasEl) return;
    const buf = new Uint8Array(analyser.frequencyBinCount);
    const ctx = canvasEl.getContext("2d")!;
    const W = canvasEl.width, H = canvasEl.height;

    function loop() {
      animFrame = requestAnimationFrame(loop);
      analyser!.getByteTimeDomainData(buf);
      ctx.clearRect(0, 0, W, H);
      ctx.strokeStyle = "#6366f1";
      ctx.lineWidth = 2;
      ctx.beginPath();
      const step = W / buf.length;
      buf.forEach((v, i) => {
        const y = (v / 128) * (H / 2);
        i === 0 ? ctx.moveTo(0, y) : ctx.lineTo(i * step, y);
      });
      ctx.stroke();
    }
    loop();
  }

  function stopWave() {
    if (animFrame !== null) { cancelAnimationFrame(animFrame); animFrame = null; }
    if (canvasEl) {
      const ctx = canvasEl.getContext("2d")!;
      ctx.clearRect(0, 0, canvasEl.width, canvasEl.height);
    }
  }

  async function save() {
    if (!audioBlob) return;
    saving = true;
    error = "";
    try {
      const arrayBuffer = await audioBlob.arrayBuffer();
      const bytes = Array.from(new Uint8Array(arrayBuffer));
      const file = await invoke<{ stored_path: string; name: string; size: number }>(
        "save_audio_bytes", { spaceId, bytes }
      );
      onSave(file.stored_path, file.name, file.size);
    } catch (e) {
      error = $t.recordAudio.saveError(e);
      saving = false;
    }
  }

  function discard() {
    if (previewUrl) URL.revokeObjectURL(previewUrl);
    audioBlob = null;
    previewUrl = "";
    state = "idle";
    seconds = 0;
  }

  onDestroy(() => {
    if (ticker) clearInterval(ticker);
    stopWave();
    stream?.getTracks().forEach(t => t.stop());
    if (previewUrl) URL.revokeObjectURL(previewUrl);
  });
</script>

<div class="overlay" on:click={onClose} role="presentation" />

<div class="modal">
  <div class="modal-header">
    <span class="modal-title">{$t.recordAudio.title}</span>
    <button class="close-btn" on:click={onClose}>×</button>
  </div>

  <div class="modal-body">

    {#if permState === "denied"}
      <!-- ── Permiso denegado ── -->
      <div class="perm-denied">
        <div class="perm-icon">🎙</div>
        <p class="perm-title">{$t.recordAudio.micDenied}</p>
        <p class="perm-hint">{$t.recordAudio.permHint}</p>
        <button class="btn-settings" on:click={openMicSettings}>
          {$t.recordAudio.openMicSettings}
        </button>
      </div>

    {:else}
      <!-- ── Waveform / preview ── -->
      <div class="wave-wrap">
        {#if state === "recording"}
          <canvas bind:this={canvasEl} width="340" height="60" class="wave-canvas"></canvas>
          <span class="rec-dot"></span>
          <span class="timer">{fmt(seconds)}</span>
        {:else if state === "stopped"}
          <div class="preview-wrap">
            <audio src={previewUrl} controls class="preview-audio"></audio>
          </div>
        {:else}
          <canvas bind:this={canvasEl} width="340" height="60" class="wave-canvas idle"></canvas>
          <span class="idle-hint">
            {permState === "checking" ? $t.recordAudio.checkingPerms : $t.recordAudio.pressRecord}
          </span>
        {/if}
      </div>

      {#if error}<p class="err">{error}</p>{/if}
    {/if}

  </div>

  <div class="modal-footer">
    {#if permState === "denied"}
      <button class="btn-close-perm" on:click={onClose}>{$t.recordAudio.close}</button>

    {:else if state === "idle"}
      <button class="btn-record" on:click={startRecording} disabled={permState === "checking"}>
        {$t.recordAudio.record}
      </button>

    {:else if state === "recording"}
      <button class="btn-stop" on:click={stopRecording}>{$t.recordAudio.stop}</button>

    {:else}
      <button class="btn-discard" on:click={discard}>{$t.recordAudio.discard}</button>
      <button class="btn-save" on:click={save} disabled={saving}>
        {saving ? $t.recordAudio.saving : $t.recordAudio.save}
      </button>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.5);
    z-index: 200; backdrop-filter: blur(2px);
  }
  .modal {
    position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
    z-index: 201; width: 400px;
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius-md); box-shadow: 0 16px 48px rgba(0,0,0,0.4);
    display: flex; flex-direction: column; overflow: hidden;
  }
  .modal-header {
    display: flex; align-items: center; gap: 8px;
    padding: 14px 16px; border-bottom: 1px solid var(--border); flex-shrink: 0;
  }
  .modal-title { flex: 1; font-size: 14px; font-weight: 700; color: var(--text-primary); }
  .close-btn {
    font-size: 18px; color: var(--text-muted); padding: 2px 6px;
    border-radius: 4px; line-height: 1;
    transition: color 0.15s, background 0.15s;
  }
  .close-btn:hover { color: var(--text-primary); background: var(--bg-hover); }

  .modal-body { padding: 20px 16px; }

  /* ── Permission denied screen ── */
  .perm-denied {
    display: flex; flex-direction: column; align-items: center;
    gap: 10px; text-align: center; padding: 8px 0 4px;
  }
  .perm-icon { font-size: 36px; opacity: 0.5; line-height: 1; }
  .perm-title { font-size: 14px; font-weight: 700; color: var(--text-primary); margin: 0; }
  .perm-hint {
    font-size: 12px; color: var(--text-muted); line-height: 1.6;
    margin: 0; max-width: 300px;
  }
  .perm-hint strong { color: var(--text-secondary); }
  .perm-actions {
    display: flex; gap: 8px; margin-top: 4px; flex-wrap: wrap; justify-content: center;
  }
  .btn-settings {
    padding: 9px 16px; background: var(--bg-active);
    border: 1px solid var(--border); border-radius: 9px;
    font-size: 13px; color: var(--text-secondary); font-weight: 600;
    transition: background 0.15s;
  }
  .btn-settings:hover { background: var(--bg-hover); }

  /* ── Waveform ── */
  .wave-wrap {
    height: 80px; display: flex; align-items: center; justify-content: center;
    background: var(--bg-overlay); border-radius: 10px; gap: 10px;
    position: relative; overflow: hidden;
  }
  .wave-canvas { width: 100%; height: 60px; }
  .wave-canvas.idle { opacity: 0.2; }
  .idle-hint { position: absolute; font-size: 12px; color: var(--text-muted); }
  .rec-dot {
    position: absolute; top: 10px; right: 12px;
    width: 8px; height: 8px; border-radius: 50%; background: #ef4444;
    animation: blink 1s infinite;
  }
  @keyframes blink { 0%,100% { opacity: 1; } 50% { opacity: 0.2; } }
  .timer {
    position: absolute; bottom: 8px; right: 12px;
    font-size: 11px; font-weight: 700; color: #ef4444; font-variant-numeric: tabular-nums;
  }
  .preview-wrap { width: 100%; padding: 0 8px; }
  .preview-audio { width: 100%; }

  .err { color: #ef4444; font-size: 11px; margin-top: 8px; }

  /* ── Footer ── */
  .modal-footer {
    display: flex; gap: 8px; padding: 12px 16px;
    border-top: 1px solid var(--border); flex-shrink: 0;
  }
  .btn-record {
    flex: 1; padding: 10px; background: #ef4444; color: #fff;
    border-radius: 9px; font-size: 13px; font-weight: 700;
    transition: opacity 0.15s;
  }
  .btn-record:hover:not(:disabled) { opacity: 0.88; }
  .btn-record:disabled { opacity: 0.45; cursor: not-allowed; }
  .btn-stop {
    flex: 1; padding: 10px; background: var(--bg-active);
    border: 1px solid var(--border); border-radius: 9px;
    font-size: 13px; font-weight: 700; color: var(--text-primary);
    transition: background 0.15s;
  }
  .btn-stop:hover { background: var(--bg-hover); }
  .btn-discard {
    flex: 1; padding: 10px; background: var(--bg-active);
    border: 1px solid var(--border); border-radius: 9px;
    font-size: 13px; color: var(--text-secondary);
    transition: background 0.15s;
  }
  .btn-discard:hover { background: var(--bg-hover); }
  .btn-save {
    flex: 1; padding: 10px; background: var(--accent); color: #fff;
    border-radius: 9px; font-size: 13px; font-weight: 700;
    transition: opacity 0.15s;
  }
  .btn-save:hover:not(:disabled) { opacity: 0.88; }
  .btn-save:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-close-perm {
    flex: 1; padding: 10px; background: var(--bg-active);
    border: 1px solid var(--border); border-radius: 9px;
    font-size: 13px; color: var(--text-secondary);
    transition: background 0.15s;
  }
  .btn-close-perm:hover { background: var(--bg-hover); }
</style>
