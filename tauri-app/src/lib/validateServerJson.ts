// Client-side mirrors of the Rust validation functions in
// `src-tauri/src/commands.rs`. Duplicating the logic here lets us run
// the same checks synchronously as the user types, without an IPC
// round-trip for every keystroke — and keeps the error wording in
// lockstep with what the backend would say on Save.
//
// If the Rust rules ever change, both sides need to move together or
// the UI will lie to the user.

/// Validate a single MCP server config block — i.e. the value stored
/// under `mcpServers["name"]` in Claude's config. Used by the MCP
/// Servers view's detail editor when the user tweaks an existing entry
/// inline. Matches the Rust `validate_server_entries` rules:
///
///   • Must be valid JSON
///   • Must be a JSON object (not an array/primitive)
///   • Must have a non-empty `command` (stdio) or `url` (http)
///
/// Returns null on success, or a short human-readable message on
/// failure.
export function validateServerConfigJson(raw: string): string | null {
  if (!raw.trim()) {
    return "Config is empty.";
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (e) {
    return `Invalid JSON: ${(e as Error).message}`;
  }

  if (
    typeof parsed !== "object" ||
    parsed === null ||
    Array.isArray(parsed)
  ) {
    return "Config must be a JSON object.";
  }

  const obj = parsed as Record<string, unknown>;
  const hasCommand =
    typeof obj.command === "string" && obj.command.trim().length > 0;
  const hasUrl =
    typeof obj.url === "string" && obj.url.trim().length > 0;

  if (!hasCommand && !hasUrl) {
    return "Missing both `command` (for stdio) and `url` (for http).";
  }

  return null;
}

/// Validate the "Paste JSON" input from the Add Server modal. Accepts
/// the same three shapes the Rust `parse_server_input` command does,
/// and returns matching error strings:
///
///   1. `{ "mcpServers": { "name": {...} } }` — full wrapper
///   2. `{ "command": ..., "args": [...] }`  — single server body
///      (requires a fallback name from the adjacent input)
///   3. `{ "name": { ... } }`                 — bare map
///
/// Returns null on success, or a human-readable error message.
export function validatePasteInput(
  raw: string,
  fallbackName: string
): string | null {
  const trimmed = raw.trim();
  if (!trimmed) {
    return "Nothing to parse.";
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(trimmed);
  } catch {
    return "Invalid JSON. Check for trailing commas, missing quotes, or extra braces.";
  }

  if (
    typeof parsed !== "object" ||
    parsed === null ||
    Array.isArray(parsed)
  ) {
    return "Top-level must be a JSON object.";
  }

  const root = parsed as Record<string, unknown>;

  // Case 1: { "mcpServers": { ... } }
  if (
    typeof root.mcpServers === "object" &&
    root.mcpServers !== null &&
    !Array.isArray(root.mcpServers)
  ) {
    const entries = Object.entries(
      root.mcpServers as Record<string, unknown>
    ).filter(
      ([, v]) =>
        typeof v === "object" && v !== null && !Array.isArray(v)
    ) as Array<[string, Record<string, unknown>]>;
    if (entries.length === 0) {
      return "mcpServers is empty.";
    }
    return validateParsedEntries(entries);
  }

  // Case 2: single server body
  const looksSingle =
    "command" in root || "url" in root || "type" in root;
  if (looksSingle) {
    if (!fallbackName.trim()) {
      return "This JSON is a single server body — provide a name.";
    }
    return validateParsedEntries([[fallbackName.trim(), root]]);
  }

  // Case 3: bare map of name → config
  const entries = Object.entries(root).filter(
    ([, v]) => typeof v === "object" && v !== null && !Array.isArray(v)
  ) as Array<[string, Record<string, unknown>]>;
  if (entries.length === 0) {
    return "No valid server configs found.";
  }
  return validateParsedEntries(entries);
}

/// Shared body check — every entry must have a non-empty `command` or
/// `url`. Collects every offender into one message so the user can fix
/// a whole paste in one pass.
function validateParsedEntries(
  entries: Array<[string, Record<string, unknown>]>
): string | null {
  const bad: string[] = [];
  for (const [name, config] of entries) {
    const hasCommand =
      typeof config.command === "string" &&
      (config.command as string).trim().length > 0;
    const hasUrl =
      typeof config.url === "string" &&
      (config.url as string).trim().length > 0;
    if (!hasCommand && !hasUrl) {
      bad.push(`"${name}" is missing both \`command\` and \`url\``);
    }
  }
  return bad.length > 0 ? bad.join("; ") : null;
}
