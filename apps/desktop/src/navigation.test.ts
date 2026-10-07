import { describe, expect, it } from "vitest";

import { NAV_ITEMS, getNavItem, isPageId } from "./navigation";

describe("navigation model", () => {
  it("contains each required destination exactly once", () => {
    const ids = NAV_ITEMS.map((item) => item.id);

    expect(ids).toEqual([
      "overview",
      "health",
      "clean",
      "startup",
      "apps",
      "updates",
      "storage",
      "duplicates",
      "monitor",
      "restore",
      "settings",
    ]);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("looks up known pages and rejects unknown IDs", () => {
    expect(getNavItem("storage").label).toBe("Storage");
    expect(isPageId("monitor")).toBe(true);
    expect(isPageId("anything-else")).toBe(false);
  });
});
