import { describe, expect, it } from "vitest";

import { formatCleanupCategory, isCurrentScan } from "./smartClean";

describe("Smart Clean helpers", () => {
  it("labels provider categories clearly", () => {
    expect(formatCleanupCategory("windowsTemp")).toBe("Windows temp");
    expect(formatCleanupCategory("browserCache")).toBe("Browser cache");
    expect(formatCleanupCategory("applicationCache")).toBe("Application cache");
    expect(formatCleanupCategory("recycleBin")).toBe("Recycle Bin");
  });

  it("discards stale scan generations after cancellation", () => {
    expect(isCurrentScan(4, 4)).toBe(true);
    expect(isCurrentScan(4, 5)).toBe(false);
  });
});
