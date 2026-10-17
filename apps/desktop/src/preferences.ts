export type ThemePreference = "system" | "dark" | "light";

const THEME_KEY = "pc-manager.theme";
const SIDEBAR_KEY = "pc-manager.sidebar-collapsed";
const MANAGED_MODE_KEY = "pc-manager.managed-enabled";

export function readThemePreference(): ThemePreference {
  try {
    const value = window.localStorage.getItem(THEME_KEY);
    return value === "dark" || value === "light" || value === "system"
      ? value
      : "system";
  } catch {
    return "system";
  }
}

/** Report storage denial instead of showing a non-persistent preference. */
export function writeThemePreference(value: ThemePreference): boolean {
  try {
    window.localStorage.setItem(THEME_KEY, value);
    return window.localStorage.getItem(THEME_KEY) === value;
  } catch {
    return false;
  }
}

export function readSidebarCollapsed(): boolean {
  try {
    return window.localStorage.getItem(SIDEBAR_KEY) === "true";
  } catch {
    return false;
  }
}

/** Fail safely if the setting cannot be read back after writing. */
export function writeSidebarCollapsed(value: boolean): boolean {
  try {
    window.localStorage.setItem(SIDEBAR_KEY, String(value));
    return window.localStorage.getItem(SIDEBAR_KEY) === String(value);
  } catch {
    return false;
  }
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
    return window.localStorage.getItem(MANAGED_MODE_KEY) === String(enabled);
  } catch {
    return false;
  }
}
