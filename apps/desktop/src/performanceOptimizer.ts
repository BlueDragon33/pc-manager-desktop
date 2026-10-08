import { invoke } from "@tauri-apps/api/core";

export type PerformanceFindingKind =
  "persistentProcess" | "startupImpact" | "scheduledTask" | "service";

export type PerformanceAction =
  | "reviewStartup"
  | "reviewAppSettings"
  | "reviewScheduledTask"
  | "reviewService"
  | "none";

export type RiskLevel = "none" | "low" | "medium" | "high";

export interface PerformanceFinding {
  id: string;
  kind: PerformanceFindingKind;
  title: string;
  summary: string;
  evidence: string[];
  risk: RiskLevel;
  recommendation: string;
  action: PerformanceAction;
}

export interface PerformanceOptimizerReport {
  generatedAtEpochMs: number;
  sampleCount: number;
  sampleWindowMs: number;
  averageCpuPercent: number | null;
  averageMemoryPercent: number | null;
  startupEntriesAnalyzed: number;
  servicesAnalyzed: number;
  findings: PerformanceFinding[];
  limitations: string[];
}

export async function runPerformanceOptimizer(): Promise<PerformanceOptimizerReport> {
  return invoke<PerformanceOptimizerReport>("run_performance_optimizer");
}

export function findingKindLabel(kind: PerformanceFindingKind): string {
  switch (kind) {
    case "persistentProcess":
      return "Persistent process";
    case "startupImpact":
      return "Startup evidence";
    case "scheduledTask":
      return "Scheduled task";
    case "service":
      return "Service review";
  }
}

export function formatOptimizerPercent(value: number | null): string {
  return value === null || !Number.isFinite(value)
    ? "Unavailable"
    : `${value.toFixed(1)}%`;
}

export function sortPerformanceFindings(
  findings: PerformanceFinding[],
): PerformanceFinding[] {
  const rank: Record<RiskLevel, number> = {
    high: 3,
    medium: 2,
    low: 1,
    none: 0,
  };
  return [...findings].sort((a, b) => rank[b.risk] - rank[a.risk]);
}
