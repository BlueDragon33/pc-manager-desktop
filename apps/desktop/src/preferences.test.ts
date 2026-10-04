import { describe, expect, it } from "vitest";

import { resolveTheme } from "./preferences";

describe("theme resolution", () => {
  it("respects explicit theme choices", () => {
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("light", true)).toBe("light");
  });

  it("follows the operating-system preference in system mode", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });
});
