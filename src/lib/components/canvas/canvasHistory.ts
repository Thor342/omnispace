import type { Block, StrokePath } from "../../types";

export type HistoryEntry =
  | { type: "strokes"; before: StrokePath[] }
  | { type: "block_deleted"; block: Block }
  | { type: "block_added"; blockId: string }
  | { type: "block_moved"; blockId: string; x: number; y: number; w: number; h: number };

export const MAX_HISTORY = 50;

export function pushHistory(stack: HistoryEntry[], entry: HistoryEntry): HistoryEntry[] {
  return [...stack.slice(-(MAX_HISTORY - 1)), entry];
}
