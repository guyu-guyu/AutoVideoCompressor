import { describe, it, expect } from "vitest";
import { formatFileSize } from "./format";

describe("formatFileSize", () => {
  it("formats units", () => {
    expect(formatFileSize(0)).toBe("0.0 B");
    expect(formatFileSize(1536)).toBe("1.5 KB");
    expect(formatFileSize(1024 * 1024)).toBe("1.0 MB");
  });
});
