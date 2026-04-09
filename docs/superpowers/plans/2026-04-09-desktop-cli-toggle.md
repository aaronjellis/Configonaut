# Desktop/CLI Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a global Desktop/CLI toggle to Configonaut so users can switch between managing Claude Desktop and Claude Code configurations.

**Architecture:** Add an `AppMode` enum (`.desktop`, `.cli`) to ConfigManager. Convert static path properties to computed instance properties that switch on mode. Add a segmented control to the sidebar. Disable Hooks/Agents/Skills views in Desktop mode. Separate stored servers and backups per mode with migration for existing data.

**Tech Stack:** Swift, SwiftUI, Foundation, UserDefaults

---

### Task 1: Add AppMode enum and mode property to ConfigManager

**Files:**
- Modify: `Sources/ConfigManager.swift:1-12` (add enum after imports)
- Modify: `Sources/ConfigManager.swift:80-93` (add mode property)

- [ ] **Step 1: Add AppMode enum after the imports**

Add at line 3, before the `ServerEntry` struct:

```swift
enum AppMode: String, CaseIterable {
    case desktop = "Desktop"
    case cli = "CLI"
}
```

- [ ] **Step 2: Add mode property to ConfigManager**

Add after line 90 (`@Published var needsRestart = false`), before the `lastBackupDate` line:

```swift
@Published var mode: AppMode {
    didSet {
        if oldValue != mode {
            UserDefaults.standard.set(mode.rawValue, forKey: "appMode")
            needsRestart = false
            lastBackupHash = nil
            reloadAll()
        }
    }
}
```

- [ ] **Step 3: Initialize mode from UserDefaults in init**

Replace the current `init() { reloadAll() }` at line 134 with:

```swift
init() {
    let saved = UserDefaults.standard.string(forKey: "appMode") ?? AppMode.desktop.rawValue
    self.mode = AppMode(rawValue: saved) ?? .desktop
    reloadAll()
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cd /Users/aaronellis/Configonaut && swift build 2>&1 | tail -5`
Expected: Build Succeeded

- [ ] **Step 5: Commit**

```bash
git add Sources/ConfigManager.swift
git commit -m "feat: add AppMode enum and persisted mode property to ConfigManager"
```

---

### Task 2: Convert static path properties to mode-aware computed properties

**Files:**
- Modify: `Sources/ConfigManager.swift:96-132` (path properties)
- Modify: `Sources/ConfigManager.swift` (all `Self.` references to paths)

The key insight: in Desktop mode, MCP servers come from `claude_desktop_config.json`. In CLI mode, MCP servers come from `~/.claude/settings.json` (same file as hooks). The `configURL` property must switch based on mode. Other paths like `globalSettingsURL`, `commandsDir`, etc. are always the same regardless of mode.

- [ ] **Step 1: Replace the static path block with instance computed properties**

Replace lines 94-132 (from `// MARK: - File Paths` through the `pluginsDir` static let) with:

```swift
// MARK: - File Paths

private static let home = FileManager.default.homeDirectoryForCurrentUser

/// The config file that holds mcpServers for the current mode
var configURL: URL {
    switch mode {
    case .desktop:
        return Self.home
            .appendingPathComponent("Library/Application Support/Claude/claude_desktop_config.json")
    case .cli:
        return Self.home.appendingPathComponent(".claude/settings.json")
    }
}

/// Configonaut's own storage directory
var storageDir: URL {
    Self.home.appendingPathComponent("Library/Application Support/Configonaut")
}

/// Stored (inactive) servers file — separate per mode
var storedURL: URL {
    storageDir.appendingPathComponent("stored_servers_\(mode.rawValue.lowercased()).json")
}

/// Backup directory — separate per mode
var backupDir: URL {
    storageDir.appendingPathComponent("backups/\(mode.rawValue.lowercased())")
}

/// Claude Code global settings (always ~/.claude/settings.json, used for hooks & plugins)
static let globalSettingsURL: URL = {
    home.appendingPathComponent(".claude/settings.json")
}()

static let commandsDir: URL = {
    home.appendingPathComponent(".claude/commands")
}()

static let skillsDir: URL = {
    home.appendingPathComponent(".claude/skills")
}()

static let personalAgentsDir: URL = {
    home.appendingPathComponent(".claude/agents")
}()

static let pluginsDir: URL = {
    home.appendingPathComponent(".claude/plugins/marketplaces/claude-plugins-official/plugins")
}()
```

