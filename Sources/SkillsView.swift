import SwiftUI
import AppKit

struct SkillsView: View {
    @ObservedObject var config: ConfigManager
    @State private var searchText = ""
    @State private var selectedSkill: SkillEntry?
    @State private var editingContent = ""
    @State private var bottomPanelHeight: CGFloat = 260
    @State private var showNewSkill = false
    @State private var newSkillName = ""
    @State private var newSkillContent = ""
    @State private var newSkillType: SkillSource = .command

    private var sourceGroups: [(source: SkillSource, skills: [SkillEntry])] {
        let grouped = Dictionary(grouping: filteredSkills) { $0.source }
        let order: [SkillSource] = [.command, .skill, .plugin]
        return order.compactMap { source in
            guard let skills = grouped[source], !skills.isEmpty else { return nil }
            return (source: source, skills: skills)
        }
    }

    private var filteredSkills: [SkillEntry] {
        if searchText.isEmpty { return config.skills }
        let q = searchText.lowercased()
        return config.skills.filter {
            $0.name.lowercased().contains(q) ||
            $0.description.lowercased().contains(q) ||
            $0.source.rawValue.lowercased().contains(q)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 8) {
                        Text("Skills")
                            .font(.title3.bold())
                        Text("\(config.skills.count)")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(Theme.amber)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 2)
                            .background(Theme.amber.opacity(0.12), in: Capsule())
                    }
                    Text("Custom commands and skills for Claude Code.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()

                Button {
                    withAnimation(.easeInOut(duration: 0.2)) {
                        selectedSkill = nil
                        showNewSkill = true
                        newSkillName = ""
                        newSkillContent = Self.skillTemplate("my-command", source: .command)
                    }
                } label: {
                    HStack(spacing: 5) {
                        Image(systemName: "plus")
                            .font(.system(size: 11, weight: .bold))
                        Text("New Skill")
                            .font(.callout.weight(.semibold))
                    }
                    .foregroundStyle(.white)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 7)
                    .background(
                        LinearGradient(colors: [Theme.amber, Theme.orange], startPoint: .leading, endPoint: .trailing),
                        in: RoundedRectangle(cornerRadius: 8)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .strokeBorder(Color.white.opacity(0.1), lineWidth: 0.5)
                    )
                }
                .buttonStyle(.plain)

                Button(action: { config.loadSkills() }) {
                    Image(systemName: "arrow.clockwise").foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)

                Button(action: openSkillsFolder) {
                    Image(systemName: "folder").foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
            }
            .padding(16)

