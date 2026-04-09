import SwiftUI
import AppKit

struct AgentsView: View {
    @ObservedObject var config: ConfigManager
    @State private var searchText = ""
    @State private var selectedAgent: AgentEntry?
    @State private var editingContent = ""
    @State private var bottomPanelHeight: CGFloat = 260
    @State private var newAgentName = ""
    @State private var newAgentContent = ""
    @State private var showNewAgent = false
    @State private var confirmDelete: AgentEntry?

    private var personalAgents: [AgentEntry] {
        filteredAgents.filter { $0.source == .personal }.sorted { $0.name < $1.name }
    }

    private var pluginGroups: [(plugin: String, agents: [AgentEntry], isEnabled: Bool)] {
        let pluginAgents = filteredAgents.filter { $0.source == .plugin }
        let grouped = Dictionary(grouping: pluginAgents) { $0.pluginName }
        return grouped.keys.sorted().map { plugin in
            let agents = grouped[plugin]!.sorted { $0.name < $1.name }
            let key = "\(plugin)@claude-plugins-official"
            let isEnabled = config.enabledPlugins[key] ?? false
            return (plugin: plugin, agents: agents, isEnabled: isEnabled)
        }
    }

    private var filteredAgents: [AgentEntry] {
        if searchText.isEmpty { return config.agents }
        let q = searchText.lowercased()
        return config.agents.filter {
            $0.name.lowercased().contains(q) ||
            $0.description.lowercased().contains(q) ||
            $0.pluginName.lowercased().contains(q)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 8) {
                        Text("Agents")
                            .font(.title3.bold())
                        Text("\(config.agents.count)")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(Theme.purple)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 2)
                            .background(Theme.purple.opacity(0.12), in: Capsule())
                    }
                    Text("Personal and plugin agents for Claude Code.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()

                Button {
                    withAnimation(.easeInOut(duration: 0.2)) {
                        selectedAgent = nil
                        showNewAgent = true
                        newAgentName = ""
                        newAgentContent = Self.agentTemplate("my-agent")
                    }
                } label: {
                    HStack(spacing: 5) {
                        Image(systemName: "plus")
                            .font(.system(size: 11, weight: .bold))
                        Text("New Agent")
                            .font(.callout.weight(.semibold))
                    }
                    .foregroundStyle(.white)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 7)
                    .background(
                        LinearGradient(colors: [Theme.blue, Theme.purple], startPoint: .leading, endPoint: .trailing),
                        in: RoundedRectangle(cornerRadius: 8)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .strokeBorder(Color.white.opacity(0.1), lineWidth: 0.5)
                    )
                }
                .buttonStyle(.plain)

                Button(action: { config.loadAgents() }) {
                    Image(systemName: "arrow.clockwise").foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
            }
            .padding(16)