- [ ] **Step 2: Update all Self. references to instance properties**

The following properties changed from `static` to instance: `configURL`, `storageDir`, `storedURL`, `backupDir`. All references using `Self.configURL`, `Self.storageDir`, `Self.storedURL`, `Self.backupDir` must change to `self.configURL` (or just drop `Self.`).

In `loadConfigRoot()` (around line 868):
```swift
private func loadConfigRoot() -> [String: Any] {
    guard let data = try? Data(contentsOf: configURL),
          let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
    else { return [:] }
    return obj
}
```

In `loadStoredRoot()` (around line 875):
```swift
private func loadStoredRoot() -> [String: Any] {
    guard let data = try? Data(contentsOf: storedURL),
          let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
    else { return [:] }
    return obj
}
```

In `saveConfig()` (around line 890):
```swift
@discardableResult
private func saveConfig(_ root: [String: Any]) -> Bool {
    do {
        createBackup()
        let dir = configURL.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let data = try JSONSerialization.data(
            withJSONObject: root,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        try data.write(to: configURL, options: .atomic)
        return true
    } catch {
        setStatus("Save error: \(error.localizedDescription)", isError: true)
        return false
    }
}
```

In `saveStored()` (around line 907):
```swift
@discardableResult
private func saveStored(_ stored: [String: Any]) -> Bool {
    do {
        createBackup()
        try Self.ensureSecureDirectory(storageDir)
        let data = try JSONSerialization.data(
            withJSONObject: stored,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        try data.write(to: storedURL, options: .atomic)
        Self.lockFile(storedURL)
        return true
    } catch {
        setStatus("Storage error: \(error.localizedDescription)", isError: true)
        return false
    }
}
```

In `createBackup()` (around line 939):
```swift
func createBackup() {
    guard FileManager.default.fileExists(atPath: configURL.path) else { return }
    guard let currentData = try? Data(contentsOf: configURL) else { return }
    let currentHash = currentData.hashValue
    if currentHash == lastBackupHash { return }
    if let last = lastBackupDate, Date().timeIntervalSince(last) < 300 { return }
    writeBackup(data: currentData, hash: currentHash)
}
```

In `forceBackup()` (around line 954):
```swift
func forceBackup() {
    guard FileManager.default.fileExists(atPath: configURL.path),
          let data = try? Data(contentsOf: configURL)
    else { return }
    writeBackup(data: data, hash: data.hashValue)
}
```

In `writeBackup()` (around line 961):
```swift
private func writeBackup(data: Data, hash: Int) {
    do {
        try Self.ensureSecureDirectory(backupDir)
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd_HH-mm-ss"
        let timestamp = formatter.string(from: Date())
        let backupURL = backupDir.appendingPathComponent("config_\(timestamp).json")
        try data.write(to: backupURL, options: .atomic)
        Self.lockFile(backupURL)

        lastBackupDate = Date()
        lastBackupHash = hash

        let backups = try FileManager.default.contentsOfDirectory(
            at: backupDir, includingPropertiesForKeys: nil
        ).filter { $0.pathExtension == "json" }
         .sorted { $0.lastPathComponent < $1.lastPathComponent }

        if backups.count > 30 {
            for old in backups.prefix(backups.count - 30) {
                try? FileManager.default.removeItem(at: old)
            }
        }
    } catch {
        // Backup failure should not block saves
    }
}
```

