import { describe, expect, it } from "vitest";

import {
  canSelectDuplicate,
  duplicateWaste,
  parseExclusions,
  type DuplicateGroup,
} from "./duplicates";

const group: DuplicateGroup = {
  id: "g",
  bytesEach: 10,
  totalBytes: 30,
  recoverableBytes: 20,
  files: [
    { id: "a", path: "a", bytes: 10, modifiedAtEpochMs: null },
    { id: "b", path: "b", bytes: 10, modifiedAtEpochMs: null },
    { id: "c", path: "c", bytes: 10, modifiedAtEpochMs: null },
  ],
};

describe("Duplicate Finder helpers", () => {
  it("normalizes and deduplicates exclusion names", () => {
    expect(parseExclusions("node_modules, Target, node_modules")).toEqual([
      "node_modules",
      "target",
    ]);
  });

  it("prevents selecting every visible copy in a group", () => {
    const selected = new Set(["a", "b"]);
    expect(canSelectDuplicate(group, selected, "c")).toBe(false);
    expect(canSelectDuplicate(group, selected, "a")).toBe(true);
  });

  it("sums only proven recoverable duplicate bytes", () => {
    expect(duplicateWaste([group, { ...group, id: "g2" }])).toBe(40);
  });
});
