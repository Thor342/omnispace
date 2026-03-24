import type { StrokePath } from "../../types";

export const CW = 3200;
export const CH = 2400;

export function renderStroke(c: CanvasRenderingContext2D, s: StrokePath) {
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

export function redrawAll(
  ctx: CanvasRenderingContext2D | null,
  markerCtx: CanvasRenderingContext2D | null,
  strokes: StrokePath[]
) {
  if (ctx) {
    ctx.clearRect(0, 0, CW, CH);
    for (const s of strokes) if ((s.layer ?? "base") === "base") renderStroke(ctx, s);
  }
  if (markerCtx) {
    markerCtx.clearRect(0, 0, CW, CH);
    for (const s of strokes) if (s.layer === "top") renderStroke(markerCtx, s);
  }
}

/** Devuelve los trazos resultantes tras borrar en (x,y) y si hubo cambios. */
export function eraseAt(
  x: number, y: number,
  strokes: StrokePath[],
  eraserSize: number
): { strokes: StrokePath[]; changed: boolean } {
  const r = eraserSize;
  let changed = false;
  const result: StrokePath[] = [];

  for (const stroke of strokes) {
    const hasHit = stroke.points.some(pt => Math.hypot(pt[0] - x, pt[1] - y) < r);
    if (!hasHit) { result.push(stroke); continue; }
    changed = true;
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

  return { strokes: result, changed };
}

/** Convierte coordenadas de puntero (screen) a coordenadas del canvas. */
export function getCanvasPos(
  e: PointerEvent,
  canvasEl: HTMLCanvasElement
): [number, number] {
  const rect = canvasEl.getBoundingClientRect();
  return [
    (e.clientX - rect.left) * (CW / rect.width),
    (e.clientY - rect.top)  * (CH / rect.height),
  ];
}
