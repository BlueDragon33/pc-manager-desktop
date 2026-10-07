import { describe, expect, it } from "vitest";

import {
  filterDriverUpdates,
  filterInstalledDrivers,
  type DriverInventoryEntry,
  type DriverUpdateCandidate,
} from "./driverCenter";

const update: DriverUpdateCandidate = {
  id: "update",
  title: "Example Corp - Net - 2.0",
  manufacturer: "Example Corp",
  provider: "Example Provider",
  model: "Example Adapter",
  className: "Net",
  driverDate: null,
  downloaded: false,
  eulaAccepted: false,
  rebootRequired: false,
  source: "Windows Update",
  recommendation: "Applicable",
};

const installed: DriverInventoryEntry = {
  id: "installed",
  deviceName: "Example Adapter",
  manufacturer: "Example Corp",
  provider: "Example Provider",
  version: "1.0",
  driverDate: null,
  className: "Net",
  signer: "Example Signer",
  infName: "oem1.inf",
  isSigned: true,
};

describe("Driver Center filters", () => {
  it("filters applicable updates by provider and model", () => {
    expect(filterDriverUpdates([update], "adapter")).toHaveLength(1);
    expect(filterDriverUpdates([update], "windows update")).toHaveLength(1);
    expect(filterDriverUpdates([update], "audio")).toHaveLength(0);
  });

  it("filters installed inventory without exposing native IDs", () => {
    expect(filterInstalledDrivers([installed], "1.0")).toHaveLength(1);
    expect(filterInstalledDrivers([installed], "oem1")).toHaveLength(1);
    expect(filterInstalledDrivers([installed], "missing")).toHaveLength(0);
  });
});
