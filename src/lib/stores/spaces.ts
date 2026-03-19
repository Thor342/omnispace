import { writable } from "svelte/store";
import type { Space, Page, Category } from "../types";

export const spaces     = writable<Space[]>([]);
export const categories = writable<Category[]>([]);
export const activeSpaceId = writable<string | null>(null);

// Pages keyed by spaceId
export const pagesMap   = writable<Record<string, Page[]>>({});
export const activePageId = writable<string | null>(null);
