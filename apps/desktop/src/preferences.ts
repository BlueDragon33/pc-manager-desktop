export type ThemePreference = "system" | "dark" | "light";

const THEME_KEY = "pc-manager.theme";
const SIDEBAR_KEY = "pc-manager.sidebar-collapsed";
const MANAGED_MODE_KEY = "pc-manager.managed-enabled";

export function readThemePreference(): ThemePreference {
  const value = window.localStorage.getItem(THEME_KEY);
  return value === "dark" || value === "light" || value === "system"
    ? value
    : "system";
}

export function writeThemePreference(value: ThemePreference): void {
  window.localStorage.setItem(THEME_KEY, value);
}

export function readSidebarCollapsed(): boolean {
  return window.localStorage.getItem(SIDEBAR_KEY) === "true";
}

export function writeSidebarCollapsed(value: boolean): void {
  window.localStorage.setItem(SIDEBAR_KEY, String(value));
}

export function resolveTheme(
  preference: ThemePreference,
  prefersDark: boolean,
): "dark" | "light" {
  if (preference === "system") {
    return prefersDark ? "dark" : "light";
  }

  return preference;
}

/**
 * Central management is opt-in. An inaccessible or unrecognized local state
 * always defaults to standalone/offline, never implicitly enabling sync.
 */
export function readManagedModeEnabled(): boolean {
  try {
    return window.localStorage.getItem(MANAGED_MODE_KEY) === "true";
  } catch {
    return false;
  }
}

/** Do not enable optional networking if the local preference cannot persist. */
export function writeManagedModeEnabled(enabled: boolean): boolean {
  try {
    window.localStorage.setItem(MANAGED_MODE_KEY, String(enabled));
    return true;
  } catch {
    return false;
  }
}