In `loadBackups()` (around line 421):
```swift
func loadBackups() {
    guard let files = try? FileManager.default.contentsOfDirectory(
        at: backupDir,
        includingPropertiesForKeys: [.fileSizeKey, .creationDateKey]
    ) else {
        backupFiles = []
        return
    }
    // ... rest unchanged
}
```

In `restoreBackup()` (around line 443):
```swift
func restoreBackup(_ backup: BackupFile) -> Bool {
    do {
        forceBackup()
        let data = try Data(contentsOf: backup.url)
        guard (try? JSONSerialization.jsonObject(with: data)) != nil else {
            setStatus("Backup file contains invalid JSON.", isError: true)
            return false
        }
        try data.write(to: configURL, options: .atomic)
        loadActive()
        needsRestart = true
        setStatus("Restored backup from \(backup.formattedDate).", isError: false)
        return true
    } catch {
        setStatus("Restore failed: \(error.localizedDescription)", isError: true)
        return false
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /Users/aaronellis/Configonaut && swift build 2>&1 | tail -5`
Expected: Build Succeeded

- [ ] **Step 4: Commit**

```bash
git add Sources/ConfigManager.swift
git commit -m "refactor: convert static paths to mode-aware instance properties"
```

---

### Task 3: Add mode toggle to ContentView sidebar

**Files:**
- Modify: `Sources/ContentView.swift:93-173` (sidebarView)

- [ ] **Step 1: Add the mode toggle picker after the app branding block**

In `sidebarView`, after the closing of the app branding `HStack` (after `.padding(.bottom, 18)` around line 135), add:

```swift
// Desktop / CLI toggle
Picker("Mode", selection: $config.mode) {
    ForEach(AppMode.allCases, id: \.self) { mode in
        Text(mode.rawValue).tag(mode)
    }
}
.pickerStyle(.segmented)
.padding(.horizontal, 14)
.padding(.bottom, 12)
```

- [ ] **Step 2: Update subtitle text to reflect mode**

In the app branding VStack, change the subtitle from the static "Config Explorer" to be mode-aware. Replace:

```swift
Text("Config Explorer")
    .font(.system(size: 10, weight: .medium))
    .foregroundStyle(Theme.green.opacity(0.5))
```

with:

```swift
Text(config.mode == .desktop ? "Desktop Config" : "CLI Config")
    .font(.system(size: 10, weight: .medium))
    .foregroundStyle(config.mode == .desktop ? Theme.green.opacity(0.5) : Theme.blue.opacity(0.5))
```

- [ ] **Step 3: Verify it compiles and run**

Run: `cd /Users/aaronellis/Configonaut && swift build 2>&1 | tail -5`
Expected: Build Succeeded

Run: `cd /Users/aaronellis/Configonaut && open .build/release/Configonaut` to visually verify the toggle appears in the sidebar.

- [ ] **Step 4: Commit**

```bash
git add Sources/ContentView.swift
git commit -m "feat: add Desktop/CLI mode toggle to sidebar"
```

---

### Task 4: Disable CLI-only views in Desktop mode

**Files:**
- Modify: `Sources/ContentView.swift` (detailView, sidebarItem, badgeCount)
- Modify: `Sources/SupportViews.swift` (HooksView)
- Modify: `Sources/AgentsView.swift`
- Modify: `Sources/SkillsView.swift`

In Desktop mode, Hooks, Agents, and Skills should show a "Not available for Claude Desktop" placeholder. The sidebar items remain visible but appear disabled.

- [ ] **Step 1: Add a helper to check if a section is available in current mode**

In `ContentView.swift`, add this method after `badgeCount(for:)`:

