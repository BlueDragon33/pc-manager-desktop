import { describe, expect, it } from "vitest";

import {
  filterStartupEntries,
  sourceTypeLabel,
  type StartupEntry,
} from "./startup";

const entries: StartupEntry[] = [
  {
    id: "startup-a",
    name: "Example Sync",
    sourceType: "registryCurrentUserRun",
    sourceLabel: "Current user Run",
    command: "C:\\Example\\sync.exe",
    publisher: "Example Ltd",
    enabled: true,
    canChange: true,
    requiresElevation: false,
    impact: "unknown",
    impactEvidence: null,
    detail: "Current user Run",
  },
  {
    id: "startup-b",
    name: "Build Agent",
    sourceType: "scheduledTask",
    sourceLabel: "Scheduled Task",
    command: "agent.exe",
    publisher: null,
    enabled: false,
    canChange: true,
    requiresElevation: false,
    impact: "unknown",
    impactEvidence: null,
    detail: "Scheduled Task",
  },
];

describe("Startup Manager helpers", () => {
  it("filters by name, source, command, or publisher", () => {
    expect(filterStartupEntries(entries, "sync")).toHaveLength(1);
    expect(filterStartupEntries(entries, "scheduled")).toHaveLength(1);
    expect(filterStartupEntries(entries, "example ltd")).toHaveLength(1);
    expect(filterStartupEntries(entries, "")).toHaveLength(2);
  });

  it("labels supported source types", () => {
    expect(sourceTypeLabel("registryCurrentUserRun")).toBe("User Run");
    expect(sourceTypeLabel("scheduledTask")).toBe("Scheduled Task");
  });
});
