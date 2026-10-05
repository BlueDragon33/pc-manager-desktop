import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  loadCleanupOperations,
  saveCleanupOperation,
} from "./operationHistory";
import type { CleanupExecutionResult } from "./smartClean";

const result: CleanupExecutionResult = {
  operationId: "op-1",
  planId: "plan-1",
  startedAtEpochMs: 1,
  completedAtEpochMs: 2,
  requestedFiles: 2,
  requestedBytes: 20,
  deletedFiles: 1,
  deletedBytes: 10,
  failedItems: 1,
  providers: [],
  rollbackAvailable: false,
  rollbackSummary: "Not restorable",
  errors: ["one skipped item"],
};

describe("cleanup operation history", () => {
  beforeEach(() => {
    const store = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, value: string) => store.set(key, value),
    });
  });

  it("persists completed operation results", () => {
    saveCleanupOperation(result);
    expect(loadCleanupOperations()).toEqual([result]);
  });

  it("keeps operation IDs unique", () => {
    saveCleanupOperation(result);
    saveCleanupOperation(result);
    expect(loadCleanupOperations()).toHaveLength(1);
  });
});
