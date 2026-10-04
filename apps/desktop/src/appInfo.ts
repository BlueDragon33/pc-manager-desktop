import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  appId: "pc-manager";
  platform: "windows";
  deviceType: "desktop-native";
  phase: string;
}

export function isAppInfo(value: unknown): value is AppInfo {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as Record<string, unknown>;

  return (
    candidate.appId === "pc-manager" &&
    candidate.platform === "windows" &&
    candidate.deviceType === "desktop-native" &&
    typeof candidate.phase === "string" &&
    candidate.phase.length > 0
  );
}

export async function getAppInfo(): Promise<AppInfo> {
  const value = await invoke<unknown>("get_app_info");

  if (!isAppInfo(value)) {
    throw new Error("Native bridge returned invalid application metadata.");
  }

  return value;
}
