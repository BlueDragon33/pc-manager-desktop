import { describe, expect, it } from "vitest";

import { topTypeShare, type StorageScanSummary } from "./storage";

describe("Storage helpers", () => {
  it("calculates the top file-type share from real aggregate bytes", () => {
    const summary: StorageScanSummary = {
      scanId: "s",
      totalFiles: 3,
      totalBytes: 100,
      largestFiles: [],
      largestFolders: [],
      fileTypes: [
        { extension: ".bin", bytes: 60, fileCount: 1 },
        { extension: ".txt", bytes: 40, fileCount: 2 },
      ],
      warnings: [],
    };

    expect(topTypeShare(summary)).toBe(0.6);
  });

  it("returns zero for an empty scan", () => {
    const summary: StorageScanSummary = {
      scanId: "s",
      totalFiles: 0,
      totalBytes: 0,
      largestFiles: [],
      largestFolders: [],
      fileTypes: [],
      warnings: [],
    };

    expect(topTypeShare(summary)).toBe(0);
  });
});
