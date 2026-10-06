import { describe, expect, it } from "vitest";

import {
  appManagerApprovalLabel,
  appManagerConnectionLabel,
  type AppManagerState,
} from "./appManager";

const state = (overrides: Partial<AppManagerState> = {}): AppManagerState => ({
  configured: true,
  online: true,
  endpoint: "https://manager.example.test",
  deviceId: "device",
  deviceCode: "PC-AAAA-BBBB-CCCC",
  approvalState: "approved",
  appVersion: "0.1.0",
  releaseChannel: "stable",
  entitlementState: "active",
  updatePolicy: "notify",
  lastSuccessEpochMs: 1,
  retryAfterSeconds: 60,
  message: "Connected",
  ...overrides,
});

describe("App Manager presentation helpers", () => {
  it("keeps configured offline distinct from not configured", () => {
    expect(appManagerConnectionLabel(state())).toBe("Online");
    expect(appManagerConnectionLabel(state({ online: false }))).toBe("Offline");
    expect(
      appManagerConnectionLabel(state({ configured: false, online: false })),
    ).toBe("Not configured");
  });

  it("renders approval states without inventing access", () => {
    expect(appManagerApprovalLabel("approved")).toBe("Approved");
    expect(appManagerApprovalLabel("pending")).toBe("Pending approval");
    expect(appManagerApprovalLabel("blocked")).toBe("Blocked");
    expect(appManagerApprovalLabel("something-new")).toBe("Unknown");
  });
});
