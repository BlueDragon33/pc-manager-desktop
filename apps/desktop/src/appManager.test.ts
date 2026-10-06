import { describe, expect, it } from "vitest";

import {
  appManagerStatusText,
  isAppManagerRuntimeStatus,
  type AppManagerRuntimeStatus,
} from "./appManager";

function status(
  overrides: Partial<AppManagerRuntimeStatus> = {},
): AppManagerRuntimeStatus {
  return {
    configured: true,
    connection: "online",
    device: {
      deviceId: "a".repeat(64),
      deviceCode: "PC-AAAA-BBBB-CCCC",
      appId: "pc-manager",
      platform: "windows",
      deviceType: "desktop-native",
      status: "approved",
      online: true,
      appVersion: "0.1.0",
      releaseChannel: "stable",
      entitlementState: "active",
      updatePolicy: { channel: "stable", autoCheck: true },
      createdAt: "2026-10-06T00:00:00Z",
      approvedAt: "2026-10-06T00:00:00Z",
      blockedAt: null,
      lastSeenAt: "2026-10-06T00:00:00Z",
    },
    lastError: null,
    retryAfterSeconds: 60,
    lastSyncEpochMs: 1,
    ...overrides,
  };
}

describe("App Manager runtime state", () => {
  it("rejects malformed native status payloads", () => {
    expect(isAppManagerRuntimeStatus(status())).toBe(true);
    expect(
      isAppManagerRuntimeStatus({
        ...status(),
        retryAfterSeconds: "60",
      }),
    ).toBe(false);
  });

  it("keeps approval and entitlement states visible", () => {
    expect(appManagerStatusText(status())).toBe("Connected");
    expect(
      appManagerStatusText(
        status({
          device: { ...status().device!, status: "pending" },
        }),
      ),
    ).toBe("Pending approval");
    expect(
      appManagerStatusText(
        status({
          device: { ...status().device!, entitlementState: "disabled" },
        }),
      ),
    ).toBe("License disabled");
  });

  it("reports offline and unconfigured without blocking local features", () => {
    expect(
      appManagerStatusText(
        status({
          configured: false,
          connection: "notConfigured",
          device: null,
        }),
      ),
    ).toBe("Not configured");
    expect(
      appManagerStatusText(status({ connection: "offline", device: null })),
    ).toBe("Offline");
  });
});
