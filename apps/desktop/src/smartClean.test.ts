import { describe, expect, it } from "vitest";

import {
  cleanupResultSummary,
  formatCleanupCategory,
  isCurrentScan,
  type CleanupOperationRecord,
} from "./smartClean";

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

  it("summarizes exact execution outcomes without hiding failures", () => {
    const operation: CleanupOperationRecord = {
      operationId: "op-1",
      planId: "plan-1",
      startedAtEpochMs: 1,
      completedAtEpochMs: 2,
      requestedFiles: 10,
      requestedBytes: 100,
      deletedFiles: 8,
      deletedBytes: 80,
      failedFiles: 2,
      providerResults: [],
      rollbackCapability: "notRestorable",
      issues: [],
    };

    expect(cleanupResultSummary(operation)).toContain("8 files");
    expect(cleanupResultSummary(operation)).toContain("2");
  });
});
