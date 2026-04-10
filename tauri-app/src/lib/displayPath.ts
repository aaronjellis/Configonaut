// Shortens an absolute filesystem path for display by collapsing the
// user's home directory to `~`. The Rust side hands us absolute paths
// with no notion of `$HOME`, so we pattern-match the common layouts on
// each platform rather than plumbing the home directory through IPC.
//
// Handles:
//   • macOS   /Users/<name>/...                    → ~/...
//   • Linux   /home/<name>/...                     → ~/...
//   • Windows C:\Users\<name>\... (or forward / )  → ~\...
//
// Windows paths come back from Rust's `to_string_lossy()` with native
// backslashes, but we accept either separator defensively — Tauri's
// dev tools occasionally normalize them to forward slashes, and it
// costs nothing to be forgiving.
//
// There's exactly one copy of this helper by design: four views had
// drifted into near-identical private implementations, none of which
// recognized Windows. Centralizing it means one place to fix when the
// next platform shows up.
export function displayPath(abs: string): string {
  if (!abs) return "";
  const mac = abs.match(/^\/Users\/[^/]+/);
  if (mac) return "~" + abs.slice(mac[0].length);
  const linux = abs.match(/^\/home\/[^/]+/);
  if (linux) return "~" + abs.slice(linux[0].length);
  const win = abs.match(/^[A-Za-z]:[\\/]Users[\\/][^\\/]+/);
  if (win) return "~" + abs.slice(win[0].length);
  return abs;
}
