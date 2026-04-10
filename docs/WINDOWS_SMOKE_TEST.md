# Windows Smoke Test

A focused checklist for validating the Configonaut Tauri build on Windows
before calling the cross-platform port "done." The Rust and TypeScript
source has been audited for obvious Windows correctness issues (path
handling, process management, window chrome), but nothing has actually
been built or run on a Windows machine yet — everything below needs live
verification.

If any step fails, capture the exact error, what you expected, and what
happened, and open an issue tagged `windows`.

## 0. Prerequisites

On a Windows 10 or Windows 11 machine:

- Rust toolchain with the `x86_64-pc-windows-msvc` target (the default
  when installing via `rustup` on Windows).
- Visual Studio Build Tools 2019+ with the "Desktop development with
  C++" workload (Tauri needs the MSVC linker).
- Node 20+ and Bun (`npm i -g bun`).
- A working copy of Claude Desktop installed to
  `%LOCALAPPDATA%\AnthropicClaude\Claude.exe`. If your install is
  somewhere else, note the path — step 6 depends on it and may need a
  code fix.
- Optional but recommended: Claude Code CLI installed, with at least one
  hook, agent, or skill configured in `%USERPROFILE%\.claude\`.

## 1. Build

From the repo root:

    cd tauri-app
    bun install
    bun run tauri build

Expected: a clean build producing `.msi` and `.exe` (NSIS) installers in
`src-tauri/target/release/bundle/`. Watch the Rust compile output for
any `cfg(target_os = "windows")` warnings or errors — the Windows
branch of `restart_claude_desktop` has never been compiled before, so
type errors here are plausible.

Known gotchas to watch for:

- `macos-private-api` feature is enabled on `tauri` in `Cargo.toml`.
  This is a no-op on Windows but shouldn't break the build. If it does,
  we need to gate the feature on `[target.'cfg(target_os = "macos")'.dependencies]`.
- The `tauri.conf.json` main window sets `titleBarStyle: "Overlay"` and
  `hiddenTitle: true`. Tauri will print warnings about these being
  macOS-only and ignore them; that's expected and fine.

## 2. First launch

Install the `.msi` (or run the built `.exe` directly), then launch.

- [ ] Splash window appears, centered, transparent background, rotating
      neon-green ring around the app icon. On Windows, transparent
      windows are supported natively — there's no `macos-private-api`
      equivalent needed. If you see a white or black rectangle around
      the splash card, the Windows compositor may not be honoring
      `transparent: true`; try toggling `shadow: true` in
      `tauri.conf.json`.
- [ ] After ~5 seconds the splash closes and the main window opens.
- [ ] The main window has native Windows chrome (minimize, maximize,
      close buttons in the top-right, standard title bar with app
      name). There should be **no** extra 28px blank strip beneath the
      title bar — that strip is macOS-only and is collapsed to zero
      height via `main.tsx` OS detection → `[data-os="windows"]` CSS.
      If you see the strip, `main.tsx` isn't setting `data-os` in time
      or the CSS selector isn't matching.
- [ ] The sidebar (floating card) has a rounded border, slight margin
      from the window edges, and blurred translucent background.
- [ ] App icon in the sidebar renders correctly (56×56).

## 3. MCP Servers view (Desktop mode)

- [ ] Header shows `~\AppData\Roaming\Claude\claude_desktop_config.json`
      (not the raw `C:\Users\<name>\...` form) — this verifies the new
      `displayPath` helper is matching the Windows pattern.
- [ ] Active servers column populates from the real
      `%APPDATA%\Claude\claude_desktop_config.json` file. Empty is fine.
- [ ] Click **+ Add Server**. Paste a valid single-server JSON snippet,
      hit Save. New row appears in Active. Realtime validation still
      works (type garbage in the textarea → red border).
- [ ] Drag a row from Active → Inactive. Row moves, the Desktop config
      file on disk updates, a new timestamped file appears in
      `%APPDATA%\Configonaut\backups\desktop\`.
- [ ] Click **Edit JSON** on an Active row, change a field, Save. Config
      file on disk reflects the change.

## 4. MCP Servers view (CLI mode)

- [ ] Switch to CLI mode in the sidebar.
- [ ] Header path becomes `~\.claude\settings.json`.
- [ ] Active/Stored columns populate from `%USERPROFILE%\.claude\settings.json`.
- [ ] Add/move/delete flows all work and write back to the CLI settings
      file without clobbering hooks or other unrelated keys.

## 5. Marketplace install

- [ ] Open **+ Add Server → Marketplace** tab. Catalog loads (may be
      from cached `%APPDATA%\Configonaut\catalog-cache.json` on first
      launch).
- [ ] Pick a server that requires env vars (e.g. one with an API key).
      Install it to **Inactive**. The row lands in the Inactive
      column with a warning indicator.
- [ ] Try to drag it to Active without filling the env var. Should show
      the "still need: VAR_NAME" error toast.
- [ ] Edit JSON, fill the real value, save, then move to Active. Should
      succeed.

## 6. Restart Claude Desktop

This is the highest-risk Windows code path.

- [ ] With Claude Desktop running, click **Restart Claude Desktop** in
      the MCP Servers footer.
- [ ] Expected: Claude Desktop window closes (no graceful quit dialog —
      we use `taskkill /F`), then a fresh Claude Desktop window opens
      within ~1–2 seconds.
- [ ] If Claude Desktop is **not** running, click Restart. Expected: a
      new Claude window appears.
- [ ] If Claude Desktop is installed at a non-standard path, you will
      get the error `Couldn't find Claude.exe at <path>. Is Claude
      Desktop installed?` — note the path and let us know so we can
      add fallback discovery.

Debugging the restart race:

- If the window flashes closed and a new one never opens, the
  `tasklist` poll loop is probably returning false negatives. Check
  that `tasklist /FI "IMAGENAME eq Claude.exe" /NH` actually prints
  the row when run from `cmd.exe` — some PowerShell environments
  localize the output.

## 7. Backups view

- [ ] Backups list populates from `%APPDATA%\Configonaut\backups\<mode>\`.
- [ ] Click a backup row → diff preview renders.
- [ ] Click **Restore** → live config is replaced by the backup content,
      a new "pre-restore" backup is created first.
- [ ] Click **Reveal in Explorer** → Windows Explorer opens to the
      backups folder.
- [ ] Delete a backup → row disappears, file is gone from disk.

## 8. Hooks view (Claude Code only)

- [ ] Hooks list populates from `%USERPROFILE%\.claude\settings.json`.
- [ ] Footer shows `~\.claude\settings.json` (Windows-style tilde
      collapse).
- [ ] Toggle a hook on/off → settings.json updates immediately.
- [ ] Edit the JSON for a rule, save, verify it round-trips on disk.

## 9. Agents + Skills views

- [ ] Agents list shows personal agents from `~\.claude\agents\` and
      plugin agents (if any plugins installed).
- [ ] Skills list shows personal skills from `~\.claude\skills\` and
      plugin skills.
- [ ] Path footers on each row show `~\.claude\agents\foo.md` (tilde
      collapsed), not the raw `C:\Users\<name>\...` form.
- [ ] Create new agent / create new skill → new file appears in the
      correct directory.
- [ ] Toggle a plugin on/off → plugin state persists.

## 10. General UX

- [ ] Window resize works smoothly.
- [ ] Minimum window size is enforced (980×620).
- [ ] Close button quits the app cleanly — no orphaned processes in
      Task Manager.
- [ ] Dark mode colors look correct; text is readable; no blown-out
      whites from the macOS-tuned blur backgrounds.

## 11. Uninstall

- [ ] Run the installer's uninstaller (or remove via Apps & Features).
- [ ] `%APPDATA%\Configonaut\` should **persist** — it holds user data
      (stored servers, backups, catalog cache) and we deliberately don't
      wipe it. Verify this matches expectations.

---

## Fallback paths the code depends on

If any of these are wrong on real Windows machines, fix them in
`src-tauri/src/paths.rs` and `src-tauri/src/commands.rs` respectively:

| Purpose                       | Path                                                      |
|-------------------------------|-----------------------------------------------------------|
| Claude Desktop config         | `%APPDATA%\Claude\claude_desktop_config.json`             |
| Claude Code settings          | `%USERPROFILE%\.claude\settings.json`                     |
| Configonaut storage           | `%APPDATA%\Configonaut\`                                  |
| Claude Desktop executable     | `%LOCALAPPDATA%\AnthropicClaude\Claude.exe`               |

The first three use `dirs::config_dir()` which returns `%APPDATA%`
(Roaming) on Windows. The executable path is hardcoded in
`restart_claude_desktop` — if Anthropic ever moves the MSI install
target, that's the single place to update.