```swift
private func isSectionAvailable(_ section: SidebarSection) -> Bool {
    switch section {
    case .servers, .backups: return true
    case .hooks, .agents, .skills: return config.mode == .cli
    }
}

/// Badge counts should return 0 for unavailable sections to avoid confusion
private func displayBadgeCount(for item: SidebarSection) -> Int {
    guard isSectionAvailable(item) else { return 0 }
    return badgeCount(for: item)
}
```

- [ ] **Step 2: Dim unavailable sidebar items**

In the `sidebarItem` method, wrap the entire button content with an opacity modifier. After the `.buttonStyle(.plain)` line, add:

```swift
.opacity(isSectionAvailable(item) ? 1.0 : 0.4)
```

In the `sidebarItem` method, also change `badgeCount(for: item)` to `displayBadgeCount(for: item)` so that dimmed sections don't show misleading counts.

Also, disable the selection highlight for unavailable items. In the button action, change:

```swift
Button {
    withAnimation(.easeInOut(duration: 0.2)) { selection = item }
}
```

to:

```swift
Button {
    guard isSectionAvailable(item) else { return }
    withAnimation(.easeInOut(duration: 0.2)) { selection = item }
}
```

- [ ] **Step 3: Add unavailable placeholder view**

In `ContentView.swift`, add a reusable placeholder view after the `badgeCount` method:

```swift
private func unavailablePlaceholder(for section: SidebarSection) -> some View {
    VStack(spacing: 16) {
        Image(systemName: section.icon)
            .font(.system(size: 40))
            .foregroundStyle(.quaternary)
        Text("Not available for Claude Desktop")
            .font(.system(size: 15, weight: .medium))
            .foregroundStyle(.secondary)
        Text("\(section.rawValue) are only used by Claude Code (CLI).")
            .font(.system(size: 12))
            .foregroundStyle(.tertiary)
            .multilineTextAlignment(.center)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
}
```

- [ ] **Step 4: Use placeholder in detailView**

Replace the `detailView` computed property:

```swift
@ViewBuilder
private var detailView: some View {
    switch selection {
    case .servers:
        MCPView(config: config)
    case .hooks:
        if config.mode == .cli {
            HooksView(config: config)
        } else {
            unavailablePlaceholder(for: .hooks)
        }
    case .agents:
        if config.mode == .cli {
            AgentsView(config: config)
        } else {
            unavailablePlaceholder(for: .agents)
        }
    case .skills:
        if config.mode == .cli {
            SkillsView(config: config)
        } else {
            unavailablePlaceholder(for: .skills)
        }
    case .backups:
        BackupsView(config: config)
    }
}
```

- [ ] **Step 5: Switch to an available section when toggling to Desktop**

When the user switches to Desktop mode while viewing Hooks/Agents/Skills, auto-select MCP Servers. Add an `.onChange` modifier to the body's `ZStack` in `ContentView`:

```swift
.onChange(of: config.mode) { _, newMode in
    if newMode == .desktop && !isSectionAvailable(selection) {
        withAnimation { selection = .servers }
    }
}
```

- [ ] **Step 6: Verify it compiles**

Run: `cd /Users/aaronellis/Configonaut && swift build 2>&1 | tail -5`
Expected: Build Succeeded

- [ ] **Step 7: Commit**

```bash
git add Sources/ContentView.swift
git commit -m "feat: disable Hooks/Agents/Skills views in Desktop mode"
```

---

### Task 5: Update restart banner to show correct app name

**Files:**
- Modify: `Sources/MCPView.swift` (restart banner text)

The restart banner currently says "Restart Claude Desktop". In CLI mode it should say "Restart Claude Code".

- [ ] **Step 1: Make MCPView mode-aware in restart banner**

In MCPView.swift line 262, the restart banner text is:

```swift
Text("Changes saved! Quit and reopen Claude Desktop to apply.")
```

Replace with:

```swift
Text("Changes saved! Quit and reopen \(config.mode == .desktop ? "Claude Desktop" : "Claude Code") to apply.")
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /Users/aaronellis/Configonaut && swift build 2>&1 | tail -5`
Expected: Build Succeeded

