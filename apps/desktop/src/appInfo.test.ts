import { describe, expect, it } from "vitest";

import { isAppInfo } from "./appInfo";

describe("isAppInfo", () => {
  it("accepts the P0 application identity", () => {
    expect(
      isAppInfo({
        appId: "pc-manager",
        platform: "windows",
        deviceType: "desktop-native",
        phase: "P0",
      }),
    ).toBe(true);
  });

  it("rejects untrusted metadata", () => {
    expect(
      isAppInfo({
        appId: "other-app",
        platform: "windows",
        deviceType: "desktop-native",
        phase: "P0",
      }),
    ).toBe(false);
  });
});
