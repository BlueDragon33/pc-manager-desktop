import { describe, expect, it } from "vitest";

import { formatBytes } from "./systemInventory";

describe("formatBytes", () => {
  it("formats useful binary size units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1024 ** 3)).toBe("1.0 GB");
  });

  it("handles invalid values safely", () => {
    expect(formatBytes(Number.NaN)).toBe("0 B");
    expect(formatBytes(-10)).toBe("0 B");
  });
});
