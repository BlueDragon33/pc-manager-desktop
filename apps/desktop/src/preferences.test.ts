import { afterEach, describe, expect, it, vi } from "vitest";

import {
  readManagedModeEnabled,
  readSidebarCollapsed,
  readThemePreference,
  resolveTheme,
  writeManagedModeEnabled,
  writeSidebarCollapsed,
  writeThemePreference,
} from "./preferences";

describe("theme resolution", () => {
  it("respects explicit theme choices", () => {
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("light", true)).toBe("light");
  });

  it("follows the operating-system preference in system mode", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });
});

describe("optional App Manager mode (offline first)", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("is OFF for a fresh installation and an invalid stored value", () => {
    const values = new Map<string, string>();
    vi.stubGlobal("window", {
      localStorage: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
      },
    });
    expect(readManagedModeEnabled()).toBe(false);
    values.set("pc-manager.managed-enabled", "maybe");
    expect(readManagedModeEnabled()).toBe(false);
  });

  it("persists explicit ON and OFF choices across reads", () => {
    const values = new Map<string, string>();
    vi.stubGlobal("window", {
      localStorage: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
      },
    });
    expect(writeManagedModeEnabled(true)).toBe(true);
    expect(readManagedModeEnabled()).toBe(true);
    expect(writeManagedModeEnabled(false)).toBe(true);
    expect(readManagedModeEnabled()).toBe(false);
  });

  it("fails closed if local storage is blocked or corrupted", () => {
    vi.stubGlobal("window", {
      localStorage: {
        getItem: () => {
          throw new Error("blocked");
        },
        setItem: () => {
          throw new Error("blocked");
        },
      },
    });
    expect(readManagedModeEnabled()).toBe(false);
    expect(writeManagedModeEnabled(true)).toBe(false);
    expect(readThemePreference()).toBe("system");
    expect(readSidebarCollapsed()).toBe(false);
  });
});

describe("offline appearance preference persistence", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("round-trips theme and compact sidebar choices", () => {
    const values = new Map<string, string>();
    vi.stubGlobal("window", {
      localStorage: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
      },
    });
    expect(writeThemePreference("dark")).toBe(true);
    expect(writeSidebarCollapsed(true)).toBe(true);
    expect(readThemePreference()).toBe("dark");
    expect(readSidebarCollapsed()).toBe(true);
    expect(writeThemePreference("light")).toBe(true);
    expect(writeSidebarCollapsed(false)).toBe(true);
    expect(readThemePreference()).toBe("light");
    expect(readSidebarCollapsed()).toBe(false);
  });

  it("does not claim persistence when storage denies writes", () => {
    const values = new Map<string, string>();
    vi.stubGlobal("window", {
      localStorage: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: () => {
          throw new Error("quota exceeded");
        },
      },
    });
    expect(writeThemePreference("dark")).toBe(false);
    expect(writeSidebarCollapsed(true)).toBe(false);
    expect(writeManagedModeEnabled(true)).toBe(false);
    expect(readThemePreference()).toBe("system");
    expect(readSidebarCollapsed()).toBe(false);
    expect(readManagedModeEnabled()).toBe(false);
  });

  it("does not trust silent storage writes without matching readback", () => {
    vi.stubGlobal("window", {
      localStorage: {
        getItem: () => null,
        setItem: () => undefined,
      },
    });
    expect(writeThemePreference("dark")).toBe(false);
    expect(writeSidebarCollapsed(true)).toBe(false);
    expect(writeManagedModeEnabled(true)).toBe(false);
  });
});
