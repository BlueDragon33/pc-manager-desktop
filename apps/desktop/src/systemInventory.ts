import { invoke } from "@tauri-apps/api/core";

export interface SystemInventory {
  collectedAtEpochMs: number;
  operatingSystem: {
    name: string;
    version: string;
    buildNumber: string | null;
    architecture: string;
  };
  device: {
    hostname: string;
    localDeviceId: string | null;
  };
  cpu: {
    brand: string;
    physicalCores: number | null;
    logicalCores: number;
    architecture: string;
  };
  memory: {
    totalBytes: number;
    availableBytes: number;
  };
  volumes: Array<{
    name: string;
    label: string | null;
    fileSystem: string | null;
    totalBytes: number;
    availableBytes: number;
  }>;
  processes: {
    totalCount: number;
    topMemory: Array<{
      pid: number;
      name: string;
      memoryBytes: number;
    }>;
  };
  installedApplications: Array<{
    name: string;
    version: string | null;
    publisher: string | null;
  }>;
  startupItems: Array<{
    name: string;
    source: string;
    location: string | null;
    command: string | null;
  }>;
  networkAdapters: Array<{
    name: string;
    description: string | null;
    enabled: boolean | null;
    linkSpeedBps: number | null;
  }>;
  warnings: Array<{
    source: string;
    message: string;
  }>;
}

export interface InventoryError {
  code: string;
  message: string;
  recoverable: boolean;
}

export async function getSystemInventory(): Promise<SystemInventory> {
  return invoke<SystemInventory>("get_system_inventory");
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  const unitIndex = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** unitIndex;
  const digits = unitIndex >= 3 ? 1 : 0;

  return `${value.toFixed(digits)} ${units[unitIndex]}`;
}
