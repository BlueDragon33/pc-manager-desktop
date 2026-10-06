import { invoke } from "@tauri-apps/api/core";

export interface PercentMetric {
  available: boolean;
  value: number | null;
  reason: string | null;
}

export interface MemoryMetric {
  available: boolean;
  totalBytes: number | null;
  usedBytes: number | null;
  availableBytes: number | null;
  usedPercent: number | null;
  reason: string | null;
}

export interface IoMetric {
  available: boolean;
  readBytesPerSec: number | null;
  writeBytesPerSec: number | null;
  reason: string | null;
}

export interface NetworkMetric {
  available: boolean;
  receiveBytesPerSec: number | null;
  sendBytesPerSec: number | null;
  reason: string | null;
}

export interface GpuMetric {
  available: boolean;
  utilizationPercent: number | null;
  metricLabel: string;
  reason: string | null;
}

export interface SensorMetric {
  name: string;
  available: boolean;
  valueCelsius: number | null;
  reason: string | null;
}

export interface ProcessUsage {
  pid: number;
  name: string;
  cpuPercent: number | null;
  memoryBytes: number | null;
}

export interface MonitorSnapshot {
  collectedAtEpochMs: number;
  sampleDurationMs: number;
  cpu: PercentMetric;
  memory: MemoryMetric;
  disk: IoMetric;
  network: NetworkMetric;
  gpu: GpuMetric;
  sensors: SensorMetric[];
  topProcesses: ProcessUsage[];
}

export interface MonitorHistoryPoint {
  at: number;
  cpu: number | null;
  memory: number | null;
}

export const MONITOR_INTERVALS_MS = [2000, 5000, 10000] as const;
export const MAX_MONITOR_HISTORY = 30;

export async function getMonitorSnapshot(): Promise<MonitorSnapshot> {
  return invoke<MonitorSnapshot>("get_monitor_snapshot");
}

export function normalizeMonitorInterval(value: number): number {
  return MONITOR_INTERVALS_MS.includes(
    value as (typeof MONITOR_INTERVALS_MS)[number],
  )
    ? value
    : 5000;
}

export function appendMonitorHistory(
  history: MonitorHistoryPoint[],
  snapshot: MonitorSnapshot,
): MonitorHistoryPoint[] {
  return [
    ...history,
    {
      at: snapshot.collectedAtEpochMs,
      cpu: snapshot.cpu.available ? snapshot.cpu.value : null,
      memory: snapshot.memory.available ? snapshot.memory.usedPercent : null,
    },
  ].slice(-MAX_MONITOR_HISTORY);
}

export function shouldScheduleMonitor(paused: boolean): boolean {
  return !paused;
}

export function formatBytes(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return "Unavailable";
  if (value < 1024) return `${Math.round(value)} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let scaled = value / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && scaled >= 1024; index += 1) {
    scaled /= 1024;
    unit = units[index];
  }
  return `${scaled >= 100 ? scaled.toFixed(0) : scaled.toFixed(1)} ${unit}`;
}

export function formatRate(value: number | null): string {
  const formatted = formatBytes(value);
  return formatted === "Unavailable" ? formatted : `${formatted}/s`;
}

export function formatPercent(value: number | null): string {
  return value === null || !Number.isFinite(value)
    ? "Unavailable"
    : `${value.toFixed(1)}%`;
}
