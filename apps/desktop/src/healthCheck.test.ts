import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  isHealthReport,
  loadLatestHealthReport,
  saveLatestHealthReport,
  type HealthReport,
} from "./healthCheck";

const report: HealthReport = {
  scanId: "health-1-test",
  collectedAtEpochMs: 1,
  score: 90,
  supportedCategories: 2,
  totalCategories: 5,
  coveragePercent: 40,
  categoryResults: [],
  findings: [],
  inventoryWarnings: [],
};

describe("health report persistence", () => {
  beforeEach(() => {
    const store = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, value: string) => store.set(key, value),
    });
  });

  it("restores a valid completed report", () => {
    saveLatestHealthReport(report);
    expect(loadLatestHealthReport()).toEqual(report);
  });

  it("rejects malformed persisted data", () => {
    globalThis.localStorage.setItem("pc-manager.health.latest.v1", "{\"bad\":true}");
    expect(loadLatestHealthReport()).toBeNull();
  });

  it("validates the minimum report shape", () => {
    expect(isHealthReport(report)).toBe(true);
    expect(isHealthReport({ scanId: "missing-fields" })).toBe(false);
  });
});
