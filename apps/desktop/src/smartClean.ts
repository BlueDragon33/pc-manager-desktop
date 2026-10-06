import { invoke } from "@tauri-apps/api/core";

export type CleanupCategory =
  "windowsTemp" | "browserCache" | "applicationCache" | "recycleBin";

export interface CleanupWarning {
  providerId: string;
  message: string;
}

export interface CleanupProviderSummary {
  providerId: string;
  displayName: string;
  category: CleanupCategory;
  enabledByDefault: boolean;
  available: boolean;
  fileCount: number;
  bytes: number;
  reversible: boolean;
  description: string;
  warnings: string[];
}

export interface CleanupScanSummary {
  planId: string;
  collectedAtEpochMs: number;
  totalBytes: number;
  totalFiles: number;
  providers: CleanupProviderSummary[];
  warnings: CleanupWarning[];
  executionAvailable: boolean;
}

export interface CleanupScanOptions {
  includeRecycleBin: boolean;
}

export async function scanCleanupCandidates(
  options: CleanupScanOptions,
): Promise<CleanupScanSummary> {
  return invoke<CleanupScanSummary>("scan_cleanup_candidates", { options });
}

export function formatCleanupCategory(category: CleanupCategory): string {
  switch (category) {
    case "windowsTemp":
      return "Windows temp";
    case "browserCache":
      return "Browser cache";
    case "applicationCache":
      return "Application cache";
    case "recycleBin":
      return "Recycle Bin";
  }
}

export function isCurrentScan(generation: number, current: number): boolean {
  return generation === current;
}


export interface CleanupExecutionError {
  providerId: string;
  code: string;
  message: string;
}

export interface CleanupProviderExecutionResult {
  providerId: string;
  requestedFiles: number;
  requestedBytes: number;
  deletedFiles: number;
  deletedBytes: number;
  failedFiles: number;
}

export interface CleanupOperationRecord {
  operationId: string;
  planId: string;
  startedAtEpochMs: number;
  completedAtEpochMs: number;
  requestedFiles: number;
  requestedBytes: number;
  deletedFiles: number;
  deletedBytes: number;
  failedFiles: number;
  providerResults: CleanupProviderExecutionResult[];
  rollbackAvailable: boolean;
  rollbackLabel: string;
  errors: CleanupExecutionError[];
}

export async function executeCleanupPlan(
  planId: string,
): Promise<CleanupOperationRecord> {
  return invoke<CleanupOperationRecord>("execute_cleanup_plan", { planId });
}

export async function listCleanupOperations(): Promise<
  CleanupOperationRecord[]
> {
  return invoke<CleanupOperationRecord[]>("list_cleanup_operations");
}
