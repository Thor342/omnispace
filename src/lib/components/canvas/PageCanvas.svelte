<script lang="ts">
  import { onMount, tick } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { t } from "../../stores/language";
  import { invoke } from "@tauri-apps/api/core";
  import {
    getBlocks, createBlock, updateBlockPosition, updateBlockContent,
    updateBlockZindex, deleteBlock, restoreBlock, getStrokes, saveStrokes, importFile
  } from "../../api";
  import type { Block, BlockType, AppFile, StrokePath } from "../../types";
  import BlockContainer from "../blocks/Block.svelte";
  import CanvasShape from "./CanvasShape.svelte";
  import CanvasToolbar from "./CanvasToolbar.svelte";
  import RecordAudioModal from "./RecordAudioModal.svelte";
  import RecordVideoModal from "./RecordVideoModal.svelte";
  import { CW, CH, renderStroke, redrawAll as _redrawAll, eraseAt as _eraseAt, getCanvasPos } from "./canvasDrawing";
  import { type SnapResult, findSnapTarget, getPortCoords, isConnectorType } from "./canvasSnap";
  import { type HistoryEntry, pushHistory as _pushHistory } from "./canvasHistory";

  export let pageId: string;
  export let spaceId: string;

  // ── State ─────────────────────────────────────────────
  let blocks: Block[] = [];
  let strokes: StrokePath[] = [];
  let maxZ = 0;
  let selectedBlockId: string | null = null;
  let showRecordModal = false;
  let showVideoModal = false;

  // Draw
  let drawMode = false;
  let eraseMode = false;
  let shapeMode = false;
  let shapeType = "";

  // Shape drag-to-create
  let shapeDragging = false;
  let shapeStartX = 0, shapeStartY = 0;
  let shapePreview: { x: number; y: number; w: number; h: number } | null = null;
  let shapeFill = "#6366f1";
  let shapeStroke = "#1e1e2e";
  let shapeStrokeWidth = 2;
  // Connector (line/arrow) creation tracking
  let shapeEndX = 0, shapeEndY = 0;
  let createSnapStart: SnapResult = null;
  let createSnapEnd: SnapResult = null;
  let lineStyle: "solid" | "dashed" | "dotted" = "solid";
  let penColor = "#6366f1";
  let penWidth = 3;
  let penOpacity = 100;
  let eraserSize = 24; // canvas units

  // Eraser cursor indicator (screen space)
  let eraserVisible = false;
  let eraserScreenX = 0, eraserScreenY = 0;
  $: eraserScreenR = eraserSize * zoom;

  // Zoom
  let zoom = 1.0;

  // Pan state
  let isPanning = false;
  let panStartX = 0, panStartY = 0, panScrollX = 0, panScrollY = 0;

  let selectMode = false;
  let selectedBlockIds = new Set<string>();

  // Rubber-band selection
  let bandActive = false;
  let bandSX = 0, bandSY = 0;
  let bandRect: { x: number; y: number; w: number; h: number } | null = null;

  // Multi-block drag
  let multiDragging = false;
  let multiDragOriginX = 0, multiDragOriginY = 0;
  let multiDragOrigPos = new Map<string, { x: number; y: number }>();

  // Multi-touch pinch tracking (viewport-level, for background touches)
  let activePointers = new Map<number, { x: number; y: number }>();
  let pinchStartDist = 0;
  let pinchStartZoom = 1.0;

  // Global pinch tracking — captures ALL pointers including those on blocks
  let gPtrs = new Map<number, { x: number; y: number }>();
  let gPinching = false;
  let gPinchDist = 0;
  let gPinchZoom = 1.0;
  let gPinchCX = 0, gPinchCY = 0;
  let gPinchMidX = 0, gPinchMidY = 0; // midpoint tracking for two-finger pan
  // Pending scroll correction — applied after Svelte re-renders the zoomed canvas
  let pendingScroll: { x: number; y: number } | null = null;

  function applyPendingScroll() {
    if (!pendingScroll || !viewportEl) return;
    viewportEl.scrollLeft = pendingScroll.x;
    viewportEl.scrollTop  = pendingScroll.y;
    pendingScroll = null;
    redrawAll();
  }

  function onGlobalPointerDown(e: PointerEvent) {
    gPtrs.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (gPtrs.size === 2) {
      const pts = [...gPtrs.values()];
      gPinchDist = Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y);
      gPinchZoom = zoom;
      gPinchCX = (pts[0].x + pts[1].x) / 2;
      gPinchCY = (pts[0].y + pts[1].y) / 2;
      gPinchMidX = gPinchCX; gPinchMidY = gPinchCY;
      gPinching = true;
      isPanning = false;
    }
  }

  function onGlobalPointerMove(e: PointerEvent) {
    if (!gPtrs.has(e.pointerId)) return;
    gPtrs.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (!gPinching || gPtrs.size < 2 || gPinchDist === 0) return;

    // Stop propagation so blocks don't drag during pinch
    e.stopPropagation();

    const pts = [...gPtrs.values()];
    const dist = Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y);
    const newZoom = clampZoom(gPinchZoom * (dist / gPinchDist));

    const midX = (pts[0].x + pts[1].x) / 2;
    const midY = (pts[0].y + pts[1].y) / 2;
    const panDX = gPinchMidX - midX;
    const panDY = gPinchMidY - midY;
    gPinchMidX = midX; gPinchMidY = midY;

    const rect = viewportEl.getBoundingClientRect();
    const mx = gPinchCX - rect.left;
    const my = gPinchCY - rect.top;
    const prevZoom = zoom;

    if (newZoom !== zoom) {
      pendingScroll = {
        x: (viewportEl.scrollLeft + mx) / prevZoom * newZoom - mx + panDX,
        y: (viewportEl.scrollTop  + my) / prevZoom * newZoom - my + panDY,
      };
      zoom = newZoom;
      tick().then(applyPendingScroll);
    } else if (panDX !== 0 || panDY !== 0) {
      // Two-finger pan (no zoom change)
      viewportEl.scrollLeft += panDX;
      viewportEl.scrollTop  += panDY;
    }
  }

  function onGlobalPointerUp(e: PointerEvent) {
    gPtrs.delete(e.pointerId);
    if (gPtrs.size < 2) { gPinching = false; gPinchDist = 0; }
  }

  // Canvas
  let canvasEl: HTMLCanvasElement;
  let markerCanvasEl: HTMLCanvasElement;
  let viewportEl: HTMLDivElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let markerCtx: CanvasRenderingContext2D | null = null;
  let drawing = false;
  let erasePushedHistory = false; // push undo only once per erase gesture
  let currentStroke: { color: string; width: number; points: [number,number][]; layer: "top" | "base" } | null = null;

  // ── Undo history ──────────────────────────────────────
  let historyStack: HistoryEntry[] = [];
  $: canUndo = historyStack.length > 0;

  function pushHistory(entry: HistoryEntry) {
    historyStack = _pushHistory(historyStack, entry);
  }

  async function undo() {
    if (!historyStack.length) return;
    const entry = historyStack[historyStack.length - 1];
    historyStack = historyStack.slice(0, -1);
    if (entry.type === "strokes") {
      strokes = entry.before;
      await saveStrokes(pageId, JSON.stringify(strokes));
      redrawAll();
    } else if (entry.type === "block_deleted") {
      await restoreBlock(entry.block);
      blocks = [...blocks, entry.block];
    } else if (entry.type === "block_added") {
      await deleteBlock(entry.blockId);
      blocks = blocks.filter(b => b.id !== entry.blockId);
    } else if (entry.type === "block_moved") {
      const b = blocks.find(b => b.id === entry.blockId);
      if (b) {
        await updateBlockPosition(entry.blockId, entry.x, entry.y, entry.w, entry.h);
        blocks = blocks.map(b =>
          b.id === entry.blockId ? { ...b, x: entry.x, y: entry.y, width: entry.w, height: entry.h } : b
        );
      }
    }
  }

  // ── Load ──────────────────────────────────────────────
  $: if (pageId) load();

  async function load() {
    blocks = []; strokes = []; historyStack = [];
    [blocks] = await Promise.all([getBlocks(pageId)]);
    maxZ = blocks.reduce((m, b) => Math.max(m, b.z_index), 0);
    const raw = await getStrokes(pageId);
    strokes = JSON.parse(raw || "[]");
    await new Promise(r => setTimeout(r, 0));
    ctx = canvasEl?.getContext("2d") ?? null;
    markerCtx = markerCanvasEl?.getContext("2d") ?? null;
    redrawAll();
  }

  // ── Helpers ───────────────────────────────────────────
  function clampZoom(z: number) {
    return Math.round(Math.min(5, Math.max(0.05, z)) * 100) / 100;
  }

  // Canvas pixel dimensions — scale with zoom for crisp rendering, capped at GPU safe limit
  $: canvasPxW = Math.min(8192, Math.round(CW * zoom));
  $: canvasPxH = Math.min(8192, Math.round(CH * zoom));

  function isCanvasBg(el: EventTarget | null): boolean {
    if (!(el instanceof HTMLElement)) return false;
    return (
      el === viewportEl || el === canvasEl ||
      el.classList.contains("canvas-scaler") ||
      el.classList.contains("canvas-content") ||
      el.classList.contains("drawing-canvas")
    );
  }

  // ── Wheel ─────────────────────────────────────────────
  // Registered on window (capture phase) so we catch events even when the
  // pointer is over a block that calls stopPropagation.
  // • ctrlKey=true  → touchpad PINCH or Ctrl+scroll → zoom
  // • ctrlKey=false → two-finger scroll or mouse wheel over canvas → pan
  function onWheel(e: WheelEvent) {
    if (!viewportEl) return;
    const overCanvas = viewportEl.contains(e.target as Node);
    if (!e.ctrlKey && !overCanvas) return; // non-pinch outside viewport: ignore

    // Si el cursor está sobre un elemento scrollable interno (ej. lista de tareas),
    // dejar que el scroll ocurra naturalmente en ese elemento.
    if (!e.ctrlKey) {
      const scrollable = (e.target as Element).closest?.('.task-list');
      if (scrollable && scrollable.scrollHeight > scrollable.clientHeight) return;
    }

    e.preventDefault();
    // Normalize: deltaMode 1 = lines (~16px), 0 = pixels
    const factor = e.deltaMode === 1 ? 16 : 1;
    const dx = e.deltaX * factor;
    const dy = e.deltaY * factor;

    if (e.ctrlKey) {
      // Pinch (touchpad) or Ctrl+scroll → zoom keeping cursor point fixed
      const rect = viewportEl.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      const prevZoom = zoom;
      const capped = Math.sign(dy) * Math.min(Math.abs(dy), 80);
      const newZoom = clampZoom(prevZoom * (1 - capped * 0.005));
      pendingScroll = {
        x: (viewportEl.scrollLeft + mx) / prevZoom * newZoom - mx,
        y: (viewportEl.scrollTop  + my) / prevZoom * newZoom - my,
      };
      zoom = newZoom;
      tick().then(applyPendingScroll);
    } else {
      // Two-finger scroll → pan
      viewportEl.scrollLeft += dx;
      viewportEl.scrollTop  += dy;
    }
  }

  // ── Pointer events: pan + pinch ───────────────────────
  async function onViewportPointerDown(e: PointerEvent) {
    activePointers.set(e.pointerId, { x: e.clientX, y: e.clientY });

    if (activePointers.size >= 2) {
      // Two fingers → pinch zoom (cancel any current drawing/pan)
      const pts = [...activePointers.values()];
      pinchStartDist = Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y);
      pinchStartZoom = zoom;
      if (drawing) { drawing = false; currentStroke = null; redrawAll(); }
      isPanning = false;
      return;
    }

    // Single finger / mouse
    if (drawMode) return; // canvas handles drawing

    // Connector mode: intercept BEFORE isCanvasBg so clicks ON shapes also start connections
    if (shapeMode && isConnectorType(shapeType)) {
      (document.activeElement as HTMLElement)?.blur();
      e.preventDefault();
      selectedBlockId = null;
      const [cx, cy] = pointerToCanvas(e);
      shapeDragging = true;
      shapeStartX = cx; shapeStartY = cy;
      shapePreview = { x: cx, y: cy, w: 0, h: 0 };
      shapeEndX = cx; shapeEndY = cy;
      const snap = findSnapTarget(cx, cy, blocks);
      createSnapStart = snap;
      if (snap) { shapeStartX = snap.x; shapeStartY = snap.y; }
      return;
    }

    if (!isCanvasBg(e.target)) return; // block handles itself

    // Shape drag-to-create: start (non-connector shapes only on canvas bg)
    if (shapeMode) {
      if (!shapeType) return; // no type selected yet — wait for user to pick one
      (document.activeElement as HTMLElement)?.blur();
      e.preventDefault();
      selectedBlockId = null; // deselect previous shape when starting to draw a new one
      const [cx, cy] = pointerToCanvas(e);
      shapeDragging = true;
      shapeStartX = cx;
      shapeStartY = cy;
      shapePreview = { x: cx, y: cy, w: 0, h: 0 };
      shapeEndX = cx; shapeEndY = cy;
      return;
    }

    // Deselect block when clicking empty canvas
    selectedBlockId = null;

    // Select mode: rubber-band selection
    if (selectMode) {
      e.preventDefault();
      selectedBlockIds = new Set();
      const [cx, cy] = pointerToCanvas(e);
      bandActive = true;
      bandSX = cx; bandSY = cy;
      bandRect = { x: cx, y: cy, w: 0, h: 0 };
      return;
    }

    // Auto-pan on empty canvas (hand mode)
    // Blur any focused text editor first so it can commit before preventDefault blocks focus change
    (document.activeElement as HTMLElement)?.blur();
    e.preventDefault();
    isPanning = true;
    panStartX = e.clientX; panStartY = e.clientY;
    panScrollX = viewportEl.scrollLeft;
    panScrollY = viewportEl.scrollTop;
  }

  function onViewportPointerMove(e: PointerEvent) {
    if (activePointers.has(e.pointerId)) {
      activePointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    }

    if (activePointers.size >= 2 && pinchStartDist > 0) {
      const pts = [...activePointers.values()];
      const dist = Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y);
      zoom = clampZoom(pinchStartZoom * (dist / pinchStartDist));
      return;
    }

    // Shape preview while dragging to create
    if (shapeDragging && shapePreview) {
      const [cx, cy] = pointerToCanvas(e);
      if (isConnectorType(shapeType)) {
        const snap = findSnapTarget(cx, cy, blocks);
        createSnapEnd = snap;
        shapeEndX = snap ? snap.x : cx;
        shapeEndY = snap ? snap.y : cy;
        shapePreview = {
          x: Math.min(shapeStartX, shapeEndX), y: Math.min(shapeStartY, shapeEndY),
          w: Math.abs(shapeEndX - shapeStartX), h: Math.abs(shapeEndY - shapeStartY),
        };
      } else {
        shapePreview = {
          x: Math.min(shapeStartX, cx), y: Math.min(shapeStartY, cy),
          w: Math.abs(cx - shapeStartX), h: Math.abs(cy - shapeStartY),
        };
      }
      return;
    }

    if (isPanning && !gPinching) {
      viewportEl.scrollLeft = panScrollX - (e.clientX - panStartX);
      viewportEl.scrollTop  = panScrollY - (e.clientY - panStartY);
    }

    // Rubber-band rect update
    if (bandActive) {
      const [cx, cy] = pointerToCanvas(e);
      bandRect = {
        x: Math.min(bandSX, cx), y: Math.min(bandSY, cy),
        w: Math.abs(cx - bandSX),  h: Math.abs(cy - bandSY),
      };
    }

    // Eraser indicator tracking
    if (drawMode && eraseMode) {
      eraserScreenX = e.clientX;
      eraserScreenY = e.clientY;
      eraserVisible = true;
    }
  }

  async function onViewportPointerUp(e: PointerEvent) {
    activePointers.delete(e.pointerId);
    if (activePointers.size < 2) pinchStartDist = 0;
    if (activePointers.size === 0) isPanning = false;

    // Finalize rubber-band selection
    if (bandActive) {
      bandActive = false;
      const r = bandRect;
      bandRect = null;
      if (r && r.w > 8 && r.h > 8) {
        const inRect = (x: number, y: number) =>
          x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
        const ids = blocks.filter(b => {
          if (b.block_type === "shape") {
            try {
              const d = JSON.parse(b.content);
              if (d.shape === "line" || d.shape === "arrow") {
                // Connector: select if either endpoint is inside the rect
                return inRect(d.x1 ?? b.x, d.y1 ?? b.y) ||
                       inRect(d.x2 ?? b.x + b.width, d.y2 ?? b.y + b.height);
              }
            } catch {}
          }
          // Regular block or shape: bounding box intersection
          return b.x < r.x + r.w && b.x + b.width > r.x &&
                 b.y < r.y + r.h && b.y + b.height > r.y;
        }).map(b => b.id);
        selectedBlockIds = new Set(ids);
      }
      return;
    }

    // Shape drag-to-create: finalize
    if (shapeDragging) {
      shapeDragging = false;
      const preview = shapePreview;
      shapePreview = null;
      const MIN_SIZE = 20;
      if (isConnectorType(shapeType)) {
        const len = Math.hypot(shapeEndX - shapeStartX, shapeEndY - shapeStartY);
        if (len > 10) {
          const x1 = shapeStartX, y1 = shapeStartY, x2 = shapeEndX, y2 = shapeEndY;
          const content = JSON.stringify({
            shape: shapeType, x1, y1, x2, y2,
            stroke: shapeStroke, strokeWidth: shapeStrokeWidth,
            lineStyle,
            startId: createSnapStart?.id ?? null, endId: createSnapEnd?.id ?? null,
            startPort: createSnapStart?.port ?? "center", endPort: createSnapEnd?.port ?? "center",
          });
          const bx = Math.min(x1,x2), by = Math.min(y1,y2);
          const bw = Math.max(Math.abs(x2-x1), 10), bh = Math.max(Math.abs(y2-y1), 10);
          const block = await createBlock(pageId, "shape", bx, by, bw, bh, content);
          pushHistory({ type: "block_added", blockId: block.id });
          maxZ = block.z_index; blocks = [...blocks, block];
        }
        // Always deselect and stay in shape mode after connector
        selectedBlockId = null;
        createSnapStart = null; createSnapEnd = null;
      } else if (preview && preview.w > MIN_SIZE && preview.h > MIN_SIZE) {
        const content = JSON.stringify({ shape: shapeType, fill: shapeFill, stroke: shapeStroke, strokeWidth: shapeStrokeWidth, text: "", rotation: 0 });
        const block = await createBlock(pageId, "shape", preview.x, preview.y, preview.w, preview.h, content);
        pushHistory({ type: "block_added", blockId: block.id });
        maxZ = block.z_index; blocks = [...blocks, block];
        // Stay in shape mode, deselect so user can draw the next shape immediately
        selectedBlockId = null;
      } else {
        // Small click on empty canvas: just deselect, stay in shape mode
        selectedBlockId = null;
      }
    }
  }

  function onViewportPointerCancel(e: PointerEvent) {
    activePointers.delete(e.pointerId);
    if (activePointers.size < 2) pinchStartDist = 0;
    if (activePointers.size === 0) isPanning = false;
    bandActive = false; bandRect = null;
    // Cancel shape drag
    shapeDragging = false;
    shapePreview = null;
  }

  function onViewportMouseLeave() {
    eraserVisible = false;
  }

  // ── Selected shape helpers ────────────────────────────
  function parseShapeContent(s: string) {
    try { return { shape: "rect", fill: "#6366f1", stroke: "#1e1e2e", strokeWidth: 2, text: "", rotation: 0, lineStyle: "solid", ...JSON.parse(s || "{}") }; }
    catch { return { shape: "rect", fill: "#6366f1", stroke: "#1e1e2e", strokeWidth: 2, text: "", rotation: 0, lineStyle: "solid" }; }
  }

  $: selectedShape = selectedBlockId
    ? (blocks.find(b => b.id === selectedBlockId && b.block_type === "shape") ?? null)
    : null;

  // Track which shape ID was last synced to toolbar colors (avoids loop)
  let _lastSyncedShapeId: string | null = null;

  $: editingConnector = selectedShape !== null &&
    (() => { try { const t = JSON.parse(selectedShape!.content).shape; return t === "line" || t === "arrow"; } catch { return false; } })();

  $: if (selectedShape) {
    const d = parseShapeContent(selectedShape.content);
    if (selectedShape.id !== _lastSyncedShapeId) {
      _lastSyncedShapeId = selectedShape.id;
      shapeFill        = d.fill       ?? shapeFill;
      shapeStroke      = d.stroke;
      shapeStrokeWidth = d.strokeWidth;
      lineStyle        = d.lineStyle  ?? "solid";
      shapeMode        = true;
    } else if (
      d.fill !== shapeFill || d.stroke !== shapeStroke ||
      d.strokeWidth !== shapeStrokeWidth || (d.lineStyle ?? "solid") !== lineStyle
    ) {
      handleBlockUpdate(selectedShape.id, {
        content: JSON.stringify({ ...d, fill: shapeFill, stroke: shapeStroke, strokeWidth: shapeStrokeWidth, lineStyle })
      });
    }
  } else {
    _lastSyncedShapeId = null;
  }

  async function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") { selectedBlockId = null; return; }
    if ((e.ctrlKey || e.metaKey) && e.key === "z") {
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) return;
      e.preventDefault();
      if (canUndo) undo();
      return;
    }
    if (e.key === "Delete" || e.key === "Backspace") {
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) return;
      if (selectedBlockIds.size > 0) {
        const ids = [...selectedBlockIds];
        selectedBlockIds = new Set();
        for (const id of ids) await handleBlockDelete(id);
        return;
      }
      if (selectedBlockId) {
        await handleBlockDelete(selectedBlockId);
        selectedBlockId = null;
      }
    }
  }

  onMount(() => {
    load();
    window.addEventListener("wheel", onWheel, { capture: true, passive: false });
    window.addEventListener("paste", onPaste);
    window.addEventListener("keydown", onKeyDown);
    // Global pinch: capture phase so we see pointers even on blocks
    window.addEventListener("pointerdown",   onGlobalPointerDown,  { capture: true });
    window.addEventListener("pointermove",   onGlobalPointerMove,  { capture: true });
    window.addEventListener("pointerup",     onGlobalPointerUp,    { capture: true });
    window.addEventListener("pointercancel", onGlobalPointerUp,    { capture: true });
    ctx = canvasEl?.getContext("2d") ?? null;
    redrawAll();
    return () => {
      window.removeEventListener("wheel", onWheel, { capture: true });
      window.removeEventListener("paste", onPaste);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("pointerdown",   onGlobalPointerDown,  { capture: true });
      window.removeEventListener("pointermove",   onGlobalPointerMove,  { capture: true });
      window.removeEventListener("pointerup",     onGlobalPointerUp,    { capture: true });
      window.removeEventListener("pointercancel", onGlobalPointerUp,    { capture: true });
    };
  });

  // ── Paste image ───────────────────────────────────────
  async function onPaste(e: ClipboardEvent) {
    const items = e.clipboardData?.items;
    if (!items) return;
    for (const item of Array.from(items)) {
      if (item.type.startsWith("image/")) {
        const file = item.getAsFile();
        if (!file) continue;
        const ab = await file.arrayBuffer();
        const bytes = Array.from(new Uint8Array(ab));
        const ext = item.type;
        try {
          const appFile = await invoke<AppFile>("save_image_bytes", { spaceId, bytes, ext });
          const content = JSON.stringify({
            stored_path: appFile.stored_path, name: appFile.name,
            file_type: "image", size: appFile.size,
          });
          const offset = (blocks.length % 8) * 24;
          const block = await createBlock(pageId, "file", 80 + offset, 80 + offset, 500, 380, content);
          pushHistory({ type: "block_added", blockId: block.id });
          maxZ = block.z_index;
          blocks = [...blocks, block];
        } catch (err) { console.error("paste error:", err); }
        break;
      }
    }
  }

  // ── Drawing ───────────────────────────────────────────
  function onCanvasPointerDown(e: PointerEvent) {
    if (!drawMode || !ctx) return;
    if (!e.isPrimary && activePointers.size < 2) return;
    e.preventDefault();
    canvasEl.setPointerCapture(e.pointerId);
    drawing = true;
    const [x, y] = getPos(e);
    if (eraseMode) { erasePushedHistory = false; eraseAt(x, y); return; }
    currentStroke = { color: penColor, width: penWidth * (e.pressure || 1), points: [[x, y]], layer: "top", opacity: penOpacity };
  }

  function onCanvasPointerMove(e: PointerEvent) {
    if (!drawing || !drawMode) return;
    e.preventDefault();
    const [x, y] = getPos(e);
    if (eraseMode) { eraseAt(x, y); return; }
    if (!currentStroke || !markerCtx) return;
    currentStroke.points.push([x, y]);
    redrawAll();
    renderStroke(markerCtx, currentStroke);
  }

  function onCanvasPointerUp() {
    if (!drawing || !drawMode) return;
    drawing = false;
    if (!eraseMode && currentStroke && currentStroke.points.length > 1) {
      pushHistory({ type: "strokes", before: strokes });
      strokes = [...strokes, currentStroke];
      saveStrokes(pageId, JSON.stringify(strokes));
    }
    currentStroke = null;
    erasePushedHistory = false;
    redrawAll();
  }

  function getPos(e: PointerEvent): [number, number] { return getCanvasPos(e, canvasEl); }

  function redrawAll() { _redrawAll(ctx, markerCtx, strokes); }

  function eraseAt(x: number, y: number) {
    const result = _eraseAt(x, y, strokes, eraserSize);
    if (!result.changed) return;
    if (!erasePushedHistory) {
      pushHistory({ type: "strokes", before: strokes });
      erasePushedHistory = true;
    }
    strokes = result.strokes;
    redrawAll();
    saveStrokes(pageId, JSON.stringify(strokes));
  }

  // ── Blocks ────────────────────────────────────────────
  const DEFAULT_SIZES: Record<BlockType, [number, number]> = {
    note: [420, 320], link: [560, 400], file: [420, 360], task: [360, 300],
    calendar: [820, 580], clock: [300, 220], shape: [200, 200],
  };
  const DEFAULT_CONTENT: Record<BlockType, object> = {
    note: { title: $t.canvas.newNote, text: "" },
    link: { url: "", title: "", link_type: "general" },
    file: { stored_path: "", name: "", file_type: "other", size: 0 },
    task: { tasks: [] },
    calendar: {},
    clock: { mode: "clock" },
    shape: { shape: "rect", fill: "#6366f1", stroke: "#4f46e5", text: "" },
  };

  // Convierte posición de puntero (viewport space) a coordenadas del canvas
  function pointerToCanvas(e: PointerEvent): [number, number] {
    const rect = viewportEl.getBoundingClientRect();
    const cx = (e.clientX - rect.left + viewportEl.scrollLeft) / zoom;
    const cy = (e.clientY - rect.top  + viewportEl.scrollTop)  / zoom;
    return [Math.max(0, cx), Math.max(0, cy)];
  }


  async function onAudioRecorded(storedPath: string, name: string, size: number) {
    showRecordModal = false;
    const offset = (blocks.length % 8) * 28;
    const x = 80 + offset, y = 80 + offset;
    const content = JSON.stringify({ stored_path: storedPath, name, file_type: "audio", size });
    const block = await createBlock(pageId, "file", x, y, 360, 160, content);
    pushHistory({ type: "block_added", blockId: block.id });
    maxZ = block.z_index;
    blocks = [...blocks, block];
  }

  async function onVideoRecorded(storedPath: string, name: string, size: number) {
    showVideoModal = false;
    const offset = (blocks.length % 8) * 28;
    const x = 80 + offset, y = 80 + offset;
    const content = JSON.stringify({ stored_path: storedPath, name, file_type: "video", size });
    const block = await createBlock(pageId, "file", x, y, 480, 290, content);
    pushHistory({ type: "block_added", blockId: block.id });
    maxZ = block.z_index;
    blocks = [...blocks, block];
  }

  async function addBlock(type: BlockType, hint?: string) {
    // Stagger en pantalla: 28px independiente del zoom
    const screenOffset = (blocks.length % 8) * 28;
    const offset = screenOffset / zoom;
    // Centro del viewport actual en coordenadas de canvas
    const vx = viewportEl ? (viewportEl.scrollLeft + viewportEl.clientWidth  / 2) / zoom : CW / 2;
    const vy = viewportEl ? (viewportEl.scrollTop  + viewportEl.clientHeight / 2) / zoom : CH / 2;

    function clampPos(x: number, y: number, w: number, h: number): [number, number] {
      return [
        Math.max(20, Math.min(CW - w - 20, x)),
        Math.max(20, Math.min(CH - h - 20, y)),
      ];
    }

    // Convierte tamaño en pantalla → canvas-space (tamaño visual constante a cualquier zoom)
    function sz(screenW: number, screenH: number): [number, number] {
      return [Math.round(screenW / zoom), Math.round(screenH / zoom)];
    }

    if (hint === "record-audio") { showRecordModal = true; return; }
    if (hint === "record-video") { showVideoModal = true; return; }
    if (type === "file") {
      const [fw, fh] = sz(...DEFAULT_SIZES.file);
      const [fx, fy] = clampPos(vx - fw/2 + offset, vy - fh/2 + offset, fw, fh);
      await addFileBlock(fx, fy, zoom, hint);
      return;
    }
    if (hint === "youtube") {
      const [w, h] = sz(560, 380);
      const [x, y] = clampPos(vx - w/2 + offset, vy - h/2 + offset, w, h);
      const block = await createBlock(pageId, "link", x, y, w, h,
        JSON.stringify({ url: "", title: "", link_type: "youtube" }));
      pushHistory({ type: "block_added", blockId: block.id });
      maxZ = block.z_index;
      blocks = [...blocks, block];
      return;
    }
    if (hint === "canva") {
      const [w, h] = sz(600, 440);
      const [x, y] = clampPos(vx - w/2 + offset, vy - h/2 + offset, w, h);
      const block = await createBlock(pageId, "link", x, y, w, h,
        JSON.stringify({ url: "", title: "", link_type: "canva" }));
      pushHistory({ type: "block_added", blockId: block.id });
      maxZ = block.z_index;
      blocks = [...blocks, block];
      return;
    }
    const [w, h] = sz(...DEFAULT_SIZES[type]);
    const [x, y] = clampPos(vx - w/2 + offset, vy - h/2 + offset, w, h);
    const block = await createBlock(pageId, type, x, y, w, h, JSON.stringify(DEFAULT_CONTENT[type]));
    pushHistory({ type: "block_added", blockId: block.id });
    maxZ = block.z_index;
    blocks = [...blocks, block];
  }

  async function addFileBlock(x: number, y: number, z: number, hint?: string) {
    type FilterDef = { name: string; extensions: string[] };
    let filters: FilterDef[];
    let screenSize: [number, number] = DEFAULT_SIZES.file;

    if (hint === "image") {
      filters = [{ name: $t.canvas.images, extensions: ["jpg","jpeg","png","gif","webp","bmp","svg"] }];
      screenSize = [480, 360];
    } else if (hint === "video") {
      filters = [{ name: $t.canvas.videos, extensions: ["mp4","webm","mkv","avi","mov","ogv"] }];
      screenSize = [560, 380];
    } else if (hint === "audio") {
      filters = [{ name: $t.canvas.audioFiles, extensions: ["mp3","wav","ogg","flac","m4a","aac","opus","wma"] }];
      screenSize = [360, 160];
    } else {
      filters = [
        { name: $t.canvas.documents, extensions: ["pdf","doc","docx","xls","xlsx","ppt","pptx"] },
        { name: "PDF",        extensions: ["pdf"] },
        { name: $t.canvas.word,       extensions: ["doc","docx"] },
        { name: $t.canvas.excel,      extensions: ["xls","xlsx"] },
        { name: $t.canvas.powerpoint, extensions: ["ppt","pptx"] },
      ];
    }

    const path = await open({ multiple: false, filters });
    if (!path || typeof path !== "string") return;
    const appFile = await importFile(spaceId, path);
    const content = JSON.stringify({ stored_path: appFile.stored_path, name: appFile.name, file_type: appFile.file_type, size: appFile.size });
    const w = Math.round(screenSize[0] / z);
    const h = Math.round(screenSize[1] / z);
    const block = await createBlock(pageId, "file", x, y, w, h, content);
    pushHistory({ type: "block_added", blockId: block.id });
    maxZ = block.z_index;
    blocks = [...blocks, block];
  }

  function bringToFront(blockId: string) {
    maxZ += 1;
    blocks = blocks.map(b => b.id === blockId ? { ...b, z_index: maxZ } : b);
    updateBlockZindex(blockId, maxZ);
  }

  function selectBlock(id: string) {
    selectedBlockId = id;
  }

  async function handleBlockUpdate(blockId: string, changes: Partial<Block>) {
    if (changes.x !== undefined || changes.width !== undefined) {
      const prev = blocks.find(b => b.id === blockId);
      if (prev) pushHistory({ type: "block_moved", blockId, x: prev.x, y: prev.y, w: prev.width, h: prev.height });
    }
    blocks = blocks.map(b => b.id === blockId ? { ...b, ...changes } : b);
    if (changes.x !== undefined || changes.y !== undefined) {
      const moved = blocks.find(b => b.id === blockId);
      if (moved) updateConnectedLines(moved);
    }
    if (changes.content !== undefined) await updateBlockContent(blockId, changes.content);
    if (changes.x !== undefined || changes.width !== undefined) {
      const b = blocks.find(bl => bl.id === blockId)!;
      await updateBlockPosition(blockId, b.x, b.y, b.width, b.height);
    }
  }

  async function handleBlockDelete(blockId: string) {
    const block = blocks.find(b => b.id === blockId);
    if (!block) return;
    pushHistory({ type: "block_deleted", block: { ...block } });
    await deleteBlock(blockId);
    blocks = blocks.filter(b => b.id !== blockId);
  }

  function onMultiDragStart(e: PointerEvent) {
    e.preventDefault();
    e.stopPropagation();
    multiDragging = true;
    const [cx, cy] = pointerToCanvas(e);
    multiDragOriginX = cx; multiDragOriginY = cy;
    multiDragOrigPos = new Map(
      blocks.filter(b => selectedBlockIds.has(b.id)).map(b => [b.id, { x: b.x, y: b.y }])
    );
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  let _multiDragRaf: number | null = null;
  let _multiDragDx = 0, _multiDragDy = 0;
  function onMultiDragMove(e: PointerEvent) {
    if (!multiDragging) return;
    const [cx, cy] = pointerToCanvas(e);
    _multiDragDx = cx - multiDragOriginX;
    _multiDragDy = cy - multiDragOriginY;
    if (_multiDragRaf !== null) return;
    _multiDragRaf = requestAnimationFrame(() => {
      _multiDragRaf = null;
      const dx = _multiDragDx, dy = _multiDragDy;
      blocks = blocks.map(b => {
        if (!selectedBlockIds.has(b.id)) return b;
        const o = multiDragOrigPos.get(b.id)!;
        return { ...b, x: Math.max(0, o.x + dx), y: Math.max(0, o.y + dy) };
      });
    });
  }

  async function onMultiDragEnd(_e: PointerEvent) {
    if (!multiDragging) return;
    multiDragging = false;
    if (_multiDragRaf !== null) { cancelAnimationFrame(_multiDragRaf); _multiDragRaf = null; }
    // Apply final position before saving
    const dx = _multiDragDx, dy = _multiDragDy;
    blocks = blocks.map(b => {
      if (!selectedBlockIds.has(b.id)) return b;
      const o = multiDragOrigPos.get(b.id)!;
      return { ...b, x: Math.max(0, o.x + dx), y: Math.max(0, o.y + dy) };
    });
    for (const id of selectedBlockIds) {
      const b = blocks.find(bl => bl.id === id);
      if (b) await updateBlockPosition(id, b.x, b.y, b.width, b.height);
    }
  }

  function updateConnectedLines(movedBlock: Block, persist = true) {
    for (const b of blocks) {
      if (b.block_type !== "shape") continue;
      let d: any;
      try { d = JSON.parse(b.content); } catch { continue; }
      if (d.shape !== "line" && d.shape !== "arrow") continue;
      let changed = false;
      if (d.startId === movedBlock.id) {
        const [px, py] = getPortCoords(movedBlock, d.startPort ?? "center");
        d.x1 = px; d.y1 = py; changed = true;
      }
      if (d.endId === movedBlock.id) {
        const [px, py] = getPortCoords(movedBlock, d.endPort ?? "center");
        d.x2 = px; d.y2 = py; changed = true;
      }
      if (!changed) continue;
      const newContent = JSON.stringify(d);
      const nx = Math.min(d.x1, d.x2), ny = Math.min(d.y1, d.y2);
      const nw = Math.max(Math.abs(d.x2-d.x1), 10), nh = Math.max(Math.abs(d.y2-d.y1), 10);
      blocks = blocks.map(bl => bl.id === b.id ? {...bl, content: newContent, x: nx, y: ny, width: nw, height: nh} : bl);
      if (persist) {
        updateBlockContent(b.id, newContent);
        updateBlockPosition(b.id, nx, ny, nw, nh);
      }
    }
  }

  // Lightweight move callback — updates block position during drag, no DB writes.
  // CanvasShape reads connected endpoints directly from allBlocks, so no need to
  // call updateConnectedLines here. RAF is intentionally NOT used to avoid firing
  // after drag-end and overwriting the final persisted position.
  function handleBlockMove(blockId: string, x: number, y: number) {
    const idx = blocks.findIndex(bl => bl.id === blockId);
    if (idx === -1) return;
    const b = blocks[idx];
    if (b.x === x && b.y === y) return;
    blocks = blocks.map(bl => bl.id === blockId ? { ...bl, x, y } : bl);
  }

  // Start a connector drag from a port dot on a shape (no tool selection needed)
  function handleConnectorStart(blockId: string, port: string, x: number, y: number) {
    if (!isConnectorType(shapeType)) shapeType = "arrow";
    selectedBlockId = null;
    shapeDragging = true;
    shapeStartX = x; shapeStartY = y;
    shapePreview = { x, y, w: 0, h: 0 };
    shapeEndX = x; shapeEndY = y;
    createSnapStart = { id: blockId, port, x, y };
  }

  function clearStrokes() {
    pushHistory({ type: "strokes", before: strokes });
    strokes = []; redrawAll(); saveStrokes(pageId, "[]");
  }

  // ── Double-click to type ───────────────────────────────
  let textDraft: { x: number; y: number } | null = null;
  let textDraftEl: HTMLDivElement;
  let _draftHandled = false; // prevent double-confirm when Escape removes element

  async function onViewportDblClick(e: MouseEvent) {
    if (drawMode || shapeMode || selectMode) return;
    if (!isCanvasBg(e.target)) return;
    e.preventDefault();
    selectedBlockId = null;
    const rect = viewportEl.getBoundingClientRect();
    const cx = (e.clientX - rect.left + viewportEl.scrollLeft) / zoom;
    const cy = (e.clientY - rect.top  + viewportEl.scrollTop)  / zoom;
    textDraft = { x: cx, y: cy };
    await tick();
    textDraftEl?.focus();
  }

  function onTextDraftKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      _draftHandled = true;
      textDraft = null;
    } else if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      textDraftEl?.blur();
    }
  }

  async function onTextDraftBlur() {
    if (_draftHandled) { _draftHandled = false; return; }
    const text = textDraftEl?.innerText?.trim() ?? "";
    const pos = textDraft;
    _draftHandled = true;
    textDraft = null;
    if (!text || !pos) return;
    // Estimate size from text length (rough heuristic)
    const lines = text.split("\n");
    const maxLen = Math.max(...lines.map(l => l.length));
    const w = Math.max(80, Math.min(maxLen * 10, 600));
    const h = Math.max(40, lines.length * 26 + 16);
    const bx = Math.max(0, pos.x - w / 2);
    const by = Math.max(0, pos.y - h / 2);
    // Create a transparent shape — moveable, resizable, editable on dblclick
    const content = JSON.stringify({
      shape: "rect", fill: "transparent", stroke: "transparent",
      strokeWidth: 0, text, rotation: 0,
    });
    const block = await createBlock(pageId, "shape", bx, by, w, h, content);
    pushHistory({ type: "block_added", blockId: block.id });
    maxZ = block.z_index;
    blocks = [...blocks, block];
    selectedBlockId = block.id;
  }

  // Cursor for canvas area
  $: canvasBgCursor = drawMode
    ? (eraseMode ? "none" : "crosshair")
    : shapeMode ? "crosshair"
    : selectMode ? "crosshair"
    : "move";
