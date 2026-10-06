import { invoke } from "@tauri-apps/api/core";

export type StartupSourceType =
  | "registryCurrentUserRun"
  | "registryCurrentUserRunOnce"
  | "registryLocalMachineRun"
  | "registryLocalMachineRunOnce"
  | "userStartupFolder"
  | "commonStartupFolder"
  | "scheduledTask";

export type StartupImpact = "unknown" | "low" | "medium" | "high";

export interface StartupEntry {
  id: string;
  name: string;
  sourceType: StartupSourceType;
  sourceLabel: string;
  command: string;
  publisher: string | null;
  enabled: boolean;
  canChange: boolean;
  requiresElevation: boolean;
  impact: StartupImpact;
  impactEvidence: string | null;
  detail: string;
}

export interface StartupOperationRecord {
  operationId: string;
  entryId: string;
  displayName: string;
  sourceType: StartupSourceType;
  previousEnabled: boolean;
  newEnabled: boolean;
  success: boolean;
  rollbackAvailable: boolean;
  completedAtEpochMs: number;
  message: string;
}

export async function listStartupEntries(): Promise<StartupEntry[]> {
  return invoke<StartupEntry[]>("list_startup_entries");
}

export async function setStartupEntryEnabled(
  entryId: string,
  enabled: boolean,
): Promise<StartupOperationRecord> {
  return invoke<StartupOperationRecord>("set_startup_entry_enabled", {
    request: { entryId, enabled },
  });
}

export async function listStartupOperations(): Promise<StartupOperationRecord[]> {
  return invoke<StartupOperationRecord[]>("list_startup_operations");
}

export function sourceTypeLabel(source: StartupSourceType): string {
  switch (source) {
    case "registryCurrentUserRun":
      return "User Run";
    case "registryCurrentUserRunOnce":
      return "User RunOnce";
    case "registryLocalMachineRun":
      return "System Run";
    case "registryLocalMachineRunOnce":
      return "System RunOnce";
    case "userStartupFolder":
      return "User Startup";
    case "commonStartupFolder":
      return "All users Startup";
    case "scheduledTask":
      return "Scheduled Task";
  }
}

export function filterStartupEntries(
  entries: StartupEntry[],
  query: string,
): StartupEntry[] {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) {
    return entries;
  }

  return entries.filter((entry) =>
    [entry.name, entry.sourceLabel, entry.command, entry.publisher ?? ""]
      .join("\n")
      .toLocaleLowerCase()
      .includes(normalized),
  );
}
