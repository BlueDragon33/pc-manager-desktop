import { invoke } from "@tauri-apps/api/core";

export type InstalledAppSource =
  "currentUser" | "localMachine64" | "localMachine32";

export type UninstallKind = "msi" | "executable" | "unavailable";

export interface InstalledAppEntry {
  id: string;
  displayName: string;
  publisher: string | null;
  version: string | null;
  installLocation: string | null;
  installDate: string | null;
  estimatedSizeBytes: number | null;
  source: InstalledAppSource;
  canUninstall: boolean;
  requiresElevation: boolean;
  uninstallKind: UninstallKind;
  detail: string;
}

export interface UninstallLaunchResult {
  appId: string;
  displayName: string;
  launched: boolean;
  message: string;
}

export async function listInstalledApps(): Promise<InstalledAppEntry[]> {
  return invoke<InstalledAppEntry[]>("list_installed_apps");
}

export async function launchUninstall(
  appId: string,
): Promise<UninstallLaunchResult> {
  return invoke<UninstallLaunchResult>("launch_uninstall", {
    request: { appId },
  });
}

export function filterInstalledApps(
  apps: InstalledAppEntry[],
  query: string,
): InstalledAppEntry[] {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) {
    return apps;
  }

  return apps.filter((app) =>
    [
      app.displayName,
      app.publisher ?? "",
      app.version ?? "",
      sourceLabel(app.source),
    ]
      .join(" ")
      .toLocaleLowerCase()
      .includes(needle),
  );
}

export function sourceLabel(source: InstalledAppSource): string {
  switch (source) {
    case "currentUser":
      return "Current user";
    case "localMachine64":
      return "Machine · 64-bit";
    case "localMachine32":
      return "Machine · 32-bit";
  }
}
