import { describe, expect, it } from "vitest";

import {
  appendMonitorHistory,
  MAX_MONITOR_HISTORY,
  normalizeMonitorInterval,
  shouldScheduleMonitor,
  type MonitorSnapshot,
} from "./monitor";

function snapshot(at: number): MonitorSnapshot {
  return {
    collectedAtEpochMs: at,
    sampleDurationMs: 10,
    cpu: { available: true, value: 25, reason: null },
    memory: {
      available: true,
      totalBytes: 100,
      usedBytes: 50,
      availableBytes: 50,
      usedPercent: 50,
      reason: null,
    },
    disk: {
      available: false,
      readBytesPerSec: null,
      writeBytesPerSec: null,
      reason: "Unavailable",
    },
    network: {
      available: false,
      receiveBytesPerSec: null,
      sendBytesPerSec: null,
      reason: "Unavailable",
    },
    gpu: {
      available: false,
      utilizationPercent: null,
      metricLabel: "GPU activity",
      reason: "Unavailable",
    },
    sensors: [],
    topProcesses: [],
  };
}

describe("System Monitor helpers", () => {
  it("accepts only conservative supported polling intervals", () => {
    expect(normalizeMonitorInterval(2000)).toBe(2000);
    expect(normalizeMonitorInterval(10000)).toBe(10000);
    expect(normalizeMonitorInterval(250)).toBe(5000);
  });

  it("does not schedule polling while paused", () => {
    expect(shouldScheduleMonitor(true)).toBe(false);
    expect(shouldScheduleMonitor(false)).toBe(true);
  });

  it("bounds recent history", () => {
    let history = [];
    for (let index = 0; index < MAX_MONITOR_HISTORY + 5; index += 1) {
      history = appendMonitorHistory(history, snapshot(index));
    }
    expect(history).toHaveLength(MAX_MONITOR_HISTORY);
    expect(history[0]?.at).toBe(5);
  });
});
