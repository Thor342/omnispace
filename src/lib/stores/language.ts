import { derived, writable } from "svelte/store";
import { translations, type Lang } from "../i18n/translations";

const stored = (typeof localStorage !== "undefined" && localStorage.getItem("language")) as Lang | null;
const validLangs: Lang[] = ["es", "en", "pt", "fr", "de", "it"];
const initial: Lang = stored && validLangs.includes(stored) ? stored : "es";

export const language = writable<Lang>(initial);

language.subscribe((val) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("language", val);
  }
});

export const t = derived(language, ($lang) => translations[$lang]);
