import { invoke } from "@tauri-apps/api/core";

export interface AppManagerState {
  configured: boolean;
  online: boolean;
  endpoint: string | null;
  deviceId: string | null;
  deviceCode: string | null;
  approvalState: string;
  appVersion: string;
  releaseChannel: string;
  entitlementState: string;
  updatePolicy: string;
  lastSuccessEpochMs: number | null;
  retryAfterSeconds: number;
  message: string;
}

export async function getAppManagerState(): Promise<AppManagerState> {
  return invoke<AppManagerState>("get_app_manager_state");
}

export async function syncAppManagerNow(): Promise<AppManagerState> {
  return invoke<AppManagerState>("sync_app_manager_now");
}

export async function setAppManagerEndpoint(
  endpoint: string | null,
): Promise<AppManagerState> {
  return invoke<AppManagerState>("set_app_manager_endpoint", {
    endpoint,
  });
}

export function appManagerConnectionLabel(state: AppManagerState): string {
  if (!state.configured) return "Not configured";
  return state.online ? "Online" : "Offline";
}

export function appManagerApprovalLabel(value: string): string {
  switch (value) {
    case "approved":
      return "Approved";
    case "pending":
      return "Pending approval";
    case "blocked":
      return "Blocked";
    case "unconfigured":
      return "Not configured";
    default:
      return "Unknown";
  }
}
