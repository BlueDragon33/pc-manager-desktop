import { invoke } from "@tauri-apps/api/core";

export type HealthCategory =
  | "storage"
  | "performance"
  | "security"
  | "updates"
  | "privacy";

export type HealthStatus = "good" | "attention" | "critical" | "unavailable";
export type HealthSeverity = "info" | "warning" | "critical";
export type RiskLevel = "none" | "low" | "medium" | "high";

export interface HealthFinding {
  id: string;
  category: HealthCategory;
  title: string;
  description: string;
  evidence: string;
  severity: HealthSeverity;
  risk: RiskLevel;
  recommendedAction: string;
  actionAvailable: boolean;
  reversible: boolean;
}

export interface HealthCategoryResult {
  category: HealthCategory;
  status: HealthStatus;
  score: number | null;
  summary: string;
}

export interface HealthReport {
  scanId: string;
  collectedAtEpochMs: number;
  score: number | null;
  supportedCategories: number;
  totalCategories: number;
  coveragePercent: number;
  categoryResults: HealthCategoryResult[];
  findings: HealthFinding[];
  inventoryWarnings: Array<{
    source: string;
    message: string;
  }>;
}

const LATEST_REPORT_KEY = "pc-manager.health.latest.v1";

export async function runHealthCheck(): Promise<HealthReport> {
  return invoke<HealthReport>("run_health_check");
}

export function loadLatestHealthReport(): HealthReport | null {
  const raw = window.localStorage.getItem(LATEST_REPORT_KEY);
  if (!raw) {
    return null;
  }

  try {
    const parsed: unknown = JSON.parse(raw);
    return isHealthReport(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function saveLatestHealthReport(report: HealthReport): void {
  window.localStorage.setItem(LATEST_REPORT_KEY, JSON.stringify(report));
}

export function formatHealthCategory(category: HealthCategory): string {
  return category[0].toUpperCase() + category.slice(1);
}

export function isHealthReport(value: unknown): value is HealthReport {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as Partial<HealthReport>;

  return (
    typeof candidate.scanId === "string" &&
    typeof candidate.collectedAtEpochMs === "number" &&
    (typeof candidate.score === "number" || candidate.score === null) &&
    typeof candidate.supportedCategories === "number" &&
    typeof candidate.totalCategories === "number" &&
    typeof candidate.coveragePercent === "number" &&
    Array.isArray(candidate.categoryResults) &&
    Array.isArray(candidate.findings) &&
    Array.isArray(candidate.inventoryWarnings)
  );
}
