import SwiftUI
import AppKit

// MARK: - Hooks View

struct HooksView: View {
    @ObservedObject var config: ConfigManager
    @State private var selectedHook: HookRule?
    @State private var editingJSON = ""
    @State private var bottomPanelHeight: CGFloat = 260

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 8) {
                        Text("Hooks")
                            .font(.title3.bold())
                        Text("\(config.hookRules.count)")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(Theme.blue)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 2)
                            .background(Theme.blue.opacity(0.12), in: Capsule())
                    }
                    Text("Automation triggers that run when Claude performs actions.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button(action: { config.loadHooks() }) {
                    Image(systemName: "arrow.clockwise")
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)

                Button(action: openSettingsInFinder) {
                    Image(systemName: "folder")
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
            }
            .padding(16)

            Rectangle().fill(Theme.subtleBorder).frame(height: 1)

            // Main content area
            GeometryReader { _ in
                VStack(spacing: 0) {
                    if config.hookRules.isEmpty {
                        hooksEmptyState
                    } else {
                        hooksList
                    }

                    // Bottom editor panel
                    if let hook = selectedHook {
                        HDivider(
                            position: $bottomPanelHeight,
                            range: 150...500
                        )
                        .padding(.horizontal, 16)

                        editorPanel(hook)
                            .frame(height: bottomPanelHeight)
                            .padding(.horizontal, 16)
                            .padding(.bottom, 6)
                    }
                }
            }

            Rectangle().fill(Theme.subtleBorder).frame(height: 1)

            HStack {
                Image(systemName: "info.circle")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                Text("Defined in ~/.claude/settings.json")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                Spacer()
                let enabled = config.hookRules.filter(\.isEnabled).count
                let disabled = config.hookRules.count - enabled
                if !config.hookRules.isEmpty {
                    Text("\(enabled) on, \(disabled) off")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
            .background(Color.black.opacity(0.1))
        }
    }

    // MARK: - Editor Panel

    private func editorPanel(_ hook: HookRule) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            // Header
            HStack(spacing: 10) {
                GlowDot(color: hook.isEnabled ? eventColor(hook.event) : .gray, size: 10)

                Text(hook.event)
                    .font(.system(size: 15, weight: .semibold, design: .monospaced))

                Text(hook.isEnabled ? "ON" : "OFF")
                    .font(.caption.weight(.bold))
                    .foregroundStyle(hook.isEnabled ? Theme.green : Theme.red)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)
                    .background(
                        (hook.isEnabled ? Theme.green : Theme.red).opacity(0.1),
                        in: Capsule()
                    )

                if hook.matcher != "*" {
                    Text(hook.matcher)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.secondary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(Color.white.opacity(0.05), in: Capsule())
                }

                Spacer()

                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        selectedHook = nil
                    }
                } label: {
                    Image(systemName: "xmark")
                        .font(.caption2.weight(.bold))
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
            }

            // Editable JSON
            TextEditor(text: $editingJSON)
                .font(.system(size: 11, design: .monospaced))
                .scrollContentBackground(.hidden)
                .padding(8)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.black.opacity(0.35))
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .strokeBorder(Theme.blue.opacity(0.3), lineWidth: 1)
                )

            // Action buttons
            HStack(spacing: 8) {
                Button {
                    _ = config.updateHookRule(hook, newJSON: editingJSON)
                    // Refresh selection
                    if let updated = config.hookRules.first(where: { $0.event == hook.event && $0.matcher == hook.matcher }) {
                        selectedHook = updated
                        editingJSON = config.hookRuleJSON(updated)
                    }
                } label: {
                    Label("Save", systemImage: "checkmark.circle.fill")
                        .font(.caption.weight(.semibold))
                }
                .buttonStyle(GradientButtonStyle(
                    colors: [Color(hex: "22C55E"), Theme.green],
                    glowColor: Theme.green,
                    isEnabled: true
                ))
                .keyboardShortcut("s", modifiers: .command)

                Button {
                    config.toggleHook(hook)
                    // Refresh selection
                    if let updated = config.hookRules.first(where: { $0.event == hook.event && $0.matcher == hook.matcher }) {
                        selectedHook = updated
                        editingJSON = config.hookRuleJSON(updated)
                    }
                } label: {
                    Label(
                        hook.isEnabled ? "Disable" : "Enable",
                        systemImage: hook.isEnabled ? "moon.fill" : "bolt.fill"
                    )
                    .font(.caption.weight(.medium))
                    .foregroundStyle(hook.isEnabled ? Theme.red : Theme.green)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(
                        (hook.isEnabled ? Theme.red : Theme.green).opacity(0.1),
                        in: RoundedRectangle(cornerRadius: 6)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 6)
                            .strokeBorder(
                                (hook.isEnabled ? Theme.red : Theme.green).opacity(0.2),
                                lineWidth: 0.5
                            )
                    )
                }
                .buttonStyle(.plain)

                Spacer()

                Text("~/.claude/settings.json")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(14)
        .glassCard(radius: 12)
    }

    // MARK: - Hooks List

    private var hooksList: some View {
        ScrollView {
            LazyVStack(spacing: 8) {
                ForEach(config.hookRules) { rule in
                    hookCard(rule)
                }
            }
            .padding(14)
        }
    }

    private func hookCard(_ rule: HookRule) -> some View {
        HStack(spacing: 8) {
            GlowDot(color: rule.isEnabled ? eventColor(rule.event) : .gray, size: 7)

            Text(rule.event)
                .font(.system(size: 13, weight: .medium, design: .monospaced))
                .foregroundStyle(rule.isEnabled ? eventColor(rule.event) : .secondary)

            if rule.matcher != "*" {
                Text(rule.matcher)
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Color.white.opacity(0.05), in: Capsule())
            }

            // Command preview
            if let firstCmd = rule.commands.first {
                Text(firstCmd)
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }

            Spacer()

            Text(rule.isEnabled ? "ON" : "OFF")
                .font(.system(size: 9, weight: .bold, design: .rounded))
                .foregroundStyle(rule.isEnabled ? Theme.green : Theme.red)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(
                    (rule.isEnabled ? Theme.green : Theme.red).opacity(0.12),
                    in: RoundedRectangle(cornerRadius: 4)
                )

            Image(systemName: selectedHook?.id == rule.id ? "chevron.down" : "chevron.right")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
        .background(
            RoundedRectangle(cornerRadius: 8)
                .fill(selectedHook?.id == rule.id ? eventColor(rule.event).opacity(0.08) : Theme.cardFill)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .strokeBorder(
                    selectedHook?.id == rule.id ? eventColor(rule.event).opacity(0.2) : Theme.cardBorder,
                    lineWidth: 0.5
                )
        )
        .contentShape(Rectangle())
        .onTapGesture {
            withAnimation(.easeInOut(duration: 0.15)) {
                if selectedHook?.id == rule.id {
                    selectedHook = nil
                } else {
                    selectedHook = rule
                    editingJSON = config.hookRuleJSON(rule)
                }
            }
        }
        .opacity(rule.isEnabled ? 1 : 0.6)
    }

    // MARK: - Empty State

    private var hooksEmptyState: some View {
        VStack(spacing: 20) {
            Spacer()

            ZStack {
                Circle()
                    .fill(Theme.blue.opacity(0.08))
                    .frame(width: 70, height: 70)
                    .blur(radius: 10)
                Image(systemName: "arrow.triangle.branch")
                    .font(.system(size: 34))
                    .foregroundStyle(Theme.blue.opacity(0.5))
            }

            Text("No Hooks Configured")
                .font(.title3.bold())

            Text("Hooks run custom commands when Claude performs actions.\nGreat for linting, formatting, or validation.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 380)

            VStack(alignment: .leading, spacing: 10) {
                Text("HOOK TYPES")
                    .font(.caption2.weight(.bold))
                    .foregroundStyle(.tertiary)
                    .tracking(0.8)

                hookTypeRow("PreToolUse", desc: "Runs before Claude uses a tool (can block it)", color: Theme.blue)
                hookTypeRow("PostToolUse", desc: "Runs after a tool completes", color: Theme.green)
                hookTypeRow("Notification", desc: "Runs when Claude sends a notification", color: Theme.orange)
                hookTypeRow("Stop", desc: "Runs when Claude finishes a task", color: Theme.red)
            }
            .padding(14)
            .glassCard(radius: 10, border: Theme.blue.opacity(0.1))

            Spacer()
        }
        .padding()
    }

    private func hookTypeRow(_ name: String, desc: String, color: Color) -> some View {
        HStack(alignment: .top, spacing: 10) {
            Text(name)
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundStyle(color)
                .frame(width: 110, alignment: .leading)
            Text(desc)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private func eventColor(_ event: String) -> Color {
        switch event {
        case "PreToolUse": return Theme.blue
        case "PostToolUse": return Theme.green
        case "Notification": return Theme.orange
        case "Stop": return Theme.red
        case "SubagentStop": return Theme.purple
        default: return .secondary
        }
    }

    private func openSettingsInFinder() {
        let url = ConfigManager.globalSettingsURL
        if FileManager.default.fileExists(atPath: url.path) {
            NSWorkspace.shared.activateFileViewerSelecting([url])
        } else {
            let folder = url.deletingLastPathComponent()
            if FileManager.default.fileExists(atPath: folder.path) {
                NSWorkspace.shared.open(folder)
            }
        }
    }
}

// MARK: - Backups View

struct BackupsView: View {
    @ObservedObject var config: ConfigManager
    @State private var confirmRestore: BackupFile?
    @State private var selectedBackup: BackupFile?
    @State private var previewContent = ""
    @State private var diffSummary: (added: [String], removed: [String])?

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 8) {
                        Text("Backups")
                            .font(.title3.bold())
                        Text("\(config.backupFiles.count)")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(Theme.cyan)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 2)
                            .background(Theme.cyan.opacity(0.12), in: Capsule())
                    }
                    Text("A snapshot is saved automatically before every config change.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button(action: { config.loadBackups() }) {
                    Image(systemName: "arrow.clockwise")
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)

                Button(action: openBackupFolder) {
                    Image(systemName: "folder")
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
            }
            .padding(16)

            Rectangle().fill(Theme.subtleBorder).frame(height: 1)

            if config.backupFiles.isEmpty {
                VStack(spacing: 16) {
                    Spacer()
                    ZStack {
                        Circle()
                            .fill(Theme.cyan.opacity(0.08))
                            .frame(width: 70, height: 70)
                            .blur(radius: 10)
                        Image(systemName: "clock.arrow.circlepath")
                            .font(.system(size: 34))
                            .foregroundStyle(Theme.cyan.opacity(0.5))
                    }
                    Text("No Backups Yet")
                        .font(.title3.bold())
                    Text("Backups are created automatically each time you\nadd, remove, or move a server. Up to 30 are kept.")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                    Spacer()
                }
            } else {
                HStack(spacing: 0) {
                    // Backup list
                    ScrollView {
                        LazyVStack(spacing: 4) {
                            ForEach(config.backupFiles) { backup in
                                backupRow(backup)
                            }
                        }
                        .padding(10)
                    }
                    .frame(minWidth: 280, maxWidth: 340)

                    Rectangle().fill(Theme.subtleBorder).frame(width: 1)

                    // Preview pane
                    VStack(spacing: 0) {
                        if let selected = selectedBackup {
                            HStack {
                                GlowDot(color: Theme.cyan, size: 6)
                                Text(selected.formattedDate)
                                    .font(.caption.weight(.semibold))
                                Spacer()
                                Text(selected.formattedSize)
                                    .font(.caption2)
                                    .foregroundStyle(.tertiary)
                            }
                            .padding(.horizontal, 12)
                            .padding(.vertical, 8)
                            .background(Color.black.opacity(0.15))

                            // Diff summary
                            if let diff = diffSummary, (!diff.added.isEmpty || !diff.removed.isEmpty) {
                                VStack(alignment: .leading, spacing: 4) {
                                    if !diff.removed.isEmpty {
                                        ForEach(diff.removed, id: \.self) { name in
                                            HStack(spacing: 6) {
                                                Image(systemName: "minus.circle.fill")
                                                    .font(.system(size: 10))
                                                    .foregroundStyle(Theme.red)
                                                Text(name)
                                                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                                                    .foregroundStyle(Theme.red)
                                            }
                                        }
                                    }
                                    if !diff.added.isEmpty {
                                        ForEach(diff.added, id: \.self) { name in
                                            HStack(spacing: 6) {
                                                Image(systemName: "plus.circle.fill")
                                                    .font(.system(size: 10))
                                                    .foregroundStyle(Theme.green)
                                                Text(name)
                                                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                                                    .foregroundStyle(Theme.green)
                                            }
                                        }
                                    }
                                }
                                .padding(.horizontal, 12)
                                .padding(.vertical, 8)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .background(Color.black.opacity(0.1))
                            }

                            ScrollView {
                                Text(previewContent)
                                    .font(.system(size: 11, design: .monospaced))
                                    .textSelection(.enabled)
                                    .foregroundStyle(.secondary)
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .padding(10)
                            }
                        } else {
                            VStack {
                                Spacer()
                                Image(systemName: "doc.text.magnifyingglass")
                                    .font(.title2)
                                    .foregroundStyle(.quaternary)
                                Text("Click a backup to preview")
                                    .font(.callout)
                                    .foregroundStyle(.quaternary)
                                Spacer()
                            }
                            .frame(maxWidth: .infinity)
                        }
                    }
                }
            }

            Rectangle().fill(Theme.subtleBorder).frame(height: 1)

            HStack {
                Image(systemName: "info.circle")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                Text("~/Library/Application Support/Configonaut/backups/")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                Spacer()
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
            .background(Color.black.opacity(0.1))
        }
        .alert(
            "Restore Backup?",
            isPresented: Binding(
                get: { confirmRestore != nil },
                set: { if !$0 { confirmRestore = nil } }
            )
        ) {
            Button("Cancel", role: .cancel) { confirmRestore = nil }
            Button("Restore") {
                if let backup = confirmRestore {
                    _ = config.restoreBackup(backup)
                    config.loadBackups()
                }
                confirmRestore = nil
            }
        } message: {
            Text("Replace your current config with the backup from \(confirmRestore?.formattedDate ?? "")? Your current config will be backed up first.")
        }
    }

    private func backupRow(_ backup: BackupFile) -> some View {
        HStack(spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text(backup.formattedDate)
                    .font(.system(size: 12, weight: .medium))
                Text(backup.formattedSize)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }

            Spacer()

            Button("Restore") { confirmRestore = backup }
                .buttonStyle(.plain)
                .font(.caption.weight(.medium))
                .foregroundStyle(.white)
                .padding(.horizontal, 10)
                .padding(.vertical, 4)
                .background(Theme.cyan.opacity(0.2), in: RoundedRectangle(cornerRadius: 5))
                .overlay(
                    RoundedRectangle(cornerRadius: 5)
                        .strokeBorder(Theme.cyan.opacity(0.3), lineWidth: 0.5)
                )

            Button {
                config.deleteBackup(backup)
                if selectedBackup?.id == backup.id { selectedBackup = nil; diffSummary = nil }
            } label: {
                Image(systemName: "trash")
                    .font(.caption)
                    .foregroundStyle(Theme.red.opacity(0.5))
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
        .background(
            RoundedRectangle(cornerRadius: 6)
                .fill(selectedBackup?.id == backup.id ? Theme.cyan.opacity(0.08) : Theme.cardFill)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .strokeBorder(
                    selectedBackup?.id == backup.id ? Theme.cyan.opacity(0.2) : Theme.cardBorder,
                    lineWidth: 0.5
                )
        )
        .contentShape(Rectangle())
        .onTapGesture {
            selectBackup(backup)
        }
    }

    private func selectBackup(_ backup: BackupFile) {
        if selectedBackup?.id == backup.id {
            selectedBackup = nil
            diffSummary = nil
            return
        }

        selectedBackup = backup
        if let data = try? Data(contentsOf: backup.url),
           let text = String(data: data, encoding: .utf8) {
            previewContent = text
            // Compute diff against current config
            computeDiff(backupData: data, backup: backup)
        } else {
            previewContent = "(Unable to read)"
            diffSummary = nil
        }
    }

    private func computeDiff(backupData: Data, backup: BackupFile) {
        // Parse backup servers
        guard let backupObj = try? JSONSerialization.jsonObject(with: backupData) as? [String: Any],
              let backupServers = backupObj["mcpServers"] as? [String: Any]
        else {
            diffSummary = nil
            return
        }

        // Find the next-newer backup (or current config) to compare against
        let sortedBackups = config.backupFiles // already sorted newest-first
        let idx = sortedBackups.firstIndex(where: { $0.id == backup.id })

        var newerServers: Set<String>
        if let idx = idx, idx > 0 {
            // Compare against the backup that came after this one
            let newerBackup = sortedBackups[idx - 1]
            if let newerData = try? Data(contentsOf: newerBackup.url),
               let newerObj = try? JSONSerialization.jsonObject(with: newerData) as? [String: Any],
               let servers = newerObj["mcpServers"] as? [String: Any] {
                newerServers = Set(servers.keys)
            } else {
                diffSummary = nil
                return
            }
        } else {
            // This is the newest backup -- compare against current config
            let configURL = ConfigManager.configURL
            if let currentData = try? Data(contentsOf: configURL),
               let currentObj = try? JSONSerialization.jsonObject(with: currentData) as? [String: Any],
               let servers = currentObj["mcpServers"] as? [String: Any] {
                newerServers = Set(servers.keys)
            } else {
                diffSummary = nil
                return
            }
        }

        let backupSet = Set(backupServers.keys)

        // What was added AFTER this backup (in newer = present in newer, absent in this backup)
        let added = newerServers.subtracting(backupSet).sorted()
        // What was removed AFTER this backup (present in this backup, absent in newer)
        let removed = backupSet.subtracting(newerServers).sorted()

        diffSummary = (added: added, removed: removed)
    }

    private func openBackupFolder() {
        let url = ConfigManager.backupDir
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        NSWorkspace.shared.open(url)
    }
}
