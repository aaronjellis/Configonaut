# Configonaut Windows — PC Setup & Build Instructions

This document walks you through building and running the Windows port on your PC after transferring the branch from macOS.

## 0. What's in this folder

A complete WinUI 3 / Windows App SDK 1.6 source tree targeting `.NET 8`. Generated end-to-end on macOS, so **nothing has been compiled yet** — that's your job. You should expect the first build to surface a few minor fixups (missing using statements, stray cosmetic issues); they should be easy to resolve.

Layout:

```
Configonaut-Windows/
  Configonaut.sln
  Configonaut/
    Configonaut.csproj
    App.xaml / App.xaml.cs
    app.manifest
    Package.appxmanifest
    MainWindow.xaml / MainWindow.xaml.cs
    Assets/AppIcon.png
    Models/           (AppMode, ServerEntry, HookRule, AgentEntry, SkillEntry, BackupFile)
    Services/         (PathResolver, JsonHelper, ConfigManager.Load, ConfigManager.Mutations)
    Views/            (MCPPage, HooksPage, AgentsPage, SkillsPage, BackupsPage)
    Theme/NeonTheme.xaml
    Converters/Converters.cs
  Configonaut.Tests/
    Configonaut.Tests.csproj
    JsonHelperTests.cs / ModelTests.cs / PathResolverTests.cs
```

## 1. PC prerequisites

Install on the Windows machine before anything else:

1. **Visual Studio 2022** (17.8 or newer) — Community edition is fine
   - Workloads: **.NET desktop development** + **Windows application development**
   - Individual components: **Windows App SDK C# Templates**, **.NET 8 SDK**
2. **Windows 10 SDK 10.0.22621.0** (or newer) — usually installed with the VS workload above
3. **Git for Windows**

Verify:
```powershell
dotnet --list-sdks          # should show 8.0.x
dotnet workload list        # optional: confirms WinUI workload if you installed via CLI
```

## 2. Transfer the branch from mac to PC

On the mac, the work lives on branch **`feature/windows-port`** (new branch, separate from `feature/desktop-cli-toggle`). Choose whichever transfer method is easiest:

### Option A — Push to the existing remote (recommended)
```bash
# On macOS:
cd /Users/aaronellis/Configonaut
git checkout feature/windows-port
git push -u origin feature/windows-port
```
```powershell
# On PC:
cd C:\path\to\your\Configonaut
git fetch origin
git checkout feature/windows-port
```

### Option B — Bundle file over USB/AirDrop
```bash
# On macOS:
cd /Users/aaronellis/Configonaut
git bundle create ~/Desktop/configonaut-windows.bundle feature/windows-port
```
Copy the `.bundle` to the PC, then:
```powershell
# On PC (inside an existing clone):
git fetch C:\path\to\configonaut-windows.bundle feature/windows-port:feature/windows-port
git checkout feature/windows-port
```

### Option C — Fresh clone from bundle
```powershell
git clone C:\path\to\configonaut-windows.bundle Configonaut
cd Configonaut
git checkout feature/windows-port
```

## 3. First build (the moment of truth)

```powershell
cd Configonaut-Windows
dotnet restore
dotnet build Configonaut.sln --configuration Debug
```

Expected outcomes and how to handle them:

- **Clean build** — ship it. Proceed to step 4.
- **Missing `using System.IO;` / `using System.Linq;`** — add at top of the offending file. These weren't strictly necessary on macOS because I was writing files blind; the implicit usings in the csproj may or may not cover every case.
- **`CommunityToolkit.WinUI.Controls.Segmented` not found** — check `Configonaut.csproj` includes `CommunityToolkit.WinUI.Controls.Segmented` as a PackageReference. If missing: `dotnet add Configonaut/Configonaut.csproj package CommunityToolkit.WinUI.Controls.Segmented`.
- **XAML: `XamlControlsResources` not found** — this lives in `Microsoft.UI.Xaml.Controls`. Should be pulled in automatically by the Windows App SDK. If not, add `xmlns:controls="using:Microsoft.UI.Xaml.Controls"` at the top of `App.xaml` and prefix the tag.
- **`InitializeComponent()` doesn't exist** — the XAML compiler hasn't run yet. Do a clean rebuild: `dotnet clean` then `dotnet build`. Usually clears up after the first successful code-gen pass.

## 4. Run it

```powershell
dotnet run --project Configonaut\Configonaut.csproj
```

or press **F5** in Visual Studio after opening `Configonaut.sln`.

Smoke test checklist:

