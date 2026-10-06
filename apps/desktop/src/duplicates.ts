import { invoke } from "@tauri-apps/api/core";

export interface ScanRootSelection {
  id: string;
  displayPath: string;
}

export interface FilesystemWarning {
  source: string;
  message: string;
}

export interface DuplicateFile {
  id: string;
  path: string;
  bytes: number;
  modifiedAtEpochMs: number | null;
}

export interface DuplicateGroup {
  id: string;
  bytesEach: number;
  totalBytes: number;
  recoverableBytes: number;
  files: DuplicateFile[];
}

export interface DuplicateScanOptions {
  minFileBytes: number;
  excludedDirectoryNames: string[];
  maxGroups: number;
  maxFilesPerGroup: number;
}

export interface DuplicateScanSummary {
  scanId: string;
  scannedFiles: number;
  scannedBytes: number;
  duplicateGroups: DuplicateGroup[];
  warnings: FilesystemWarning[];
}

export interface DuplicateDeleteError {
  fileId: string;
  code: string;
  message: string;
}

export interface DuplicateDeleteResult {
  operationId: string;
  scanId: string;
  requestedFiles: number;
  deletedFiles: number;
  deletedBytes: number;
  failedFiles: number;
  errors: DuplicateDeleteError[];
  completedAtEpochMs: number;
}

export async function selectScanRoot(): Promise<ScanRootSelection | null> {
  return invoke<ScanRootSelection | null>("select_scan_root");
}

export async function scanDuplicates(
  requestId: string,
  rootIds: string[],
  options: DuplicateScanOptions,
): Promise<DuplicateScanSummary> {
  return invoke<DuplicateScanSummary>("scan_duplicates", {
    requestId,
    rootIds,
    options,
  });
}

export async function cancelFilesystemScan(requestId: string): Promise<boolean> {
  return invoke<boolean>("cancel_filesystem_scan", { requestId });
}

export async function deleteDuplicateFiles(
  scanId: string,
  fileIds: string[],
): Promise<DuplicateDeleteResult> {
  return invoke<DuplicateDeleteResult>("delete_duplicate_files", {
    request: { scanId, fileIds },
  });
}

export function parseExclusions(value: string): string[] {
  return Array.from(
    new Set(
      value
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean)
        .map((item) => item.toLocaleLowerCase()),
    ),
  );
}

export function canSelectDuplicate(
  group: DuplicateGroup,
  selectedIds: ReadonlySet<string>,
  fileId: string,
): boolean {
  if (selectedIds.has(fileId)) {
    return true;
  }

  const selectedInGroup = group.files.filter((file) =>
    selectedIds.has(file.id),
  ).length;
  return selectedInGroup < group.files.length - 1;
}

export function duplicateWaste(groups: DuplicateGroup[]): number {
  return groups.reduce((total, group) => total + group.recoverableBytes, 0);
}
