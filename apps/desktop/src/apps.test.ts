import { describe, expect, it } from "vitest";

import {
  filterInstalledApps,
  sourceLabel,
  type InstalledAppEntry,
} from "./apps";

const entries: InstalledAppEntry[] = [
  {
    id: "app-a",
    displayName: "Example Editor",
    publisher: "Example Ltd",
    version: "2.0",
    installLocation: null,
    installDate: null,
    estimatedSizeBytes: null,
    source: "currentUser",
    canUninstall: true,
    requiresElevation: false,
    uninstallKind: "executable",
    detail: "Standard uninstall available.",
  },
  {
    id: "app-b",
    displayName: "Machine Tool",
    publisher: "Vendor",
    version: "1.0",
    installLocation: null,
    installDate: null,
    estimatedSizeBytes: null,
    source: "localMachine64",
    canUninstall: false,
    requiresElevation: true,
    uninstallKind: "unavailable",
    detail: "Read-only.",
  },
];

describe("Apps helpers", () => {
  it("filters by name, publisher, version, and source", () => {
    expect(filterInstalledApps(entries, "editor")).toHaveLength(1);
    expect(filterInstalledApps(entries, "vendor")).toHaveLength(1);
    expect(filterInstalledApps(entries, "2.0")).toHaveLength(1);
    expect(filterInstalledApps(entries, "64-bit")).toHaveLength(1);
    expect(filterInstalledApps(entries, "")).toHaveLength(2);
  });

  it("labels source scope clearly", () => {
    expect(sourceLabel("currentUser")).toBe("Current user");
    expect(sourceLabel("localMachine64")).toBe("Machine · 64-bit");
  });
});
