import { describe, expect, it } from "vitest";
import { displayPath } from "./displayPath";

describe("displayPath", () => {
  it("collapses macOS home directory to ~", () => {
    expect(displayPath("/Users/alice/Library/Claude/config.json")).toBe(
      "~/Library/Claude/config.json"
    );
  });

  it("collapses macOS home with no trailing path", () => {
    expect(displayPath("/Users/alice")).toBe("~");
  });

  it("collapses Linux home directory to ~", () => {
    expect(displayPath("/home/bob/.claude.json")).toBe("~/.claude.json");
  });

  it("collapses Windows path with backslashes", () => {
    expect(displayPath("C:\\Users\\carol\\AppData\\config.json")).toBe(
      "~\\AppData\\config.json"
    );
  });

  it("collapses Windows path with forward slashes", () => {
    expect(displayPath("C:/Users/carol/AppData/config.json")).toBe(
      "~/AppData/config.json"
    );
  });

  it("handles lowercase drive letter", () => {
    expect(displayPath("c:\\Users\\dave\\file.txt")).toBe("~\\file.txt");
  });

  it("returns non-home paths unchanged", () => {
    expect(displayPath("/etc/config")).toBe("/etc/config");
    expect(displayPath("/var/log/app.log")).toBe("/var/log/app.log");
  });

  it("returns empty string for empty input", () => {
    expect(displayPath("")).toBe("");
  });

  it("handles deeply nested paths", () => {
    expect(displayPath("/Users/alice/a/b/c/d/e.json")).toBe("~/a/b/c/d/e.json");
  });
});
