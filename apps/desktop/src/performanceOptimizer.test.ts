import { describe, expect, it } from "vitest";

import {
  findingKindLabel,
  formatOptimizerPercent,
  sortPerformanceFindings,
  type PerformanceFinding,
} from "./performanceOptimizer";

describe("performance optimizer UI helpers", () => {
  it("formats unavailable evidence honestly", () => {
    expect(formatOptimizerPercent(null)).toBe("Unavailable");
    expect(formatOptimizerPercent(12.34)).toBe("12.3%");
  });

  it("sorts stronger evidence first without changing the input", () => {
    const findings = [
      { id: "low", risk: "low" },
      { id: "medium", risk: "medium" },
    ] as PerformanceFinding[];
    const sorted = sortPerformanceFindings(findings);
    expect(sorted.map((item) => item.id)).toEqual(["medium", "low"]);
    expect(findings.map((item) => item.id)).toEqual(["low", "medium"]);
  });

  it("uses restrained labels", () => {
    expect(findingKindLabel("service")).toBe("Service review");
    expect(findingKindLabel("persistentProcess")).toBe("Persistent process");
  });
});
