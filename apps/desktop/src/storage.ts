import { invoke } from "@tauri-apps/api/core";

import type { FilesystemWarning, ScanRootSelection } from "./duplicates";

export interface StorageScanOptions {
  excludedDirectoryNames: string[];
  topFiles: number;
  topFolders: number;
}

export interface StorageFileEntry {
  path: string;
  bytes: number;
}

export interface StorageFolderAggregate {
  path: string;
  bytes: number;
  fileCount: number;
}

export interface StorageTypeAggregate {
  extension: string;
  bytes: number;
  fileCount: number;
}

export interface StorageScanSummary {
  scanId: string;
  totalFiles: number;
  totalBytes: number;
  largestFiles: StorageFileEntry[];
  largestFolders: StorageFolderAggregate[];
  fileTypes: StorageTypeAggregate[];
  warnings: FilesystemWarning[];
}

export async function selectStorageRoot(): Promise<ScanRootSelection | null> {
  return invoke<ScanRootSelection | null>("select_scan_root");
}

export async function scanStorage(
  requestId: string,
  rootIds: string[],
  options: StorageScanOptions,
): Promise<StorageScanSummary> {
  return invoke<StorageScanSummary>("scan_storage", {
    requestId,
    rootIds,
    options,
  });
}

export function topTypeShare(summary: StorageScanSummary): number {
  const top = summary.fileTypes[0]?.bytes ?? 0;
  if (summary.totalBytes <= 0) {
    return 0;
  }
  return top / summary.totalBytes;
}
