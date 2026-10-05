import type { CleanupExecutionResult } from "./smartClean";

const OPERATION_HISTORY_KEY = "pc-manager.cleanup.operations.v1";
const MAX_OPERATIONS = 30;

export function loadCleanupOperations(): CleanupExecutionResult[] {
  const raw = globalThis.localStorage?.getItem(OPERATION_HISTORY_KEY);
  if (!raw) {
    return [];
  }

  try {
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed.filter(isCleanupExecutionResult).slice(0, MAX_OPERATIONS)
      : [];
  } catch {
    return [];
  }
}

export function saveCleanupOperation(operation: CleanupExecutionResult): void {
  const next = [
    operation,
    ...loadCleanupOperations().filter(
      (entry) => entry.operationId !== operation.operationId,
    ),
  ].slice(0, MAX_OPERATIONS);

  globalThis.localStorage?.setItem(OPERATION_HISTORY_KEY, JSON.stringify(next));
}

function isCleanupExecutionResult(
  value: unknown,
): value is CleanupExecutionResult {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as Partial<CleanupExecutionResult>;
  return (
    typeof candidate.operationId === "string" &&
    typeof candidate.planId === "string" &&
    typeof candidate.startedAtEpochMs === "number" &&
    typeof candidate.completedAtEpochMs === "number" &&
    typeof candidate.requestedFiles === "number" &&
    typeof candidate.requestedBytes === "number" &&
    typeof candidate.deletedFiles === "number" &&
    typeof candidate.deletedBytes === "number" &&
    typeof candidate.failedItems === "number" &&
    typeof candidate.rollbackAvailable === "boolean" &&
    typeof candidate.rollbackSummary === "string" &&
    Array.isArray(candidate.providers) &&
    Array.isArray(candidate.errors)
  );
}
