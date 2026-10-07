import { invoke } from "@tauri-apps/api/core";

export interface DriverInventoryEntry {
  id: string;
  deviceName: string;
  manufacturer: string | null;
  provider: string | null;
  version: string | null;
  driverDate: string | null;
  className: string | null;
  signer: string | null;
  infName: string | null;
  isSigned: boolean | null;
}

export interface DriverUpdateCandidate {
  id: string;
  title: string;
  manufacturer: string | null;
  provider: string | null;
  model: string | null;
  className: string | null;
  driverDate: string | null;
  downloaded: boolean;
  eulaAccepted: boolean;
  rebootRequired: boolean;
  source: string;
  recommendation: string;
}

export interface DriverCenterWarning {
  code: string;
  message: string;
}

export interface DriverCenterSnapshot {
  provider: string;
  providerAvailable: boolean;
  sourceDetail: string;
  installedDrivers: DriverInventoryEntry[];
  availableUpdates: DriverUpdateCandidate[];
  warnings: DriverCenterWarning[];
  generatedEpochMs: number;
}

export interface DriverSettingsLaunchResult {
  launched: boolean;
  target: string;
  message: string;
}

export async function scanDriverCenter(): Promise<DriverCenterSnapshot> {
  return invoke<DriverCenterSnapshot>("scan_driver_center");
}

export async function openDriverUpdateSettings(): Promise<DriverSettingsLaunchResult> {
  return invoke<DriverSettingsLaunchResult>("open_driver_update_settings");
}

function matches(values: Array<string | null>, query: string): boolean {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return true;

  return values
    .filter((value): value is string => value !== null)
    .join(" ")
    .toLocaleLowerCase()
    .includes(needle);
}

export function filterDriverUpdates(
  updates: DriverUpdateCandidate[],
  query: string,
): DriverUpdateCandidate[] {
  return updates.filter((update) =>
    matches(
      [
        update.title,
        update.manufacturer,
        update.provider,
        update.model,
        update.className,
        update.source,
      ],
      query,
    ),
  );
}

export function filterInstalledDrivers(
  drivers: DriverInventoryEntry[],
  query: string,
): DriverInventoryEntry[] {
  return drivers.filter((driver) =>
    matches(
      [
        driver.deviceName,
        driver.manufacturer,
        driver.provider,
        driver.version,
        driver.className,
        driver.signer,
        driver.infName,
      ],
      query,
    ),
  );
}
