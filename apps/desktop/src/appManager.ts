import { invoke } from "@tauri-apps/api/core";

export type AppManagerConnection = "notConfigured" | "offline" | "online";
export type AppManagerApproval = "pending" | "approved" | "blocked";
export type AppManagerEntitlement = "unknown" | "active" | "trial" | "disabled";
export type AppManagerReleaseChannel = "dev" | "beta" | "stable";

export interface AppManagerDevice {
  deviceId: string;
  deviceCode: string;
  appId: "pc-manager";
  platform: "windows";
  deviceType: "desktop-native";
  status: AppManagerApproval;
  online: boolean;
  appVersion: string;
  releaseChannel: AppManagerReleaseChannel;
  entitlementState: AppManagerEntitlement;
  updatePolicy: {
    channel: AppManagerReleaseChannel;
    autoCheck: boolean;
    minimumVersion?: string | null;
  };
  createdAt: string;
  approvedAt?: string | null;
  blockedAt?: string | null;
  lastSeenAt?: string | null;
}

export interface AppManagerRuntimeStatus {
  configured: boolean;
  connection: AppManagerConnection;
  device: AppManagerDevice | null;
  lastError: string | null;
  retryAfterSeconds: number;
  lastSyncEpochMs: number | null;
}

function isConnection(value: unknown): value is AppManagerConnection {
  return (
    value === "notConfigured" || value === "offline" || value === "online"
  );
}

export function isAppManagerRuntimeStatus(
  value: unknown,
): value is AppManagerRuntimeStatus {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.configured === "boolean" &&
    isConnection(candidate.connection) &&
    (candidate.device === null ||
      (typeof candidate.device === "object" && candidate.device !== null)) &&
    (candidate.lastError === null || typeof candidate.lastError === "string") &&
    typeof candidate.retryAfterSeconds === "number" &&
    candidate.retryAfterSeconds >= 0 &&
    (candidate.lastSyncEpochMs === null ||
      typeof candidate.lastSyncEpochMs === "number")
  );
}

async function invokeStatus(
  command: "get_app_manager_status" | "sync_app_manager",
): Promise<AppManagerRuntimeStatus> {
  const value = await invoke<unknown>(command);
  if (!isAppManagerRuntimeStatus(value)) {
    throw new Error("Native bridge returned invalid App Manager state.");
  }
  return value;
}

export function getAppManagerStatus(): Promise<AppManagerRuntimeStatus> {
  return invokeStatus("get_app_manager_status");
}

export function syncAppManager(): Promise<AppManagerRuntimeStatus> {
  return invokeStatus("sync_app_manager");
}

export function appManagerStatusText(state: AppManagerRuntimeStatus): string {
  if (!state.configured) return "Not configured";
  if (state.connection === "offline") return "Offline";
  if (state.device?.status === "pending") return "Pending approval";
  if (state.device?.status === "blocked") return "Blocked";
  if (state.device?.entitlementState === "disabled") return "License disabled";
  return "Connected";
}
