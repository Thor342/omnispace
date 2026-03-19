<script lang="ts">
  import { onMount, tick } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";
  import {
    getBlocks, createBlock, updateBlockPosition, updateBlockContent,
    updateBlockZindex, deleteBlock, restoreBlock, getStrokes, saveStrokes, importFile
  } from "../../api";
  import type { Block, BlockType, StrokePath, AppFile } from "../../types";
  import BlockContainer from "../blocks/Block.svelte";
  import CanvasShape from "./CanvasShape.svelte";
  import CanvasToolbar from "./CanvasToolbar.svelte";

  export let pageId: string;
  export let spaceId: string;

  // ── State ─────────────────────────────────────────────
  let blocks: Block[] = [];
  let strokes: StrokePath[] = [];
  let maxZ = 0;
  let selectedBlockId: string | null = null;

  // Draw
  let drawMode = false;
  let eraseMode = false;
  let shapeMode = false;
  let shapeType = "rect";

  // Shape drag-to-create
  let shapeDragging = false;
  let shapeStartX = 0, shapeStartY = 0;
  let shapePreview: { x: number; y: number; w: number; h: number } | null = null;
  let shapeFill = "#6366f1";
  let shapeStroke = "#1e1e2e";
  let shapeStrokeWidth = 2;
  // Connector (line/arrow) creation tracking
  let shapeEndX = 0, shapeEndY = 0;
  let createStartSnapId: string | null = null;
  let createEndSnapId: string | null = null;
  let penColor = "#6366f1";
  let penWidth = 3;
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

  // Multi-touch pinch tracking
  let activePointers = new Map<number, { x: number; y: number }>();
  let pinchStartDist = 0;
  let pinchStartZoom = 1.0;

  // Canvas
  let canvasEl: HTMLCanvasElement;
  let viewportEl: HTMLDivElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let drawing = false;
  let erasePushedHistory = false; // push undo only once per erase gesture
  let currentStroke: StrokePath | null = null;
  const CW = 3200;
  const CH = 2400;

  // ── Undo history ──────────────────────────────────────
  type HistoryEntry =
    | { type: "strokes"; before: StrokePath[] }
    | { type: "block_deleted"; block: Block }
    | { type: "block_added"; blockId: string }
    | { type: "block_moved"; blockId: string; x: number; y: number; w: number; h: number };

  let historyStack: HistoryEntry[] = [];
  const MAX_HISTORY = 50;
  $: canUndo = historyStack.length > 0;

  function pushHistory(entry: HistoryEntry) {
    historyStack = [...historyStack.slice(-(MAX_HISTORY - 1)), entry];
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
    redrawAll();
  }

  // ── Helpers ───────────────────────────────────────────
  function clampZoom(z: number) {
    return Math.round(Math.min(3, Math.max(0.2, z)) * 100) / 100;
  }

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
  // • ctrlKey=true  → trackpad PINCH or Ctrl+scroll → zoom
  // • ctrlKey=false → mouse wheel / trackpad two-finger scroll → native pan
  async function onWheel(e: WheelEvent) {
    if (!e.ctrlKey) return;
    e.preventDefault();

    // Capture mouse position relative to viewport BEFORE changing zoom
    const rect = viewportEl.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const prevZoom = zoom;

    // Cap delta so mouse wheel (delta ~100) and trackpad pinch (delta ~3) both feel natural
    const capped = Math.sign(e.deltaY) * Math.min(Math.abs(e.deltaY), 80);
    zoom = clampZoom(prevZoom * (1 - capped * 0.005));

    // Keep the content point under the cursor stationary after zoom
    const contentX = (viewportEl.scrollLeft + mx) / prevZoom;
    const contentY = (viewportEl.scrollTop  + my) / prevZoom;
    await tick();
    viewportEl.scrollLeft = contentX * zoom - mx;
    viewportEl.scrollTop  = contentY * zoom - my;
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
    if (!isCanvasBg(e.target)) return; // block handles itself

    // Shape drag-to-create: start
    if (shapeMode) {
      e.preventDefault();
      const [cx, cy] = pointerToCanvas(e);
      shapeDragging = true;
      shapeStartX = cx;
      shapeStartY = cy;
      shapePreview = { x: cx, y: cy, w: 0, h: 0 };
      shapeEndX = cx; shapeEndY = cy;
      if (isConnectorType(shapeType)) {
        const snap = findSnapTarget(cx, cy);
        createStartSnapId = snap?.id ?? null;
        if (snap) { shapeStartX = snap.x + snap.width/2; shapeStartY = snap.y + snap.height/2; }
      }
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
        const snap = findSnapTarget(cx, cy);
        createEndSnapId = snap?.id ?? null;
        shapeEndX = snap ? snap.x + snap.width/2 : cx;
        shapeEndY = snap ? snap.y + snap.height/2 : cy;
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

    if (isPanning) {
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
        const ids = blocks
          .filter(b => b.x < r.x + r.w && b.x + b.width > r.x &&
                       b.y < r.y + r.h && b.y + b.height > r.y)
          .map(b => b.id);
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
        const minLen = 10;
        const len = Math.hypot(shapeEndX - shapeStartX, shapeEndY - shapeStartY);
        if (len > minLen) {
          const x1 = shapeStartX, y1 = shapeStartY, x2 = shapeEndX, y2 = shapeEndY;
          const content = JSON.stringify({
            shape: shapeType, x1, y1, x2, y2,
            stroke: shapeStroke, strokeWidth: shapeStrokeWidth,
            startId: createStartSnapId, endId: createEndSnapId,
          });
          const bx = Math.min(x1,x2), by = Math.min(y1,y2);
          const bw = Math.max(Math.abs(x2-x1), 10), bh = Math.max(Math.abs(y2-y1), 10);
          const block = await createBlock(pageId, "shape", bx, by, bw, bh, content);
          pushHistory({ type: "block_added", blockId: block.id });
          maxZ = block.z_index; blocks = [...blocks, block]; selectedBlockId = block.id;
        }
        createStartSnapId = null; createEndSnapId = null;
      } else if (preview && preview.w > MIN_SIZE && preview.h > MIN_SIZE) {
        const content = JSON.stringify({ shape: shapeType, fill: shapeFill, stroke: shapeStroke, strokeWidth: shapeStrokeWidth, text: "", rotation: 0 });
        const block = await createBlock(pageId, "shape", preview.x, preview.y, preview.w, preview.h, content);
        pushHistory({ type: "block_added", blockId: block.id });
        maxZ = block.z_index; blocks = [...blocks, block]; selectedBlockId = block.id;
      } else {
        // Small click on empty canvas = deselect + exit shape mode
        selectedBlockId = null;
        shapeMode = false;
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
    try { return { shape: "rect", fill: "#6366f1", stroke: "#1e1e2e", strokeWidth: 2, text: "", rotation: 0, ...JSON.parse(s || "{}") }; }
    catch { return { shape: "rect", fill: "#6366f1", stroke: "#1e1e2e", strokeWidth: 2, text: "", rotation: 0 }; }
  }

  $: selectedShape = selectedBlockId
    ? (blocks.find(b => b.id === selectedBlockId && b.block_type === "shape") ?? null)
    : null;

  // Track which shape ID was last synced to toolbar colors (avoids loop)
  let _lastSyncedShapeId: string | null = null;

  $: if (selectedShape) {
    const d = parseShapeContent(selectedShape.content);
    if (selectedShape.id !== _lastSyncedShapeId) {
      // New shape selected → load its colors into toolbar and open shape mode
      _lastSyncedShapeId = selectedShape.id;
      shapeFill        = d.fill;
      shapeStroke      = d.stroke;
      shapeStrokeWidth = d.strokeWidth;
      shapeMode        = true;
    } else if (d.fill !== shapeFill || d.stroke !== shapeStroke || d.strokeWidth !== shapeStrokeWidth) {
      // User changed colors in toolbar → update the shape
      handleBlockUpdate(selectedShape.id, {
        content: JSON.stringify({ ...d, fill: shapeFill, stroke: shapeStroke, strokeWidth: shapeStrokeWidth })
      });
    }
  } else {
    _lastSyncedShapeId = null;
  }

  async function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") { selectedBlockId = null; return; }
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
    viewportEl?.addEventListener("wheel", onWheel, { passive: false });
    window.addEventListener("paste", onPaste);
    window.addEventListener("keydown", onKeyDown);
    ctx = canvasEl?.getContext("2d") ?? null;
    redrawAll();
    return () => {
      viewportEl?.removeEventListener("wheel", onWheel);
      window.removeEventListener("paste", onPaste);
      window.removeEventListener("keydown", onKeyDown);
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
    if (!e.isPrimary && activePointers.size < 2) return; // let 2nd finger go to pinch
    e.preventDefault();
    canvasEl.setPointerCapture(e.pointerId);
    drawing = true;
    const [x, y] = getPos(e);
    if (eraseMode) {
      erasePushedHistory = false; // reset per gesture
      eraseAt(x, y);
      return;
    }
    currentStroke = { color: penColor, width: penWidth * (e.pressure || 1), points: [[x, y]] };
  }

  function onCanvasPointerMove(e: PointerEvent) {
    if (!drawing || !ctx || !drawMode) return;
    e.preventDefault();
    const [x, y] = getPos(e);
    if (eraseMode) { eraseAt(x, y); return; }
    if (!currentStroke) return;
    currentStroke.points.push([x, y]);
    redrawAll();
    renderStroke(ctx!, currentStroke);
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

  function getPos(e: PointerEvent): [number, number] {
    const rect = canvasEl.getBoundingClientRect();
    return [
      (e.clientX - rect.left) * (CW / rect.width),
      (e.clientY - rect.top)  * (CH / rect.height),
    ];
  }

  // Segment-based eraser: splits strokes at erased points instead of removing whole stroke
  function eraseAt(x: number, y: number) {
    const r = eraserSize;
    let changed = false;
    const result: StrokePath[] = [];

    for (const stroke of strokes) {
      const hasHit = stroke.points.some(pt => Math.hypot(pt[0] - x, pt[1] - y) < r);
      if (!hasHit) { result.push(stroke); continue; }

      changed = true;
      // Split into sub-strokes at erased points
      let current: [number, number][] = [];
      for (const pt of stroke.points) {
        if (Math.hypot(pt[0] - x, pt[1] - y) < r) {
          if (current.length >= 2) result.push({ ...stroke, points: current });
          current = [];
        } else {
          current.push([pt[0], pt[1]]);
        }
      }
      if (current.length >= 2) result.push({ ...stroke, points: current });
    }

    if (!changed) return;

    if (!erasePushedHistory) {
      pushHistory({ type: "strokes", before: strokes });
      erasePushedHistory = true;
    }
    strokes = result;
    redrawAll();
    saveStrokes(pageId, JSON.stringify(strokes));
  }

  function redrawAll() {
    if (!ctx) return;
    ctx.clearRect(0, 0, CW, CH);
    for (const s of strokes) renderStroke(ctx!, s);
  }

  function renderStroke(c: CanvasRenderingContext2D, s: StrokePath) {
    if (s.points.length < 2) return;
    c.beginPath();
    c.strokeStyle = s.color;
    c.lineWidth = s.width;
    c.lineCap = "round"; c.lineJoin = "round";
    c.moveTo(s.points[0][0], s.points[0][1]);
    for (let i = 1; i < s.points.length - 1; i++) {
      const mx = (s.points[i][0] + s.points[i + 1][0]) / 2;
      const my = (s.points[i][1] + s.points[i + 1][1]) / 2;
      c.quadraticCurveTo(s.points[i][0], s.points[i][1], mx, my);
    }
    c.lineTo(s.points.at(-1)![0], s.points.at(-1)![1]);
    c.stroke();
  }

  // ── Blocks ────────────────────────────────────────────
  const DEFAULT_SIZES: Record<BlockType, [number, number]> = {
    note: [420, 320], link: [560, 400], file: [420, 360], task: [360, 300],
    calendar: [820, 580], clock: [300, 220], shape: [200, 200],
  };
  const DEFAULT_CONTENT: Record<BlockType, object> = {
    note: { title: "Nueva nota", text: "" },
    link: { url: "", title: "Nuevo enlace", link_type: "general" },
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

  function isConnectorType(t: string) { return t === "line" || t === "arrow"; }

  function findSnapTarget(cx: number, cy: number, excludeId?: string): Block | null {
    const PAD = 28; // zona de snap alrededor del bloque
    for (const b of blocks) {
      if (b.id === excludeId) continue;
      if (b.block_type === "shape") {
        try { const d = JSON.parse(b.content); if (d.shape === "line" || d.shape === "arrow") continue; } catch {}
      }
      if (cx >= b.x - PAD && cx <= b.x + b.width  + PAD &&
          cy >= b.y - PAD && cy <= b.y + b.height + PAD) return b;
    }
    return null;
  }

  async function addBlock(type: BlockType) {
    const offset = (blocks.length % 8) * 28;
    const x = 80 + offset, y = 80 + offset;
    if (type === "file") { await addFileBlock(x, y); return; }
    const [w, h] = DEFAULT_SIZES[type];
    const block = await createBlock(pageId, type, x, y, w, h, JSON.stringify(DEFAULT_CONTENT[type]));
    pushHistory({ type: "block_added", blockId: block.id });
    maxZ = block.z_index;
    blocks = [...blocks, block];
  }

  async function addFileBlock(x: number, y: number) {
    const path = await open({
      multiple: false,
      filters: [
        { name: "Documentos", extensions: ["pdf","doc","docx","xls","xlsx","ppt","pptx"] },
        { name: "PDF",        extensions: ["pdf"] },
        { name: "Word",       extensions: ["doc","docx"] },
        { name: "Excel",      extensions: ["xls","xlsx"] },
        { name: "PowerPoint", extensions: ["ppt","pptx"] },
      ],
    });
    if (!path || typeof path !== "string") return;
    const appFile = await importFile(spaceId, path);
    const content = JSON.stringify({ stored_path: appFile.stored_path, name: appFile.name, file_type: appFile.file_type, size: appFile.size });
    const [w, h] = DEFAULT_SIZES.file;
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

  async function onMultiDragEnd(e: PointerEvent) {
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

  function updateConnectedLines(movedBlock: Block) {
    const cx = movedBlock.x + movedBlock.width / 2;
    const cy = movedBlock.y + movedBlock.height / 2;
    for (const b of blocks) {
      if (b.block_type !== "shape") continue;
      let d: any;
      try { d = JSON.parse(b.content); } catch { continue; }
      if (d.shape !== "line" && d.shape !== "arrow") continue;
      let changed = false;
      if (d.startId === movedBlock.id) { d.x1 = cx; d.y1 = cy; changed = true; }
      if (d.endId === movedBlock.id)   { d.x2 = cx; d.y2 = cy; changed = true; }
      if (!changed) continue;
      const newContent = JSON.stringify(d);
      const nx = Math.min(d.x1, d.x2), ny = Math.min(d.y1, d.y2);
      const nw = Math.max(Math.abs(d.x2-d.x1), 10), nh = Math.max(Math.abs(d.y2-d.y1), 10);
      blocks = blocks.map(bl => bl.id === b.id ? {...bl, content: newContent, x: nx, y: ny, width: nw, height: nh} : bl);
      updateBlockContent(b.id, newContent);
      updateBlockPosition(b.id, nx, ny, nw, nh);
    }
  }

  function clearStrokes() {
    pushHistory({ type: "strokes", before: strokes });
    strokes = []; redrawAll(); saveStrokes(pageId, "[]");
  }

  // Cursor for canvas area
  $: canvasBgCursor = drawMode
    ? (eraseMode ? "none" : "crosshair")
    : shapeMode ? "crosshair"
    : selectMode ? "crosshair"
    : (isPanning ? "grabbing" : "grab");
</script>

<!-- Eraser circle indicator (fixed, follows mouse, outside scaled canvas) -->
{#if drawMode && eraseMode && eraserVisible}
  <div
    class="eraser-indicator"
    style="left:{eraserScreenX}px; top:{eraserScreenY}px; width:{eraserScreenR * 2}px; height:{eraserScreenR * 2}px;"
  />
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
  >
    <div class="canvas-scaler" style="width:{CW * zoom}px; height:{CH * zoom}px">
      <div class="canvas-content" style="width:{CW}px; height:{CH}px; transform:scale({zoom}); transform-origin:top left">

        <canvas
          class="drawing-canvas"
          class:draw-active={drawMode}
          width={CW} height={CH}
          bind:this={canvasEl}
          on:pointerdown={onCanvasPointerDown}
          on:pointermove={onCanvasPointerMove}
          on:pointerup={onCanvasPointerUp}
          style="cursor:{drawMode ? (eraseMode ? 'none' : 'crosshair') : 'default'}"
        />

        <!-- Connector (line/arrow) drag preview -->
        {#if shapeDragging && isConnectorType(shapeType) && Math.hypot(shapeEndX-shapeStartX, shapeEndY-shapeStartY) > 4}
          <svg class="connector-preview-svg" style="position:absolute;left:0;top:0;width:{CW}px;height:{CH}px;overflow:visible;pointer-events:none;z-index:998;">
            <defs>
              {#if shapeType === "arrow"}
                <marker id="prev-arrow" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
                  <polygon points="0 0, 10 3.5, 0 7" fill={shapeStroke} />
                </marker>
              {/if}
            </defs>
            <line x1={shapeStartX} y1={shapeStartY} x2={shapeEndX} y2={shapeEndY}
                  stroke={shapeStroke} stroke-width={shapeStrokeWidth}
                  stroke-linecap="round" stroke-dasharray="8 5"
                  marker-end={shapeType === "arrow" ? "url(#prev-arrow)" : ""} />
            <!-- Start snap highlight -->
            {#if createStartSnapId}
              {@const snap = blocks.find(b => b.id === createStartSnapId)}
              {#if snap}
                <circle cx={snap.x + snap.width/2} cy={snap.y + snap.height/2} r="18"
                        fill="none" stroke={shapeStroke} stroke-width="2" stroke-dasharray="5 3" opacity="0.7" />
              {/if}
            {/if}
            <!-- End snap highlight -->
            {#if createEndSnapId}
              {@const snap = blocks.find(b => b.id === createEndSnapId)}
              {#if snap}
                <circle cx={snap.x + snap.width/2} cy={snap.y + snap.height/2} r="18"
                        fill="none" stroke={shapeStroke} stroke-width="2" stroke-dasharray="5 3" opacity="0.7" />
              {/if}
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
              {/if}
            </svg>
          </div>
        {/if}

        <!-- Rubber-band selection rect -->
        {#if bandRect && bandRect.w > 4 && bandRect.h > 4}
          <div class="band-select"
            style="left:{bandRect.x}px; top:{bandRect.y}px; width:{bandRect.w}px; height:{bandRect.h}px;"
          />
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
            />
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
              onUpdate={handleBlockUpdate}
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
              onUpdate={handleBlockUpdate}
              onDelete={handleBlockDelete}
              onBringToFront={bringToFront}
              onSelect={selectBlock}
            />
          {/if}
        {/each}
      </div>
    </div>
  </div>

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
    bind:zoom
    bind:eraserSize
    bind:selectMode
    {canUndo}
    editingShape={selectedShape !== null}
    onAddBlock={addBlock}
    onClearStrokes={clearStrokes}
    onUndo={undo}
  />
</div>

<style>
  .page-root { display: flex; flex-direction: column; flex: 1; overflow: hidden; }

  .canvas-viewport {
    flex: 1; overflow: auto;
    background-color: var(--bg-base);
    background-image: radial-gradient(circle, var(--border) 1px, transparent 1px);
    background-size: 28px 28px;
    /* Hide scrollbars */
    scrollbar-width: none;
    /* Prevent native browser touch zoom/scroll so we handle it ourselves */
    touch-action: none;
  }
  .canvas-viewport::-webkit-scrollbar { display: none; }

  .canvas-scaler { position: relative; }
  .canvas-content { position: relative; }

  .drawing-canvas {
    position: absolute; inset: 0;
    z-index: 0; pointer-events: none; touch-action: none;
  }
  .drawing-canvas.draw-active { z-index: 100; pointer-events: all; }

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
    position: absolute; cursor: grab;
    border: 2px dashed var(--accent);
    background: rgba(99,102,241,0.05);
    border-radius: 8px;
  }
  .multi-drag-overlay:active { cursor: grabbing; }

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