- [ ] **Step 3: Commit**

```bash
git add Sources/MCPView.swift
git commit -m "feat: update restart banner text based on Desktop/CLI mode"
```

---

### Task 6: Update Finder/folder reveal for mode-aware config path

**Files:**
- Modify: `Sources/MCPView.swift` (openConfigInFinder method)

The "open in Finder" button should reveal the correct config file based on mode.

- [ ] **Step 1: Update openConfigInFinder**

Find `openConfigInFinder()` in MCPView.swift (around line 841-848). It currently references `ConfigManager.configURL`. Since `configURL` is now an instance property, update it to use `config.configURL`:

```swift
private func openConfigInFinder() {
    let url = config.configURL
    if FileManager.default.fileExists(atPath: url.path) {
        NSWorkspace.shared.activateFileViewerSelecting([url])
    } else {
        NSWorkspace.shared.open(url.deletingLastPathComponent())
    }
}
```

- [ ] **Step 2: Update any other references to ConfigManager.configURL**

Search for `ConfigManager.configURL` throughout the file and replace with `config.configURL`. There may be references in the detail panel path display as well.

- [ ] **Step 3: Verify it compiles**

Run: `cd /Users/aaronellis/Configonaut && swift build 2>&1 | tail -5`
Expected: Build Succeeded

- [ ] **Step 4: Commit**

```bash
git add Sources/MCPView.swift
git commit -m "feat: update Finder reveal to use mode-aware config path"
```

---

### Task 7: Add migration for existing stored servers and backups

**Files:**
- Modify: `Sources/ConfigManager.swift` (init, add migration method)

Existing data from before the toggle was added should be treated as Desktop data. On first launch, migrate `stored_servers.json` → `stored_servers_desktop.json` and `backups/*.json` → `backups/desktop/*.json`.

- [ ] **Step 1: Add migration method to ConfigManager**

Add before `reloadAll()`:

```swift
/// One-time migration: move pre-toggle data into desktop-specific paths
private func migrateIfNeeded() {
    let key = "migrated_v2_mode_split"
    guard !UserDefaults.standard.bool(forKey: key) else { return }
    defer { UserDefaults.standard.set(true, forKey: key) }

    let fm = FileManager.default

    // Migrate stored_servers.json → stored_servers_desktop.json
    let oldStored = storageDir.appendingPathComponent("stored_servers.json")
    let newStored = storageDir.appendingPathComponent("stored_servers_desktop.json")
    if fm.fileExists(atPath: oldStored.path) && !fm.fileExists(atPath: newStored.path) {
        try? fm.moveItem(at: oldStored, to: newStored)
    }

    // Migrate backups/*.json → backups/desktop/*.json
    let oldBackupDir = storageDir.appendingPathComponent("backups")
    let newBackupDir = storageDir.appendingPathComponent("backups/desktop")
    // Only migrate if the old flat backup dir has JSON files directly in it
    if let items = try? fm.contentsOfDirectory(at: oldBackupDir, includingPropertiesForKeys: nil) {
        let jsonFiles = items.filter { $0.pathExtension == "json" }
        if !jsonFiles.isEmpty {
            try? fm.createDirectory(at: newBackupDir, withIntermediateDirectories: true)
            for file in jsonFiles {
                let dest = newBackupDir.appendingPathComponent(file.lastPathComponent)
                try? fm.moveItem(at: file, to: dest)
            }
        }
    }

    // Create CLI backup dir
    let cliBackupDir = storageDir.appendingPathComponent("backups/cli")
    try? fm.createDirectory(at: cliBackupDir, withIntermediateDirectories: true)
}
```

- [ ] **Step 2: Call migration in init**

Update `init()` to call `migrateIfNeeded()` before `reloadAll()`:

