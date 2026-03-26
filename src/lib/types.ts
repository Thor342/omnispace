export interface Space {
  id: string;
  name: string;
  icon: string;
  color: string;
  category_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface Category {
  id: string;
  name: string;
  icon: string;
  color: string;
  order_index: number;
  created_at: string;
  updated_at: string;
}

export interface Page {
  id: string;
  space_id: string;
  title: string;
  order_index: number;
  created_at: string;
  updated_at: string;
}

export type BlockType = "note" | "link" | "file" | "task" | "calendar" | "clock" | "shape";

export interface Block {
  id: string;
  page_id: string;
  block_type: BlockType;
  x: number;
  y: number;
  width: number;
  height: number;
  content: string; // JSON string
  z_index: number;
  created_at: string;
  updated_at: string;
}

// ─── Block content shapes ────────────────────────────────
export interface NoteContent   { title: string; text?: string; html?: string; }
export type LinkType = "youtube" | "canva" | "figma" | "gslides" | "gdocs" | "miro" | "loom" | "general";
export interface LinkContent {
  url: string;
  title: string;
  link_type: LinkType;
  og_title?: string;
  og_description?: string;
  og_image?: string;
  og_site?: string;
}
export interface FileContent   { stored_path: string; name: string; file_type: "image" | "pdf" | "video" | "audio" | "word" | "excel" | "powerpoint" | "other"; size: number; word_html?: string; }
export interface TaskItem      { id: string; title: string; completed: boolean; }
export interface TaskContent   { tasks: TaskItem[]; }
export interface CalendarContent { note?: string; }
export interface ClockContent  { mode: "clock" | "timer" | "stopwatch"; }
export interface ShapeContent  { shape: "rect" | "circle" | "triangle" | "diamond"; fill: string; stroke: string; text: string; }

// ─── Strokes ─────────────────────────────────────────────
export interface StrokePath {
  color: string;
  width: number;
  points: [number, number][];
  layer?: "base" | "top"; // "base" = lápiz (detrás de bloques), "top" = marcador (encima de todo)
  opacity?: number; // 1-100, 100 = opaco, menor = más transparente
}

// ─── Notes ───────────────────────────────────────────────
export interface Note {
  id: string;
  space_id: string;
  title: string;
  content: string;
  created_at: string;
  updated_at: string;
}

// ─── Tasks ───────────────────────────────────────────────
export interface Task {
  id: string;
  space_id: string;
  title: string;
  completed: boolean;
  order_index: number;
  created_at: string;
  updated_at: string;
}

// ─── Links ───────────────────────────────────────────────
export interface Link {
  id: string;
  space_id: string;
  title: string;
  url: string;
  link_type: string;
  created_at: string;
}

// ─── App file (for import) ───────────────────────────────
export interface AppFile {
  id: string;
  space_id: string;
  name: string;
  original_path: string;
  stored_path: string;
  file_type: string;
  size: number;
  created_at: string;
}
