import { invoke } from "@tauri-apps/api/core";
import type { Space, Page, Block, BlockType, AppFile } from "./types";

// ─── Spaces ─────────────────────────────────────────────
export const getSpaces = () => invoke<Space[]>("get_spaces");
export const createSpace = (name: string, icon: string, color: string) =>
  invoke<Space>("create_space", { name, icon, color });
export const updateSpace = (id: string, name: string, icon: string, color: string) =>
  invoke<Space>("update_space", { id, name, icon, color });
export const deleteSpace = (id: string) => invoke<void>("delete_space", { id });

// ─── Pages ──────────────────────────────────────────────
export const getPages = (spaceId: string) =>
  invoke<Page[]>("get_pages", { spaceId });
export const createPage = (spaceId: string, title: string) =>
  invoke<Page>("create_page", { spaceId, title });
export const updatePageTitle = (id: string, title: string) =>
  invoke<void>("update_page_title", { id, title });
export const reorderPages = (pageIds: string[]) =>
  invoke<void>("reorder_pages", { pageIds });
export const deletePage = (id: string) => invoke<void>("delete_page", { id });
export const exportPage = (pageId: string, destDir: string) =>
  invoke<string>("export_page", { pageId, destDir });
export const importPage = (spaceId: string, importDir: string) =>
  invoke<import("./types").Page>("import_page", { spaceId, importDir });

// ─── Blocks ─────────────────────────────────────────────
export const getBlocks = (pageId: string) =>
  invoke<Block[]>("get_blocks", { pageId });
export const createBlock = (
  pageId: string, blockType: BlockType,
  x: number, y: number, width: number, height: number,
  content: string
) => invoke<Block>("create_block", { pageId, blockType, x, y, width, height, content });
export const updateBlockPosition = (id: string, x: number, y: number, width: number, height: number) =>
  invoke<void>("update_block_position", { id, x, y, width, height });
export const updateBlockContent = (id: string, content: string) =>
  invoke<void>("update_block_content", { id, content });
export const updateBlockZindex = (id: string, zIndex: number) =>
  invoke<void>("update_block_zindex", { id, zIndex });
export const deleteBlock = (id: string) => invoke<void>("delete_block", { id });
export const restoreBlock = (block: Block) => invoke<void>("restore_block", { block });

// ─── Strokes ────────────────────────────────────────────
export const getStrokes = (pageId: string) =>
  invoke<string>("get_strokes", { pageId });
export const saveStrokes = (pageId: string, data: string) =>
  invoke<void>("save_strokes", { pageId, data });

// ─── Categories ─────────────────────────────────────────
export const getCategories = () => invoke<import("./types").Category[]>("get_categories");
export const createCategory = (name: string, icon: string, color: string) =>
  invoke<import("./types").Category>("create_category", { name, icon, color });
export const renameCategory = (id: string, name: string, icon: string, color: string) =>
  invoke<void>("rename_category", { id, name, icon, color });
export const deleteCategory = (id: string) => invoke<void>("delete_category", { id });
export const assignSpaceToCategory = (spaceId: string, categoryId: string | null) =>
  invoke<void>("assign_space_to_category", { spaceId, categoryId });

// ─── Files ──────────────────────────────────────────────
export const importFile = (spaceId: string, sourcePath: string) =>
  invoke<AppFile>("import_file", { spaceId, sourcePath });
export const deleteFile = (id: string) => invoke<void>("delete_file", { id });
export const readFileAsBase64 = (path: string) =>
  invoke<string>("read_file_as_base64", { path });
export const fetchOgMeta = (url: string) =>
  invoke<{ title: string; description: string; image: string; site_name: string }>("fetch_og_meta", { url });
