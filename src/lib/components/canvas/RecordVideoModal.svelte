<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy, tick } from "svelte";
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
  let videoChunks: Blob[] = [];
  let videoBlob: Blob | null = null;
  let previewUrl = "";
  let stream: MediaStream | null = null;

  let liveVideoEl: HTMLVideoElement;

  function fmt(s: number) {
    const m  = Math.floor(s / 60).toString().padStart(2, "0");
    const ss = (s % 60).toString().padStart(2, "0");
    return `${m}:${ss}`;
  }

  async function requestCamera(): Promise<boolean> {
    error = "";
    try {
      stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
      permState = "granted";
      return true;
    } catch (e) {
      const name = (e as DOMException).name;
      const msg  = (e as DOMException).message;
      console.error("getUserMedia (video) error:", name, msg);
      if (name === "NotAllowedError" || name === "PermissionDeniedError") {
        permState = "denied";
        error = msg || name;
      } else if (name === "NotFoundError") {
        error = $t.recordVideo.camNotFound;
      } else {
        error = $t.recordVideo.camError(name);
      }
      return false;
    }
  }

  async function retry() {
    await invoke("reset_camera_permission");
    permState = "unknown";
    await startRecording();
  }

  async function startRecording() {
    if (!(await requestCamera())) return;

    const mimeType = MediaRecorder.isTypeSupported("video/webm;codecs=vp8,opus")
      ? "video/webm;codecs=vp8,opus"
      : "video/webm";

    mediaRecorder = new MediaRecorder(stream!, { mimeType });
    videoChunks = [];

    mediaRecorder.ondataavailable = e => { if (e.data.size > 0) videoChunks.push(e.data); };
    mediaRecorder.onstop = () => {
      videoBlob = new Blob(videoChunks, { type: mimeType });
      previewUrl = URL.createObjectURL(videoBlob);
      if (liveVideoEl) { liveVideoEl.srcObject = null; }
      state = "stopped";
    };

    mediaRecorder.start(100);
    state = "recording";
    seconds = 0;
    ticker = setInterval(() => seconds++, 1000);
    await tick();
    if (liveVideoEl) {
      liveVideoEl.srcObject = stream;
      liveVideoEl.play().catch(() => {});
    }
  }

  function stopRecording() {
    if (ticker) { clearInterval(ticker); ticker = null; }
    mediaRecorder?.stop();
    stream?.getTracks().forEach(t => t.stop());
  }

  async function save() {
    if (!videoBlob) return;
    saving = true;
    error = "";
    try {
      const arrayBuffer = await videoBlob.arrayBuffer();
      const bytes = Array.from(new Uint8Array(arrayBuffer));
      const file = await invoke<{ stored_path: string; name: string; size: number }>(
        "save_video_bytes", { spaceId, bytes }
      );
      onSave(file.stored_path, file.name, file.size);
    } catch (e) {
      error = $t.recordVideo.saveError(e);
      saving = false;
    }
  }

  function discard() {
    if (previewUrl) URL.revokeObjectURL(previewUrl);
    videoBlob = null;
    previewUrl = "";
    state = "idle";
    seconds = 0;
  }

  onDestroy(() => {
    if (ticker) clearInterval(ticker);
    stream?.getTracks().forEach(t => t.stop());
    if (previewUrl) URL.revokeObjectURL(previewUrl);
  });
</script>

<div class="overlay" on:click={onClose} role="presentation"></div>

<div class="modal">
  <div class="modal-header">
    <span class="modal-title">{$t.recordVideo.title}</span>
    <button class="close-btn" on:click={onClose}>×</button>
  </div>

  <div class="modal-body">

    {#if permState === "denied"}
      <div class="perm-denied">
        <div class="perm-icon">📹</div>
        <p class="perm-title">{$t.recordVideo.camDenied}</p>
        <p class="perm-hint">{$t.recordVideo.permHint}</p>
        {#if error}<p class="err" style="font-size:11px;margin-top:4px">{error}</p>{/if}
      </div>

    {:else}
      <div class="video-wrap">
        {#if state === "recording"}
          <video bind:this={liveVideoEl} autoplay muted playsinline class="live-video"></video>
          <span class="rec-dot"></span>
          <span class="timer">{fmt(seconds)}</span>
        {:else if state === "stopped"}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video src={previewUrl} controls class="preview-video"></video>
        {:else}
          <!-- idle -->
          <video bind:this={liveVideoEl} autoplay muted playsinline class="live-video hidden"></video>
          <div class="idle-placeholder">
            <span class="idle-icon">📹</span>
            <span class="idle-hint">
              {permState === "checking" ? $t.recordVideo.checkingPerms : $t.recordVideo.pressRecord}
            </span>
          </div>
        {/if}
      </div>

      {#if error}<p class="err">{error}</p>{/if}
    {/if}

  </div>

  <div class="modal-footer">
    {#if permState === "denied"}
      <button class="btn-record" on:click={retry}>{$t.recordVideo.retry}</button>
      <button class="btn-close-perm" on:click={onClose}>{$t.recordVideo.close}</button>

    {:else if state === "idle"}
      <button class="btn-record" on:click={startRecording} disabled={permState === "checking"}>
        {$t.recordVideo.record}
      </button>

    {:else if state === "recording"}
      <button class="btn-stop" on:click={stopRecording}>{$t.recordVideo.stop}</button>

    {:else}
      <button class="btn-discard" on:click={discard}>{$t.recordVideo.discard}</button>
      <button class="btn-save" on:click={save} disabled={saving}>
        {saving ? $t.recordVideo.saving : $t.recordVideo.save}
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
    z-index: 201; width: 500px;
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

  .modal-body { padding: 16px; }

  /* ── Permission denied ── */
  .perm-denied {
    display: flex; flex-direction: column; align-items: center;
    gap: 10px; text-align: center; padding: 16px 0 8px;
  }
  .perm-icon { font-size: 36px; opacity: 0.5; line-height: 1; }
  .perm-title { font-size: 14px; font-weight: 700; color: var(--text-primary); margin: 0; }
  .perm-hint { font-size: 12px; color: var(--text-muted); line-height: 1.6; margin: 0; max-width: 320px; }

  /* ── Video area ── */
  .video-wrap {
    width: 100%; aspect-ratio: 16/9;
    background: #000; border-radius: 10px;
    overflow: hidden; position: relative;
    display: flex; align-items: center; justify-content: center;
  }
  .live-video, .preview-video {
    width: 100%; height: 100%; object-fit: cover; display: block;
  }
  .live-video.hidden { display: none; }
  .idle-placeholder {
    display: flex; flex-direction: column; align-items: center; gap: 8px;
    position: absolute; inset: 0; justify-content: center;
  }
  .idle-icon { font-size: 40px; opacity: 0.4; }
  .idle-hint { font-size: 12px; color: rgba(255,255,255,0.5); }
  .rec-dot {
    position: absolute; top: 10px; right: 12px;
    width: 10px; height: 10px; border-radius: 50%; background: #ef4444;
    animation: blink 1s infinite;
  }
  @keyframes blink { 0%,100% { opacity: 1; } 50% { opacity: 0.2; } }
  .timer {
    position: absolute; bottom: 10px; right: 14px;
    font-size: 12px; font-weight: 700; color: #fff;
    background: rgba(0,0,0,0.5); padding: 2px 6px; border-radius: 4px;
    font-variant-numeric: tabular-nums;
  }

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
