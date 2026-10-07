import { describe, expect, it } from "vitest";

import {
  filterUpdateCandidates,
  type SoftwareUpdateCandidate,
} from "./softwareUpdater";

const candidate: SoftwareUpdateCandidate = {
  id: "opaque",
  name: "Microsoft PowerToys",
  packageId: "Microsoft.PowerToys",
  installedVersion: "0.90.0",
  availableVersion: "0.91.0",
  source: "winget",
  sourceVerified: true,
  publisher: null,
  publisherVerification: "Unavailable",
  detail: "Trusted provider",
};

describe("filterUpdateCandidates", () => {
  it("matches package identity and versions", () => {
    expect(filterUpdateCandidates([candidate], "powertoys")).toHaveLength(1);
    expect(filterUpdateCandidates([candidate], "0.91.0")).toHaveLength(1);
    expect(filterUpdateCandidates([candidate], "other")).toHaveLength(0);
  });

  it("returns all candidates for an empty query", () => {
    expect(filterUpdateCandidates([candidate], "  ")).toEqual([candidate]);
  });
});