</script>

<!-- Eraser circle indicator (fixed, follows mouse, outside scaled canvas) -->
{#if drawMode && eraseMode && eraserVisible}
  <div
    class="eraser-indicator"
    style="left:{eraserScreenX}px; top:{eraserScreenY}px; width:{eraserScreenR * 2}px; height:{eraserScreenR * 2}px;"
  ></div>
{/if}

<div class="page-root">
  <div
    class="canvas-viewport"
    bind:this={viewportEl}
    style="cursor:{canvasBgCursor}"
    on:pointerdown={onViewportPointerDown}
    on:pointermove={onViewportPointerMove}
    on:pointerup={onViewportPointerUp}
    on:pointercancel={onViewportPointerCancel}
    on:mouseleave={onViewportMouseLeave}
    on:dblclick={onViewportDblClick}
    role="presentation"
  >
    <div class="canvas-scaler" style="width:{CW * zoom}px; height:{CH * zoom}px">
      <div class="canvas-content" style="width:{CW}px; height:{CH}px; transform:scale({zoom}); transform-origin:top left">

        <canvas
          class="drawing-canvas"
          class:draw-active={drawMode}
          width={canvasPxW} height={canvasPxH}
          bind:this={canvasEl}
          on:pointerdown={onCanvasPointerDown}
          on:pointermove={onCanvasPointerMove}
          on:pointerup={onCanvasPointerUp}
          style="width:{CW}px; height:{CH}px; cursor:{drawMode ? (eraseMode ? 'none' : 'crosshair') : 'default'}"
        ></canvas>

        <canvas
          class="marker-canvas"
          width={canvasPxW} height={canvasPxH}
          bind:this={markerCanvasEl}
          style="width:{CW}px; height:{CH}px;"
        ></canvas>

        <!-- Connector (line/arrow) drag preview -->
        {#if shapeDragging && isConnectorType(shapeType) && Math.hypot(shapeEndX-shapeStartX, shapeEndY-shapeStartY) > 4}
          <svg style="position:absolute;left:0;top:0;width:{CW}px;height:{CH}px;overflow:visible;pointer-events:none;z-index:998;">
            <defs>
              {#if shapeType === "arrow"}
                <marker id="prev-arrow" markerWidth="6" markerHeight="4" refX="6" refY="2" orient="auto" markerUnits="strokeWidth">
                  <polygon points="0 0, 6 2, 0 4" fill={shapeStroke} />
                </marker>
              {/if}
            </defs>
            <line x1={shapeStartX} y1={shapeStartY} x2={shapeEndX} y2={shapeEndY}
                  stroke={shapeStroke} stroke-width={shapeStrokeWidth}
                  stroke-linecap="round" stroke-dasharray="8 5"
                  marker-end={shapeType === "arrow" ? "url(#prev-arrow)" : ""} />
            {#if createSnapStart}
              <circle cx={createSnapStart.x} cy={createSnapStart.y} r="10"
                      fill="rgba(99,102,241,0.15)" stroke="var(--accent)" stroke-width="2" />
            {/if}
            {#if createSnapEnd}
              <circle cx={createSnapEnd.x} cy={createSnapEnd.y} r="10"
                      fill="rgba(99,102,241,0.15)" stroke="var(--accent)" stroke-width="2" />
            {/if}
          </svg>
        {/if}

        <!-- Shape drag preview -->
        {#if shapePreview && shapePreview.w > 4 && shapePreview.h > 4}
          <div class="shape-drag-preview" style="left:{shapePreview.x}px; top:{shapePreview.y}px; width:{shapePreview.w}px; height:{shapePreview.h}px;">
            <svg viewBox="0 0 100 100" preserveAspectRatio="none" style="width:100%;height:100%;display:block;">
              {#if shapeType === "rect"}
                <rect x="1" y="1" width="98" height="98" rx="6" fill={shapeFill} stroke={shapeStroke} stroke-width="3" opacity="0.55" />
              {:else if shapeType === "circle"}
                <ellipse cx="50" cy="50" rx="49" ry="49" fill={shapeFill} stroke={shapeStroke} stroke-width="3" opacity="0.55" />
              {:else if shapeType === "triangle"}
                <path d="M50,4 L96,88 L4,88 Z" fill={shapeFill} stroke={shapeStroke} stroke-width="3" stroke-linejoin="round" opacity="0.55" />
              {:else if shapeType === "diamond"}
                <path d="M50,4 L96,50 L50,96 L4,50 Z" fill={shapeFill} stroke={shapeStroke} stroke-width="3" stroke-linejoin="round" opacity="0.55" />
              {:else if shapeType === "heart"}
                <path d="M50,82 C50,82 8,57 8,33 C8,18 18,8 33,8 C41,8 47,12 50,18 C53,12 59,8 67,8 C82,8 92,18 92,33 C92,57 50,82 50,82 Z" fill={shapeFill} stroke={shapeStroke} stroke-width="3" stroke-linejoin="round" opacity="0.55" />
              {/if}
            </svg>
          </div>
        {/if}

        <!-- Rubber-band selection rect -->
        {#if bandRect && bandRect.w > 4 && bandRect.h > 4}
          <div class="band-select"
            style="left:{bandRect.x}px; top:{bandRect.y}px; width:{bandRect.w}px; height:{bandRect.h}px;"
          ></div>
        {/if}

        <!-- Multi-select drag overlay -->
        {#if selectMode && selectedBlockIds.size > 0}
          {@const selBlocks = blocks.filter(b => selectedBlockIds.has(b.id))}
          {#if selBlocks.length > 0}
            {@const bx  = Math.min(...selBlocks.map(b => b.x))}
            {@const by  = Math.min(...selBlocks.map(b => b.y))}
            {@const bx2 = Math.max(...selBlocks.map(b => b.x + b.width))}
            {@const by2 = Math.max(...selBlocks.map(b => b.y + b.height))}
            <div class="multi-drag-overlay"
              style="left:{bx-6}px; top:{by-6}px; width:{bx2-bx+12}px; height:{by2-by+12}px; z-index:{maxZ+10};"
              on:pointerdown={onMultiDragStart}
              on:pointermove={onMultiDragMove}
              on:pointerup={onMultiDragEnd}
              on:pointercancel={onMultiDragEnd}
              role="presentation"
            ></div>
          {/if}
        {/if}

        {#each blocks as block (block.id)}
          {#if block.block_type === "shape"}
            <CanvasShape
              {block}
              {drawMode}
              {zoom}
              selected={selectedBlockId === block.id}
              multiSelected={selectedBlockIds.has(block.id)}
              connectorMode={shapeMode && isConnectorType(shapeType)}
              onConnectorStart={handleConnectorStart}
              onUpdate={handleBlockUpdate}
              onMove={handleBlockMove}
              onDelete={handleBlockDelete}
              onBringToFront={bringToFront}
              onSelect={selectBlock}
              allBlocks={blocks}
            />
          {:else}
            <BlockContainer
              {block}
              {drawMode}
              {zoom}
              selected={selectedBlockId === block.id}
              multiSelected={selectedBlockIds.has(block.id)}
              connectorMode={shapeMode && isConnectorType(shapeType)}
              onUpdate={handleBlockUpdate}
              onMove={handleBlockMove}
              onDelete={handleBlockDelete}
              onBringToFront={bringToFront}
              onSelect={selectBlock}
            />
          {/if}
        {/each}

        <!-- Inline text draft (double-click to type) -->
        {#if textDraft !== null}
          <!-- svelte-ignore a11y-no-static-element-interactions -->
          <div
            class="canvas-text-draft"
            style="left:{textDraft.x}px; top:{textDraft.y}px;"
            contenteditable="true"
            role="textbox"
            tabindex="0"
            aria-label={$t.canvas.writeSomething}
            data-placeholder={$t.canvas.writeSomething}
            bind:this={textDraftEl}
            on:blur={onTextDraftBlur}
            on:keydown={onTextDraftKey}
          ></div>
        {/if}

      </div>
    </div>
  </div>

  {#if showRecordModal}
    <RecordAudioModal
      {spaceId}
      onSave={onAudioRecorded}
      onClose={() => showRecordModal = false}
    />
  {/if}

  {#if showVideoModal}
    <RecordVideoModal
      {spaceId}
      onSave={onVideoRecorded}
      onClose={() => showVideoModal = false}
    />
  {/if}

  <CanvasToolbar
    bind:drawMode
    bind:eraseMode
    bind:shapeMode
    bind:shapeType
    bind:shapeFill
    bind:shapeStroke
    bind:shapeStrokeWidth
    bind:penColor
    bind:penWidth
    bind:penOpacity
    bind:zoom
    bind:eraserSize
    bind:selectMode
    bind:lineStyle
    {canUndo}
    editingShape={selectedShape !== null}
    {editingConnector}
    onAddBlock={addBlock}
    onClearStrokes={clearStrokes}
    onUndo={undo}
  />
</div>

<style>
  .page-root { display: flex; flex-direction: column; flex: 1; overflow: hidden; }

  .canvas-viewport {
    flex: 1; overflow: auto;
    touch-action: none;
    background-color: var(--bg-base);
    background-image: radial-gradient(circle, var(--border) 1px, transparent 1px);
    background-size: 28px 28px;
    /* Hide scrollbars */
    scrollbar-width: none;
    /* overscroll bounce prevention */
    overscroll-behavior: none;
  }
  .canvas-viewport::-webkit-scrollbar { display: none; }

  .canvas-scaler { position: relative; }
  .canvas-content { position: relative; }

  .drawing-canvas {
    position: absolute; inset: 0;
    z-index: 0; pointer-events: none; touch-action: none;
  }
  .drawing-canvas.draw-active { z-index: 9991; pointer-events: all; }

  .marker-canvas {
    position: absolute; inset: 0;
    z-index: 9990; pointer-events: none; touch-action: none;
  }

  /* Shape drag preview */
  .shape-drag-preview {
    position: absolute;
    pointer-events: none;
    z-index: 998;
  }

  /* Rubber-band selection */
  .band-select {
    position: absolute; pointer-events: none; z-index: 9998;
    border: 1.5px dashed var(--accent);
    background: rgba(99,102,241,0.07);
    border-radius: 3px;
  }

  /* Multi-select drag overlay */
  .multi-drag-overlay {
    position: absolute; cursor: move;
    border: 2px dashed var(--accent);
    background: rgba(99,102,241,0.05);
    border-radius: 8px;
  }

  /* Inline text draft (double-click to type) */
  .canvas-text-draft {
    position: absolute;
    min-width: 120px; min-height: 32px;
    max-width: 600px;
    padding: 8px 14px;
    background: var(--bg-surface);
    border: 1.5px solid var(--accent);
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.25);
    font-size: 14px; font-weight: 500;
    color: var(--text-primary);
    line-height: 1.5;
    outline: none;
    white-space: pre-wrap;
    word-break: break-word;
    cursor: text;
    z-index: 9995;
    /* Center on click point */
    transform: translate(-50%, -50%);
    /* Grow with content (contenteditable auto-sizes) */
  }
  .canvas-text-draft:empty::before {
    content: attr(data-placeholder);
    color: var(--text-muted);
    pointer-events: none;
  }

  /* Eraser circle indicator */
  .eraser-indicator {
    position: fixed;
    border-radius: 50%;
    border: 2px solid rgba(150, 150, 150, 0.7);
    background: rgba(255, 255, 255, 0.12);
    pointer-events: none;
    transform: translate(-50%, -50%);
    z-index: 9999;
    transition: width 0.1s, height 0.1s;
  }
</style>