```swift
init() {
    let saved = UserDefaults.standard.string(forKey: "appMode") ?? AppMode.desktop.rawValue
    self.mode = AppMode(rawValue: saved) ?? .desktop
    migrateIfNeeded()
    reloadAll()
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /Users/aaronellis/Configonaut && swift build 2>&1 | tail -5`
Expected: Build Succeeded

- [ ] **Step 4: Commit**

```bash
git add Sources/ConfigManager.swift
git commit -m "feat: add one-time migration for stored servers and backups to per-mode paths"
```

---

### Task 8: Show active config file path in MCP header

**Files:**
- Modify: `Sources/MCPView.swift:133-135` (header subtitle)

Replace the static subtitle text with the active config file path so users can see which file they're editing.

- [ ] **Step 1: Replace the header subtitle**

In MCPView.swift at line 133, replace:

```swift
Text("Drag servers between Active and Inactive to toggle them.")
    .font(.caption)
    .foregroundStyle(.secondary)
```

with:

```swift
Text(config.configURL.path.replacingOccurrences(of: NSHomeDirectory(), with: "~"))
    .font(.system(size: 10, design: .monospaced))
    .foregroundStyle(.tertiary)
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /Users/aaronellis/Configonaut && swift build 2>&1 | tail -5`
Expected: Build Succeeded

- [ ] **Step 3: Commit**

```bash
git add Sources/MCPView.swift
git commit -m "feat: show active config file path in MCP view header"
```

---

### Task 9: Build, run, and manually verify

**Files:** None (testing only)

- [ ] **Step 1: Clean build**

Run: `cd /Users/aaronellis/Configonaut && swift build -c release 2>&1 | tail -5`
Expected: Build Succeeded

- [ ] **Step 2: Build the app bundle**

Run: `cd /Users/aaronellis/Configonaut && bash build.sh`
Expected: "Build complete: .../Configonaut.app"

- [ ] **Step 3: Manual smoke test checklist**

Open the app and verify:
1. Toggle appears in sidebar between app icon and navigation items
2. Default mode is "Desktop" (existing behavior preserved)
3. Switching to CLI dims Hooks/Agents/Skills sidebar items
4. Clicking a dimmed item does nothing
5. If viewing Hooks when switching to Desktop, auto-selects MCP Servers
6. MCP Servers view shows correct config path for each mode
7. Restart banner says "Claude Desktop" or "Claude Code" based on mode
8. Folder icon opens correct config file location
9. Mode persists after quitting and reopening
10. Existing stored servers still appear in Desktop mode (migration worked)

- [ ] **Step 4: Commit any fixes if needed**

```bash
git add -A
git commit -m "fix: address issues found during manual testing"
```

---

### Task 10: Update version to 1.2.0

**Files:**
- Modify: `Sources/ContentView.swift` (version display)
- Modify: `build.sh` (Info.plist version)
- Modify: `package-dmg.sh` (VERSION variable)

- [ ] **Step 1: Update version in build.sh Info.plist**

In `build.sh`, change both `CFBundleVersion` and `CFBundleShortVersionString` from `1.1.0` to `1.2.0`:

```xml
<key>CFBundleVersion</key>
<string>1.2.0</string>
<key>CFBundleShortVersionString</key>
<string>1.2.0</string>
```

- [ ] **Step 2: Update version in package-dmg.sh**

Change `VERSION="1.1.0"` to `VERSION="1.2.0"`.

- [ ] **Step 3: Update version footer in ContentView.swift**

Change `Text("v1.1.0")` to `Text("v1.2.0")`.

- [ ] **Step 4: Verify it compiles**

Run: `cd /Users/aaronellis/Configonaut && swift build 2>&1 | tail -5`
Expected: Build Succeeded

- [ ] **Step 5: Commit**

```bash
git add Sources/ContentView.swift build.sh package-dmg.sh
git commit -m "chore: bump version to 1.2.0 for Desktop/CLI toggle feature"
```
