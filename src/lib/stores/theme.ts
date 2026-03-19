import { writable } from "svelte/store";

const stored = (typeof localStorage !== "undefined" && localStorage.getItem("theme")) || "dark";

export const theme = writable<"dark" | "light">(stored as "dark" | "light");

theme.subscribe((val) => {
  if (typeof document === "undefined") return;
  document.documentElement.classList.toggle("light", val === "light");
  localStorage.setItem("theme", val);
});