            // Search bar
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.tertiary)
                    .font(.caption)
                TextField("Search agents...", text: $searchText)
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
                    if config.agents.isEmpty {
                        agentsEmptyState
                    } else if filteredAgents.isEmpty {
                        noResultsState
                    } else {
                        agentsList
                    }

                    // Bottom panel (editor or create)
                    if let agent = selectedAgent {
                        HDivider(
                            position: $bottomPanelHeight,
                            range: 150...500
                        )
                        .padding(.horizontal, 16)

                        editorPanel(agent)
                            .frame(height: bottomPanelHeight)
                            .padding(.horizontal, 16)
                            .padding(.bottom, 6)
                    } else if showNewAgent {
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

            HStack(spacing: 8) {
                GlowDot(color: config.statusIsError ? Theme.red : Theme.green, size: 6)
                Text(config.statusMessage)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Spacer()
                let personalCount = config.agents.filter { $0.source == .personal }.count
                Text("\(personalCount) personal, \(config.enabledPlugins.filter(\.value).count) plugins")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
            .background(Color.black.opacity(0.1))
        }
        .alert(
            "Delete Agent?",
            isPresented: Binding(
                get: { confirmDelete != nil },
                set: { if !$0 { confirmDelete = nil } }
            )
        ) {
            Button("Cancel", role: .cancel) { confirmDelete = nil }
            Button("Delete", role: .destructive) {
                if let agent = confirmDelete {
                    config.deleteAgent(agent)
                    if selectedAgent?.id == agent.id { selectedAgent = nil }
                }
                confirmDelete = nil
            }
        } message: {
            Text("Permanently delete \"\(confirmDelete?.name ?? "")\"? This can't be undone.")
        }
    }

    // MARK: - Create Panel

    private static func agentTemplate(_ name: String) -> String {
        """
        ---
        name: \(name)
        description: A custom agent
        tools: Read, Edit, Write, Bash, Glob, Grep
        model: sonnet
        color: blue
        ---

        You are a specialized agent. Describe your role and capabilities here.

        ## Instructions

        - What should this agent do?
        - What tools should it use and when?
        - What rules should it follow?
        """
    }

    private var createPanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Header
            HStack(spacing: 10) {
                Image(systemName: "plus.circle.fill")
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.green)
                    .shadow(color: Theme.green.opacity(0.4), radius: 4)

                Text("Create New Agent")
                    .font(.system(size: 15, weight: .semibold))

                Spacer()

                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        showNewAgent = false
                        newAgentName = ""
                        newAgentContent = ""
                    }
                } label: {
                    Image(systemName: "xmark")
                        .font(.caption2.weight(.bold))
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
            }

            // Name field
            HStack(spacing: 8) {
                Text("Name:")
                    .font(.caption.weight(.medium))
                    .foregroundStyle(.secondary)
                TextField("e.g. my-reviewer", text: $newAgentName)
                    .textFieldStyle(.plain)
                    .font(.system(size: 13, design: .monospaced))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 5)
                    .background(Color.black.opacity(0.2), in: RoundedRectangle(cornerRadius: 6))
                    .onChange(of: newAgentName) { _, name in
                        let safe = name.lowercased()
                            .replacingOccurrences(of: " ", with: "-")
                            .filter { $0.isLetter || $0.isNumber || $0 == "-" }
                        if !safe.isEmpty {
                            newAgentContent = Self.agentTemplate(safe)
                        }
                    }
            }

            // Editable template
            TextEditor(text: $newAgentContent)
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
                    let name = newAgentName.trimmingCharacters(in: .whitespaces)
                    if let agent = config.createAgent(name: name) {
                        // Overwrite with user-edited content
                        try? newAgentContent.write(to: agent.filePath, atomically: true, encoding: .utf8)
                        config.loadAgents()
                        selectedAgent = config.agents.first { $0.filePath == agent.filePath }
                        if let sel = selectedAgent,
                           let content = try? String(contentsOf: sel.filePath, encoding: .utf8) {
                            editingContent = content
                        }
                        showNewAgent = false
                        newAgentName = ""
                        newAgentContent = ""
                    }
                } label: {
                    Label("Create", systemImage: "plus.circle.fill")
                        .font(.caption.weight(.semibold))
                }
                .buttonStyle(GradientButtonStyle(
                    colors: [Color(hex: "22C55E"), Theme.green],
                    glowColor: Theme.green,
                    isEnabled: !newAgentName.trimmingCharacters(in: .whitespaces).isEmpty
                ))
                .disabled(newAgentName.trimmingCharacters(in: .whitespaces).isEmpty)

                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        showNewAgent = false
                        newAgentName = ""
                        newAgentContent = ""
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

    private func editorPanel(_ agent: AgentEntry) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            // Header
            HStack(spacing: 10) {
                Circle()
                    .fill(agentColor(agent.color))
                    .frame(width: 10, height: 10)
                    .shadow(color: agentColor(agent.color).opacity(0.5), radius: 4)

                Text(agent.name)
                    .font(.system(size: 15, weight: .semibold, design: .monospaced))

                if !agent.model.isEmpty {
                    Text(agent.model)
                        .font(.system(size: 9, weight: .medium, design: .monospaced))
                        .foregroundStyle(.secondary)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.white.opacity(0.06), in: Capsule())
                }

                Text(agent.source == .personal ? "Personal" : agent.pluginName)
                    .font(.caption.weight(.medium))
                    .foregroundStyle(agent.source == .personal ? Theme.blue : Theme.purple)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)
                    .background(
                        (agent.source == .personal ? Theme.blue : Theme.purple).opacity(0.1),
                        in: Capsule()
                    )

                Spacer()

                Button {
                    NSWorkspace.shared.activateFileViewerSelecting([agent.filePath])
                } label: {
                    Image(systemName: "folder")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
                .help("Show in Finder")

                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        selectedAgent = nil
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
                        try editingContent.write(to: agent.filePath, atomically: true, encoding: .utf8)
                        config.loadAgents()
                        config.setStatus("Saved \"\(agent.name)\".", isError: false)
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

                if agent.source == .plugin {
                    let pluginKey = "\(agent.pluginName)@claude-plugins-official"
                    let isPluginEnabled = config.enabledPlugins[pluginKey] ?? false
                    Button {
                        config.togglePlugin(pluginKey)
                    } label: {
                        Label(
                            isPluginEnabled ? "Disable Plugin" : "Enable Plugin",
                            systemImage: isPluginEnabled ? "moon.fill" : "bolt.fill"
                        )
                        .font(.caption.weight(.medium))
                        .foregroundStyle(isPluginEnabled ? Theme.red : Theme.green)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(
                            (isPluginEnabled ? Theme.red : Theme.green).opacity(0.1),
                            in: RoundedRectangle(cornerRadius: 6)
                        )
                        .overlay(
                            RoundedRectangle(cornerRadius: 6)
                                .strokeBorder(
                                    (isPluginEnabled ? Theme.red : Theme.green).opacity(0.2),
                                    lineWidth: 0.5
                                )
                        )
                    }
                    .buttonStyle(.plain)
                }

                Spacer()

                // File path
                Text(agent.filePath.path.replacingOccurrences(of: NSHomeDirectory(), with: "~"))
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
                    .truncationMode(.middle)

                if agent.source == .personal {
                    Button {
                        confirmDelete = agent
                    } label: {
                        Label("Delete", systemImage: "trash")
                            .font(.caption.weight(.medium))
                            .foregroundStyle(Theme.red.opacity(0.7))
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background(Theme.red.opacity(0.06), in: RoundedRectangle(cornerRadius: 6))
                            .overlay(
                                RoundedRectangle(cornerRadius: 6)
                                    .strokeBorder(Theme.red.opacity(0.12), lineWidth: 0.5)
                            )
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .padding(14)
        .glassCard(radius: 12)
    }

    // MARK: - Agents List

    private var agentsList: some View {
        ScrollView {
            LazyVStack(spacing: 14) {
                // Personal agents section
                if !personalAgents.isEmpty {
                    personalSection
                }

                // Plugin agent groups
                ForEach(pluginGroups, id: \.plugin) { group in
                    pluginSection(group)
                }
            }
            .padding(14)
        }
    }

    private var personalSection: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Section header
            HStack(spacing: 10) {
                GlowDot(color: Theme.blue, size: 8)
                Image(systemName: "person.fill")
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.blue)
                Text("Personal")
                    .font(.callout.weight(.semibold))
                Text("\(personalAgents.count)")
                    .font(.caption2.weight(.medium))
                    .foregroundStyle(Theme.blue)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Theme.blue.opacity(0.1), in: Capsule())
                Spacer()

                Button(action: openPersonalFolder) {
                    Image(systemName: "folder")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
                .help("Open agents folder")
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(Theme.blue.opacity(0.04), in: RoundedRectangle(cornerRadius: 10))
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(Theme.blue.opacity(0.1), lineWidth: 0.5)
            )

            // Thread line + cards
            HStack(alignment: .top, spacing: 0) {
                VStack(spacing: 0) {
                    Rectangle()
                        .fill(Theme.blue.opacity(0.2))
                        .frame(width: 1.5)
                }
                .frame(width: 20)
                .padding(.leading, 8)

                VStack(spacing: 6) {
                    ForEach(personalAgents) { agent in
                        agentCard(agent, isPluginEnabled: true)
                    }
                }
                .padding(.top, 6)
            }
        }
    }

    private func openPersonalFolder() {
        let fm = FileManager.default
        let url = ConfigManager.personalAgentsDir
        try? fm.createDirectory(at: url, withIntermediateDirectories: true)
        NSWorkspace.shared.open(url)
    }

    private func pluginSection(_ group: (plugin: String, agents: [AgentEntry], isEnabled: Bool)) -> some View {
        let color = group.isEnabled ? Theme.purple : Color.gray

        return VStack(alignment: .leading, spacing: 0) {
            // Plugin header
            HStack(spacing: 10) {
                GlowDot(color: group.isEnabled ? Theme.green : Theme.red, size: 8)
                Text(group.plugin)
                    .font(.callout.weight(.semibold))
                Text(group.isEnabled ? "enabled" : "disabled")
                    .font(.caption2.weight(.medium))
                    .foregroundStyle(group.isEnabled ? Theme.green : Theme.red)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(
                        (group.isEnabled ? Theme.green : Theme.red).opacity(0.1),
                        in: Capsule()
                    )
                Spacer()

                Button {
                    let key = "\(group.plugin)@claude-plugins-official"
                    config.togglePlugin(key)
                } label: {
                    Text(group.isEnabled ? "Disable" : "Enable")
                        .font(.caption.weight(.medium))
                        .foregroundStyle(group.isEnabled ? Theme.red : Theme.green)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 4)
                        .background(
                            (group.isEnabled ? Theme.red : Theme.green).opacity(0.1),
                            in: RoundedRectangle(cornerRadius: 6)
                        )
                        .overlay(
                            RoundedRectangle(cornerRadius: 6)
                                .strokeBorder(
                                    (group.isEnabled ? Theme.red : Theme.green).opacity(0.2),
                                    lineWidth: 0.5
                                )
                        )
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(Theme.purple.opacity(0.04), in: RoundedRectangle(cornerRadius: 10))
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(Theme.purple.opacity(0.1), lineWidth: 0.5)
            )

            // Agent cards with thread line
            HStack(alignment: .top, spacing: 0) {
                // Thread line
                VStack(spacing: 0) {
                    Rectangle()
                        .fill(color.opacity(0.2))
                        .frame(width: 1.5)
                }
                .frame(width: 20)
                .padding(.leading, 8)

                VStack(spacing: 6) {
                    ForEach(group.agents) { agent in
                        agentCard(agent, isPluginEnabled: group.isEnabled)
                    }
                }
                .padding(.top, 6)
            }
        }
    }

    private func agentCard(_ agent: AgentEntry, isPluginEnabled: Bool) -> some View {
        HStack(spacing: 8) {
            Circle()
                .fill(agentColor(agent.color))
                .frame(width: 8, height: 8)
                .shadow(color: agentColor(agent.color).opacity(0.5), radius: 3)

            Text(agent.name)
                .font(.system(size: 13, weight: .medium, design: .monospaced))
                .lineLimit(1)

            if !agent.model.isEmpty {
                Text(agent.model)
                    .font(.system(size: 9, weight: .medium, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Color.white.opacity(0.06), in: Capsule())
            }

            if !agent.description.isEmpty {
                Text(agent.description)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }

            Spacer()

            if !isPluginEnabled {
                Text("plugin off")
                    .font(.caption2)
                    .foregroundStyle(Theme.red.opacity(0.6))
            }

            Image(systemName: selectedAgent?.id == agent.id ? "chevron.down" : "chevron.right")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
        .background(
            RoundedRectangle(cornerRadius: 8)
                .fill(selectedAgent?.id == agent.id ? agentColor(agent.color).opacity(0.08) : Theme.cardFill)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .strokeBorder(
                    selectedAgent?.id == agent.id ? agentColor(agent.color).opacity(0.2) : Theme.cardBorder,
                    lineWidth: 0.5
                )
        )
        .contentShape(Rectangle())
        .onTapGesture {
            withAnimation(.easeInOut(duration: 0.15)) {
                if selectedAgent?.id == agent.id {
                    selectedAgent = nil
                } else {
                    selectedAgent = agent
                    if let content = try? String(contentsOf: agent.filePath, encoding: .utf8) {
                        editingContent = content
                    } else {
                        editingContent = "(Unable to read file)"
                    }
                }
            }
        }
        .opacity(isPluginEnabled ? 1 : 0.5)
    }

    // MARK: - Empty States

    private var agentsEmptyState: some View {
        VStack(spacing: 16) {
            Spacer()
            ZStack {
                Circle()
                    .fill(Theme.purple.opacity(0.08))
                    .frame(width: 70, height: 70)
                    .blur(radius: 10)
                Image(systemName: "person.3.fill")
                    .font(.system(size: 34))
                    .foregroundStyle(Theme.purple.opacity(0.5))
            }
            Text("No Agents Found")
                .font(.title3.bold())
            Text("Agents come from plugins installed via Claude Code.\nEnable plugins in your Claude Code settings to see their agents here.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 380)
            Spacer()
        }
    }

    private var noResultsState: some View {
        VStack(spacing: 12) {
            Spacer()
            Image(systemName: "magnifyingglass")
                .font(.title2)
                .foregroundStyle(.quaternary)
            Text("No agents match \"\(searchText)\"")
                .font(.callout)
                .foregroundStyle(.secondary)
            Spacer()
        }
        .frame(maxWidth: .infinity)
    }

    private func agentColor(_ color: String) -> Color {
        switch color.lowercased() {
        case "red": return Theme.red
        case "green": return Theme.green
        case "blue": return Theme.blue
        case "purple": return Theme.purple
        case "orange": return Theme.orange
        case "cyan": return Theme.cyan
        default: return Theme.purple
        }
    }
}
