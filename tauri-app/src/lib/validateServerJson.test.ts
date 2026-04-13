import { describe, expect, it } from "vitest";
import { validateServerConfigJson, validatePasteInput } from "./validateServerJson";

// ---------------------------------------------------------------------------
// validateServerConfigJson
// ---------------------------------------------------------------------------

describe("validateServerConfigJson", () => {
  it("returns null for a valid object with command", () => {
    expect(
      validateServerConfigJson(JSON.stringify({ command: "npx", args: ["-y", "foo"] }))
    ).toBeNull();
  });

  it("returns null for a valid object with url", () => {
    expect(
      validateServerConfigJson(JSON.stringify({ url: "http://localhost:3000/mcp" }))
    ).toBeNull();
  });

  it("returns null for an object with neither command nor url (relaxed validation)", () => {
    // We no longer block on missing command/url — just check it's valid JSON object
    expect(
      validateServerConfigJson(JSON.stringify({ type: "sse", endpoint: "/mcp" }))
    ).toBeNull();
  });

  it("returns null for a wrapper-style object (not blocked)", () => {
    const wrapper = { mcpServers: { test: { command: "npx" } } };
    expect(validateServerConfigJson(JSON.stringify(wrapper))).toBeNull();
  });

  it("returns error for empty string", () => {
    expect(validateServerConfigJson("")).toBe("Config is empty.");
    expect(validateServerConfigJson("   ")).toBe("Config is empty.");
  });

  it("returns error for invalid JSON", () => {
    const result = validateServerConfigJson("{ not json }");
    expect(result).toMatch(/^Invalid JSON:/);
  });

  it("returns error for array", () => {
    expect(validateServerConfigJson("[1, 2]")).toBe("Config must be a JSON object.");
  });

  it("returns error for primitive", () => {
    expect(validateServerConfigJson('"hello"')).toBe("Config must be a JSON object.");
    expect(validateServerConfigJson("42")).toBe("Config must be a JSON object.");
    expect(validateServerConfigJson("true")).toBe("Config must be a JSON object.");
    expect(validateServerConfigJson("null")).toBe("Config must be a JSON object.");
  });
});

// ---------------------------------------------------------------------------
// validatePasteInput
// ---------------------------------------------------------------------------

describe("validatePasteInput", () => {
  // Case 1: mcpServers wrapper
  it("accepts { mcpServers: { name: { command } } }", () => {
    const json = JSON.stringify({
      mcpServers: { github: { command: "npx", args: ["-y", "@github/mcp"] } },
    });
    expect(validatePasteInput(json, "")).toBeNull();
  });

  it("rejects empty mcpServers", () => {
    expect(
      validatePasteInput(JSON.stringify({ mcpServers: {} }), "")
    ).toBe("mcpServers is empty.");
  });

  it("reports missing command/url in mcpServers entries", () => {
    const json = JSON.stringify({
      mcpServers: { bad: { args: ["foo"] } },
    });
    const result = validatePasteInput(json, "");
    expect(result).toContain('"bad"');
    expect(result).toContain("missing both");
  });

  // Case 2: single server body
  it("accepts single server body with fallback name", () => {
    const json = JSON.stringify({ command: "docker", args: ["run", "img"] });
    expect(validatePasteInput(json, "my-server")).toBeNull();
  });

  it("rejects single server body without fallback name", () => {
    const json = JSON.stringify({ command: "docker" });
    expect(validatePasteInput(json, "")).toBe(
      "This JSON is a single server body — provide a name."
    );
    expect(validatePasteInput(json, "   ")).toBe(
      "This JSON is a single server body — provide a name."
    );
  });

  // Case 3: bare map
  it("accepts bare map of name → config", () => {
    const json = JSON.stringify({
      github: { command: "npx", args: [] },
      gitlab: { url: "http://localhost:8080" },
    });
    expect(validatePasteInput(json, "")).toBeNull();
  });

  it("rejects bare map where all entries lack command/url", () => {
    const json = JSON.stringify({
      bad1: { args: [] },
      bad2: { env: {} },
    });
    const result = validatePasteInput(json, "");
    expect(result).toContain('"bad1"');
    expect(result).toContain('"bad2"');
  });

  // Edge cases
  it("returns error for empty input", () => {
    expect(validatePasteInput("", "x")).toBe("Nothing to parse.");
    expect(validatePasteInput("  ", "x")).toBe("Nothing to parse.");
  });

  it("returns error for invalid JSON", () => {
    expect(validatePasteInput("{bad}", "x")).toMatch(/Invalid JSON/);
  });

  it("returns error for non-object JSON", () => {
    expect(validatePasteInput("[1]", "x")).toBe("Top-level must be a JSON object.");
    expect(validatePasteInput("42", "x")).toBe("Top-level must be a JSON object.");
  });

  it("returns error when no valid server configs in bare map", () => {
    // All values are non-objects
    expect(
      validatePasteInput(JSON.stringify({ foo: "string", bar: 42 }), "")
    ).toBe("No valid server configs found.");
  });
});