            // Search bar
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.tertiary)
                    .font(.caption)
                TextField("Search skills...", text: $searchText)
                    .textFieldStyle(.plain)
                    .font(.callout)
                if !searchText.isEmpty {
                    Button(action: { searchText = "" }) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.tertiary)
                            .font(.caption)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .background(Color.black.opacity(0.2), in: RoundedRectangle(cornerRadius: 8))
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .strokeBorder(Theme.cardBorder, lineWidth: 0.5)
            )
            .padding(.horizontal, 16)
            .padding(.bottom, 10)

            Rectangle().fill(Theme.subtleBorder).frame(height: 1)

            // Main content area
            GeometryReader { _ in
                VStack(spacing: 0) {
                    if config.skills.isEmpty {
                        skillsEmptyState
                    } else if filteredSkills.isEmpty {
                        noResultsState
                    } else {
                        skillsList
                    }

                    // Bottom panel (editor or create)
                    if let skill = selectedSkill {
                        HDivider(
                            position: $bottomPanelHeight,
                            range: 150...500
                        )
                        .padding(.horizontal, 16)

                        editorPanel(skill)
                            .frame(height: bottomPanelHeight)
                            .padding(.horizontal, 16)
                            .padding(.bottom, 6)
                    } else if showNewSkill {
                        HDivider(
                            position: $bottomPanelHeight,
                            range: 150...500
                        )
                        .padding(.horizontal, 16)

                        createPanel
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
                Text("Commands in ~/.claude/commands/ | Skills in ~/.claude/skills/")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                Spacer()
                let enabled = config.skills.filter(\.isEnabled).count
                let disabled = config.skills.count - enabled
                Text("\(enabled) on, \(disabled) off")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
            .background(Color.black.opacity(0.1))
        }
    }

    // MARK: - Create Panel

    private static func skillTemplate(_ name: String, source: SkillSource) -> String {
        if source == .command {
            return """
            ---
            name: \(name)
            description: A custom slash command
            ---

            You are executing the /\(name) command.

            ## Instructions

            Describe what this command should do when invoked.
            """
        } else {
            return """
            ---
            name: \(name)
            description: A custom skill
            ---

            You are a specialized skill.

            ## Instructions

            Describe what this skill does and when it should activate.
            """
        }
    }

    private var createPanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Header
            HStack(spacing: 10) {
                Image(systemName: "plus.circle.fill")
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.green)
                    .shadow(color: Theme.green.opacity(0.4), radius: 4)

                Text("Create New Skill")
                    .font(.system(size: 15, weight: .semibold))

                Spacer()

                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        showNewSkill = false
                        newSkillName = ""
                        newSkillContent = ""
                    }
                } label: {
                    Image(systemName: "xmark")
                        .font(.caption2.weight(.bold))
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
            }

            // Name + type row
            HStack(spacing: 10) {
                Picker("", selection: $newSkillType) {
                    Text("Command").tag(SkillSource.command)
                    Text("Skill").tag(SkillSource.skill)
                }
                .pickerStyle(.segmented)
                .frame(width: 160)
                .onChange(of: newSkillType) { _, type in
                    let safe = newSkillName.lowercased()
                        .replacingOccurrences(of: " ", with: "-")
                        .filter { $0.isLetter || $0.isNumber || $0 == "-" }
                    newSkillContent = Self.skillTemplate(safe.isEmpty ? "my-command" : safe, source: type)
                }

                Text("Name:")
                    .font(.caption.weight(.medium))
                    .foregroundStyle(.secondary)
                TextField("e.g. my-command", text: $newSkillName)
                    .textFieldStyle(.plain)
                    .font(.system(size: 13, design: .monospaced))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 5)
                    .background(Color.black.opacity(0.2), in: RoundedRectangle(cornerRadius: 6))
                    .onChange(of: newSkillName) { _, name in
                        let safe = name.lowercased()
                            .replacingOccurrences(of: " ", with: "-")
                            .filter { $0.isLetter || $0.isNumber || $0 == "-" }
                        if !safe.isEmpty {
                            newSkillContent = Self.skillTemplate(safe, source: newSkillType)
                        }
                    }
            }

            // Editable template
            TextEditor(text: $newSkillContent)
                .font(.system(size: 11, design: .monospaced))
                .scrollContentBackground(.hidden)
                .padding(8)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.black.opacity(0.35))
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .strokeBorder(Theme.green.opacity(0.3), lineWidth: 1)
                )

            // Action buttons
            HStack(spacing: 8) {
                Button {
                    let name = newSkillName.trimmingCharacters(in: .whitespaces)
                    if let skill = config.createSkill(name: name, source: newSkillType) {
                        // Overwrite with user-edited content
                        try? newSkillContent.write(to: skill.filePath, atomically: true, encoding: .utf8)
                        config.loadSkills()
                        selectedSkill = config.skills.first { $0.filePath == skill.filePath }
                        if let sel = selectedSkill,
                           let content = try? String(contentsOf: sel.filePath, encoding: .utf8) {
                            editingContent = content
                        }
                        showNewSkill = false
                        newSkillName = ""
                        newSkillContent = ""
                    }
                } label: {
                    Label("Create", systemImage: "plus.circle.fill")
                        .font(.caption.weight(.semibold))
                }
                .buttonStyle(GradientButtonStyle(
                    colors: [Color(hex: "22C55E"), Theme.green],
                    glowColor: Theme.green,
                    isEnabled: !newSkillName.trimmingCharacters(in: .whitespaces).isEmpty
                ))
                .disabled(newSkillName.trimmingCharacters(in: .whitespaces).isEmpty)

                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        showNewSkill = false
                        newSkillName = ""
                        newSkillContent = ""
                    }
                } label: {
                    Label("Cancel", systemImage: "xmark")
                        .font(.caption.weight(.medium))
                        .foregroundStyle(.secondary)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(Color.white.opacity(0.05), in: RoundedRectangle(cornerRadius: 6))
                        .overlay(
                            RoundedRectangle(cornerRadius: 6)
                                .strokeBorder(Theme.cardBorder, lineWidth: 0.5)
                        )
                }
                .buttonStyle(.plain)

                Spacer()
            }
        }
        .padding(14)
        .glassCard(radius: 12, border: Theme.green.opacity(0.15))
    }

    // MARK: - Editor Panel

    private func editorPanel(_ skill: SkillEntry) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            // Header
            HStack(spacing: 10) {
                Circle()
                    .fill(skill.isEnabled ? sourceColor(skill.source) : .gray)
                    .frame(width: 10, height: 10)
                    .shadow(color: (skill.isEnabled ? sourceColor(skill.source) : .gray).opacity(0.5), radius: 4)

                Text(skill.name)
                    .font(.system(size: 15, weight: .semibold, design: .monospaced))

                Text(skill.source.rawValue)
                    .font(.caption.weight(.medium))
                    .foregroundStyle(sourceColor(skill.source))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)
                    .background(sourceColor(skill.source).opacity(0.1), in: Capsule())

                if skill.source == .command {
                    Text("/\(skill.name)")
                        .font(.system(size: 11, weight: .medium, design: .monospaced))
                        .foregroundStyle(Theme.amber)
                }

                Spacer()

                Button {
                    NSWorkspace.shared.activateFileViewerSelecting([skill.filePath])
                } label: {
                    Image(systemName: "folder")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
                .help("Show in Finder")

                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        selectedSkill = nil
                    }
                } label: {
                    Image(systemName: "xmark")
                        .font(.caption2.weight(.bold))
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
            }

            // Editable content
            TextEditor(text: $editingContent)
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
                    do {
                        try editingContent.write(to: skill.filePath, atomically: true, encoding: .utf8)
                        config.loadSkills()
                        config.setStatus("Saved \"\(skill.name)\".", isError: false)
                    } catch {
                        config.setStatus("Save error: \(error.localizedDescription)", isError: true)
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

                // On/Off toggle (not for plugins)
                if skill.source != .plugin {
                    Button {
                        config.toggleSkill(skill)
                        // Re-select the skill after toggle (path changes)
                        selectedSkill = nil
                    } label: {
                        Label(
                            skill.isEnabled ? "Disable" : "Enable",
                            systemImage: skill.isEnabled ? "moon.fill" : "bolt.fill"
                        )
                        .font(.caption.weight(.medium))
                        .foregroundStyle(skill.isEnabled ? Theme.red : Theme.green)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(
                            (skill.isEnabled ? Theme.red : Theme.green).opacity(0.1),
                            in: RoundedRectangle(cornerRadius: 6)
                        )
                        .overlay(
                            RoundedRectangle(cornerRadius: 6)
                                .strokeBorder(
                                    (skill.isEnabled ? Theme.red : Theme.green).opacity(0.2),
                                    lineWidth: 0.5
                                )
                        )
                    }
                    .buttonStyle(.plain)
                }

                Spacer()

                // File path
                Text(skill.filePath.path.replacingOccurrences(of: NSHomeDirectory(), with: "~"))
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
        .padding(14)
        .glassCard(radius: 12)
    }

    // MARK: - Skills List

    private var skillsList: some View {
        ScrollView {
            LazyVStack(spacing: 14) {
                ForEach(sourceGroups, id: \.source) { group in
                    sourceSection(group)
                }
            }
            .padding(14)
        }
    }

    private func sourceSection(_ group: (source: SkillSource, skills: [SkillEntry])) -> some View {
        let color = sourceColor(group.source)

        return VStack(alignment: .leading, spacing: 0) {
            // Source header
            HStack(spacing: 10) {
                GlowDot(color: color, size: 8)
                Image(systemName: sourceIcon(group.source))
                    .font(.system(size: 13))
                    .foregroundStyle(color)
                Text(group.source.rawValue)
                    .font(.callout.weight(.semibold))
                Text("\(group.skills.count)")
                    .font(.caption2.weight(.medium))
                    .foregroundStyle(color)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(color.opacity(0.1), in: Capsule())
                Spacer()
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(color.opacity(0.04), in: RoundedRectangle(cornerRadius: 10))
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(color.opacity(0.1), lineWidth: 0.5)
            )

            // Skill cards with thread line
            HStack(alignment: .top, spacing: 0) {
                VStack(spacing: 0) {
                    Rectangle()
                        .fill(color.opacity(0.2))
                        .frame(width: 1.5)
                }
                .frame(width: 20)
                .padding(.leading, 8)

                VStack(spacing: 6) {
                    ForEach(group.skills) { skill in
                        skillCard(skill)
                    }
                }
                .padding(.top, 6)
            }
        }
    }

    private func skillCard(_ skill: SkillEntry) -> some View {
        HStack(spacing: 8) {
            Circle()
                .fill(skill.isEnabled ? sourceColor(skill.source) : .gray)
                .frame(width: 8, height: 8)
                .shadow(color: (skill.isEnabled ? sourceColor(skill.source) : .gray).opacity(0.5), radius: 3)

            Text(skill.name)
                .font(.system(size: 13, weight: .medium, design: .monospaced))
                .lineLimit(1)

            if !skill.description.isEmpty {
                Text(skill.description)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }

            Spacer()

            if skill.source != .plugin {
                Text(skill.isEnabled ? "ON" : "OFF")
                    .font(.system(size: 9, weight: .bold, design: .rounded))
                    .foregroundStyle(skill.isEnabled ? Theme.green : Theme.red)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(
                        (skill.isEnabled ? Theme.green : Theme.red).opacity(0.12),
                        in: RoundedRectangle(cornerRadius: 4)
                    )
            } else if !skill.isEnabled {
                Text("plugin off")
                    .font(.caption2)
                    .foregroundStyle(Theme.red.opacity(0.6))
            }

            Image(systemName: selectedSkill?.id == skill.id ? "chevron.down" : "chevron.right")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
        .background(
            RoundedRectangle(cornerRadius: 8)
                .fill(selectedSkill?.id == skill.id ? sourceColor(skill.source).opacity(0.08) : Theme.cardFill)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .strokeBorder(
                    selectedSkill?.id == skill.id ? sourceColor(skill.source).opacity(0.2) : Theme.cardBorder,
                    lineWidth: 0.5
                )
        )
        .contentShape(Rectangle())
        .onTapGesture {
            withAnimation(.easeInOut(duration: 0.15)) {
                if selectedSkill?.id == skill.id {
                    selectedSkill = nil
                } else {
                    selectedSkill = skill
                    if let content = try? String(contentsOf: skill.filePath, encoding: .utf8) {
                        editingContent = content
                    } else {
                        editingContent = "(Unable to read file)"
                    }
                }
            }
        }
        .opacity(skill.isEnabled ? 1 : 0.5)
    }

    // MARK: - Empty States

    private var skillsEmptyState: some View {
        VStack(spacing: 16) {
            Spacer()
            ZStack {
                Circle()
                    .fill(Theme.amber.opacity(0.08))
                    .frame(width: 70, height: 70)
                    .blur(radius: 10)
                Image(systemName: "star.fill")
                    .font(.system(size: 34))
                    .foregroundStyle(Theme.amber.opacity(0.5))
            }
            Text("No Skills Found")
                .font(.title3.bold())
            Text("Skills are custom commands and extensions for Claude Code.\nCreate .md files in ~/.claude/commands/ or ~/.claude/skills/.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 380)

            VStack(alignment: .leading, spacing: 10) {
                Text("SKILL SOURCES")
                    .font(.caption2.weight(.bold))
                    .foregroundStyle(.tertiary)
                    .tracking(0.8)

                skillTypeRow("Custom Commands", desc: "Slash commands in ~/.claude/commands/", icon: "terminal", color: Theme.amber)
                skillTypeRow("Custom Skills", desc: "Skill definitions in ~/.claude/skills/", icon: "star", color: Theme.green)
                skillTypeRow("Plugin Skills", desc: "Skills from enabled Claude Code plugins", icon: "puzzlepiece.extension", color: Theme.purple)
            }
            .padding(14)
            .glassCard(radius: 10, border: Theme.amber.opacity(0.1))

            Spacer()
        }
        .frame(maxWidth: .infinity)
        .padding()
    }

    private func skillTypeRow(_ name: String, desc: String, icon: String, color: Color) -> some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: icon)
                .font(.system(size: 11))
                .foregroundStyle(color)
                .frame(width: 16)
            VStack(alignment: .leading, spacing: 1) {
                Text(name)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(color)
                Text(desc)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var noResultsState: some View {
        VStack(spacing: 12) {
            Spacer()
            Image(systemName: "magnifyingglass")
                .font(.title2)
                .foregroundStyle(.quaternary)
            Text("No skills match \"\(searchText)\"")
                .font(.callout)
                .foregroundStyle(.secondary)
            Spacer()
        }
        .frame(maxWidth: .infinity)
    }

    // MARK: - Helpers

    private func sourceColor(_ source: SkillSource) -> Color {
        switch source {
        case .command: return Theme.amber
        case .skill: return Theme.green
        case .plugin: return Theme.purple
        }
    }

    private func sourceIcon(_ source: SkillSource) -> String {
        switch source {
        case .command: return "terminal"
        case .skill: return "star.fill"
        case .plugin: return "puzzlepiece.extension.fill"
        }
    }

    private func openSkillsFolder() {
        let url = ConfigManager.commandsDir
        if FileManager.default.fileExists(atPath: url.path) {
            NSWorkspace.shared.open(url)
        } else {
            let claude = url.deletingLastPathComponent()
            if FileManager.default.fileExists(atPath: claude.path) {
                NSWorkspace.shared.open(claude)
            }
        }
    }
}