- [ ] Window opens with dark neon theme, sidebar on the left
- [ ] Desktop/CLI mode toggle at top of sidebar works and persists (relaunch to confirm)
- [ ] **MCP Servers** — existing config loads, can drag between Active/Stored columns, "Add Server" panel parses JSON
- [ ] **Hooks** — existing hooks list, toggle button changes color
- [ ] **Agents** — personal + plugin agents listed, search filters, New Agent creates a file
- [ ] **Skills** — personal + plugin skills listed, Command/Skill type toggle works in create mode
- [ ] **Backups** — backup list shows, preview pane loads content on selection, Restore confirmation dialog appears

If a page crashes: check the Debug output for a XAML parse error — the filename + line number will point at it.

## 5. Commit history (already on the branch)

The branch was created on macOS with one commit per task. You should not need to make any new commits unless you fix something during the first build. The existing commits are:

| # | Commit | What's in it |
|---|---|---|
| 1 | `chore: scaffold WinUI 3 solution with test project` | Task 1-2: .sln, .csproj, .gitignore, Tests project |
| 2 | `feat: add data models for MCP servers, hooks, agents, skills, and backups` | Task 3: Models/*, PathResolver |
| 3 | `feat: add JsonHelper with atomic writes and parser` | Task 4: Services/JsonHelper |
| 4 | `feat: add ConfigManager with load, save, and mode persistence` | Task 5: ConfigManager.Load.cs |
| 5 | `feat: add mutation methods for servers, hooks, agents, skills, backups` | Task 6: ConfigManager.Mutations.cs |
| 6 | `feat: add neon dark theme and value converters` | Task 7: NeonTheme, Converters |
| 7 | `feat: add MainWindow shell with sidebar navigation and mode toggle` | Task 8: MainWindow |
| 8 | `feat: add MCPPage with two-column drag-drop, JSON editor, and add-server panel` | Task 9 |
| 9 | `feat: add HooksPage with hook list, JSON editor, and toggle support` | Task 10 |
| 10 | `feat: add AgentsPage with search, plugin grouping, create, and edit` | Task 11 |
| 11 | `feat: add SkillsPage with search, type picker, toggle, and create support` | Task 12 |
| 12 | `feat: add BackupsPage with backup list, preview, diff, and restore` | Task 13 |
| 13 | `feat: add App entry point, manifest, and MSIX packaging config` | Task 14 |
| 14 | `chore: Configonaut Windows v1.0.0 — initial release` | Capstone (icon copy, SETUP_ON_PC.md) |

> **If the branch was transferred without commits** (e.g., the files were copied as a zip rather than via git): initialize from scratch on the PC using the commit list above. Each commit's files are scoped by folder, so `git add` the relevant paths and use the exact commit messages above.

## 6. Tests

```powershell
dotnet test Configonaut.Tests\Configonaut.Tests.csproj
```

Unit tests cover `JsonHelper`, `PathResolver`, and model equality. They target `net8.0` (not the WinUI target), so they should run anywhere.

## 7. Creating an MSIX package (optional, later)

Once the build is clean and the app runs from F5, you can produce an installer:

1. Right-click the `Configonaut` project in Visual Studio → **Publish** → **Create App Packages…**
2. Choose **Sideloading**
3. Generate a self-signed test certificate (or use an existing code-signing cert)
4. Build configurations: **Release** / **x64** (and **arm64** if you care about Surface / Copilot+ PCs)
5. Output: `Configonaut-Windows\Configonaut\AppPackages\Configonaut_1.0.0.0_Test\`

Install the resulting `.msix` by double-clicking it — Windows will prompt to trust the test cert first.

## 8. Known shortcuts / follow-ups

These weren't in scope for v1.0.0 but are worth tracking:

- **No plugin marketplace browser** — the Windows build inherits the macOS decision to ship plugin browsing as a follow-up. Users can still enable/disable installed plugins via the Agents/Skills pages.
- **Hook "Add new hook" UI** is not wired up — Hooks page can view/edit/toggle existing hooks but doesn't have a create flow. Low priority; add when needed.
- **No drag-drop reordering within a single column** on MCP page — only cross-column moves. Matches macOS behavior.
- **No localization** — all strings are inline English.

## 9. If something is fundamentally broken

If a whole file won't compile and it's not a quick fix, it's fine to either (a) push the broken state up to a `wip/` branch and share the error, or (b) delete the offending view temporarily and add it to the sidebar `Visibility="Collapsed"` to get the rest of the app running first. The service layer (`ConfigManager`, `JsonHelper`, `PathResolver`) is where the hard logic lives — the Views are comparatively thin and easy to fix incrementally.

Good luck on the PC. The hardest part is over.
