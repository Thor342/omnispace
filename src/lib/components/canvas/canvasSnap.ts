import type { Block } from "../../types";

export const SNAP_R = 40;
const PORTS = ["n", "s", "e", "w", "center"] as const;

export type SnapResult = { id: string; x: number; y: number; port: string } | null;

export function isConnectorType(t: string): boolean {
  return t === "line" || t === "arrow";
}

export function getPortCoords(b: Block, port: string): [number, number] {
  const cx = b.x + b.width / 2, cy = b.y + b.height / 2;
  switch (port) {
    case "n": return [cx, b.y];
    case "s": return [cx, b.y + b.height];
    case "e": return [b.x + b.width, cy];
    case "w": return [b.x, cy];
    default:  return [cx, cy];
  }
}

export function findSnapTarget(cx: number, cy: number, blocks: Block[]): SnapResult {
  let best: SnapResult = null;
  let bestDist = SNAP_R;
  for (const b of blocks) {
    if (b.block_type === "shape") {
      try { const d = JSON.parse(b.content); if (isConnectorType(d.shape)) continue; } catch {}
    }
    for (const port of PORTS) {
      const [px, py] = getPortCoords(b, port);
      const dist = Math.hypot(px - cx, py - cy);
      if (dist < bestDist) { bestDist = dist; best = { id: b.id, x: px, y: py, port }; }
    }
  }
  return best;
}
