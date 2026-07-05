import { describe, expect, it } from "vitest";
import { PACKAGE_NAME } from "./index.js";

describe("core package scaffold", () => {
  it("exposes its package name", () => {
    expect(PACKAGE_NAME).toBe("@tsumugai/core");
  });
});
