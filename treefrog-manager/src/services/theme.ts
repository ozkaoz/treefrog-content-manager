// Centralized theme tokens — follows Windows system appearance via prefers-color-scheme.
// No custom light/dark toggle is the primary mechanism; this mirrors the OS.

export type Theme = "light" | "dark";

export function getSystemTheme(): Theme {
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "light";
}

export function watchSystemTheme(cb: (theme: Theme) => void): () => void {
  if (typeof window === "undefined" || !window.matchMedia) return () => {};
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = (e: MediaQueryListEvent) => cb(e.matches ? "dark" : "light");
  // Modern browsers
  if (mq.addEventListener) {
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }
  // Safari fallback
  // @ts-ignore
  mq.addListener(handler);
  return () => {
    // @ts-ignore
    mq.removeListener(handler);
  };
}

// Apply theme to document as data attribute for CSS variables + manual overrides
export function applyTheme(theme: Theme) {
  document.documentElement.setAttribute("data-theme", theme);
  // Also set color-scheme for native form controls
  document.documentElement.style.colorScheme = theme;
}

export function initTheme(): () => void {
  const t = getSystemTheme();
  applyTheme(t);
  return watchSystemTheme(applyTheme);
}
