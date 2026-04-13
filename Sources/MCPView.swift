import SwiftUI
import AppKit
import UniformTypeIdentifiers

// MARK: - Drag Divider (reusable)

struct HDivider: View {
    @Binding var position: CGFloat
    let range: ClosedRange<CGFloat>

    @State private var isDragging = false
    @State private var dragStart: CGFloat = 0

    var body: some View {
        Rectangle()
            .fill(isDragging ? Theme.blue.opacity(0.4) : Theme.subtleBorder)
            .frame(height: 6)
            .overlay(
                Capsule()
                    .fill(isDragging ? Theme.blue : Color.white.opacity(0.15))
                    .frame(width: 36, height: 3)
            )
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0, coordinateSpace: .global)
                    .onChanged { value in
                        if !isDragging {
                            isDragging = true
                            dragStart = position
                        }
                        // Drag up (negative translation) = grow panel, drag down = shrink
                        let newPos = dragStart - value.translation.height
                        position = max(range.lowerBound, min(range.upperBound, newPos))
                    }
                    .onEnded { _ in isDragging = false }
            )
            .onHover { hovering in
                if hovering { NSCursor.resizeUpDown.push() }
                else { NSCursor.pop() }
            }
    }
}

struct VDivider: View {
    @Binding var ratio: CGFloat
    let totalWidth: CGFloat

    @State private var isDragging = false

    var body: some View {
        Rectangle()
            .fill(isDragging ? Theme.blue.opacity(0.4) : Theme.subtleBorder)
            .frame(width: 6)
            .overlay(
                Capsule()
                    .fill(isDragging ? Theme.blue : Color.white.opacity(0.15))
                    .frame(width: 3, height: 36)
            )
            .contentShape(Rectangle())
            .gesture(
                DragGesture()
                    .onChanged { value in
                        isDragging = true
                        let newRatio = ratio + value.translation.width / totalWidth
                        ratio = max(0.25, min(0.75, newRatio))
                    }
                    .onEnded { _ in isDragging = false }
            )
            .onHover { hovering in
                if hovering { NSCursor.resizeLeftRight.push() }
                else { NSCursor.pop() }
            }
    }
}

// MARK: - MCP Servers View

enum AddServerTab: String, CaseIterable {
    case marketplace = "Marketplace"
    case pasteJSON = "Add manually"
}

struct MCPView: View {
    @ObservedObject var config: ConfigManager
    @StateObject private var catalogStore = CatalogStore()
    @State private var selectedServer: (name: String, source: ServerSource)?
    @State private var showAddPanel = false
    @State private var addTab: AddServerTab = .marketplace
    @State private var inputText = ""
    @State private var serverName = ""
    @State private var needsName = false
    @State private var parseError = ""
    @State private var pendingEntries: [(String, [String: Any])] = []
    @State private var activeDropTargeted = false
    @State private var inactiveDropTargeted = false
    @State private var confirmDelete: (name: String, source: ServerSource)?
    @State private var showHelp = false

    // Editing (always active when a server is selected)
    @State private var editingJSON = ""

    // Resizable bottom panel — tall enough to comfortably show an expanded
    // marketplace row with its full JSON editor without forcing the user to
    // drag the divider up on first open.
    @State private var bottomPanelHeight: CGFloat = 520

    private var canAdd: Bool {
        !pendingEntries.isEmpty && (!needsName || !serverName.trimmingCharacters(in: .whitespaces).isEmpty)
    }

    private var selectedEntry: ServerEntry? {
        guard let sel = selectedServer else { return nil }
        switch sel.source {
        case .active: return config.activeServers.first { $0.name == sel.name }
        case .stored: return config.storedServers.first { $0.name == sel.name }
        }
    }

    private var hasBottomPanel: Bool {
        selectedServer != nil || showAddPanel
    }

    var body: some View {
        VStack(spacing: 0) {
            if config.needsRestart {
                restartBanner
            }

            // Header
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 8) {
                        Text("MCP Servers")
                            .font(.title3.bold())
                        Text("\(config.activeServers.count + config.storedServers.count)")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(Theme.green)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 2)
                            .background(Theme.green.opacity(0.12), in: Capsule())
                    }
                    Text(config.configURL.path.replacingOccurrences(of: NSHomeDirectory(), with: "~"))
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(.tertiary)
                }
                Spacer()

                Button {
                    withAnimation(.easeInOut(duration: 0.2)) {
                        showAddPanel = true
                        selectedServer = nil
                    }
                } label: {
                    HStack(spacing: 5) {
                        Image(systemName: "plus")
                            .font(.system(size: 11, weight: .bold))
                        Text("Add Server")
                            .font(.callout.weight(.semibold))
                    }
                    .foregroundStyle(.white)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 7)
                    .background(
                        LinearGradient(colors: [Color(hex: "22C55E"), Theme.green], startPoint: .leading, endPoint: .trailing),
                        in: RoundedRectangle(cornerRadius: 8)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .strokeBorder(Color.white.opacity(0.1), lineWidth: 0.5)
                    )
                }
                .buttonStyle(.plain)

                Button(action: { config.reloadAll() }) {
                    Image(systemName: "arrow.clockwise").foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)

                Button(action: openConfigInFinder) {
                    Image(systemName: "folder").foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
            }
            .padding(16)

            Rectangle().fill(Theme.subtleBorder).frame(height: 1)

            GeometryReader { geo in
                VStack(spacing: 0) {
                    // Two-column server lists (50/50)
                    HStack(spacing: 12) {
                        activeColumn
                        inactiveColumn
                    }
                    .padding(.horizontal, 16)
                    .padding(.top, 10)
                    .padding(.bottom, hasBottomPanel ? 0 : 10)

                    // Bottom panel area (detail or add)
                    if let sel = selectedServer, let server = selectedEntry {
                        HDivider(
                            position: $bottomPanelHeight,
                            range: 150...500
                        )
                        .padding(.horizontal, 16)

                        detailPanel(server, isActive: sel.source == .active)
                            .frame(height: bottomPanelHeight)
                            .padding(.horizontal, 16)
                            .padding(.bottom, 6)
                    } else if showAddPanel {
                        HDivider(
                            position: $bottomPanelHeight,
                            range: 150...500
                        )
                        .padding(.horizontal, 16)

                        addPanel
                            .frame(height: bottomPanelHeight)
                            .padding(.horizontal, 16)
                            .padding(.bottom, 6)
                    }
                }
            }

            Rectangle().fill(Theme.subtleBorder).frame(height: 1)

            // Footer
            HStack(spacing: 8) {
                GlowDot(color: config.statusIsError ? Theme.red : Theme.green, size: 6)
                Text(config.statusMessage)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Spacer()
                Text("\(config.activeServers.count) active, \(config.storedServers.count) inactive")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
            .background(Color.black.opacity(0.1))
        }
        .alert(
            "Delete Server?",
            isPresented: Binding(
                get: { confirmDelete != nil },
                set: { if !$0 { confirmDelete = nil } }
            )
        ) {
            Button("Cancel", role: .cancel) { confirmDelete = nil }
            Button("Delete", role: .destructive) {
                if let d = confirmDelete {
                    config.deleteServer(d.name, from: d.source)
                    if selectedServer?.name == d.name { selectedServer = nil }
                }
                confirmDelete = nil
            }
        } message: {
            Text("Permanently delete \"\(confirmDelete?.name ?? "")\"? This can't be undone.")
        }
    }

    // MARK: - Restart Banner

    private var restartBanner: some View {
        HStack(spacing: 10) {
            Image(systemName: "arrow.trianglehead.2.clockwise.rotate.90")
                .font(.system(size: 14, weight: .bold))
                .foregroundStyle(.white)
            Text("Changes saved! Quit and reopen \(config.mode == .desktop ? "Claude Desktop" : "Claude Code") to apply.")
                .font(.callout.weight(.semibold))
                .foregroundStyle(.white)
            Spacer()
            Button("Got it") {
                withAnimation { config.needsRestart = false }
            }
            .buttonStyle(.plain)
            .font(.caption.weight(.semibold))
            .foregroundStyle(.white)
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
            .background(.white.opacity(0.2), in: RoundedRectangle(cornerRadius: 6))
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .background(
            LinearGradient(
                colors: [Theme.blue, Theme.purple.opacity(0.8)],
                startPoint: .leading,
                endPoint: .trailing
            )
        )
    }

    // MARK: - Active Column (Green)

    private var activeColumn: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                GlassSectionHeader(
                    title: "Active", count: config.activeServers.count,
                    icon: "bolt.fill", color: Theme.green
                )
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)

            Text("Running in Claude Desktop right now")
                .font(.caption)
                .foregroundStyle(Theme.green.opacity(0.6))
                .padding(.horizontal, 14)
                .padding(.bottom, 8)

            Rectangle().fill(Theme.subtleBorder).frame(height: 1)

            ScrollView {
                if config.activeServers.isEmpty {
                    emptyColumn(
                        icon: "bolt.slash",
                        title: "No active servers",
                        hint: "Click + Add Server below,\nor drag one from Inactive.",
                        color: Theme.green
                    )
                } else {
                    LazyVStack(spacing: 4) {
                        ForEach(config.activeServers) { server in
                            ServerRow(
                                server: server,
                                accentColor: Theme.green,
                                isSelected: selectedServer?.name == server.name && selectedServer?.source == .active,
                                onSelect: { selectServer(server.name, source: .active) }
                            )
                            .onDrag {
                                NSItemProvider(object: "active:\(server.name)" as NSString)
                            }
                        }
                    }
                    .padding(8)
                }
            }
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(Theme.green, lineWidth: 2)
                    .shadow(color: Theme.green.opacity(0.4), radius: 8)
                    .opacity(activeDropTargeted ? 1 : 0)
                    .animation(.easeInOut(duration: 0.2), value: activeDropTargeted)
            )
            .onDrop(of: [UTType.plainText], isTargeted: $activeDropTargeted) { providers in
                handleDrop(providers, target: .active)
            }
        }
        .glassCard(radius: 14, border: Theme.green.opacity(0.15), fill: Theme.green.opacity(0.03))
    }

    // MARK: - Inactive Column (Red)

    private var inactiveColumn: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                GlassSectionHeader(
                    title: "Inactive", count: config.storedServers.count,
                    icon: "moon.fill", color: Theme.red
                )
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)

            Text("Saved for later -- not running")
                .font(.caption)
                .foregroundStyle(Theme.red.opacity(0.5))
                .padding(.horizontal, 14)
                .padding(.bottom, 8)

            Rectangle().fill(Theme.subtleBorder).frame(height: 1)

            ScrollView {
                if config.storedServers.isEmpty {
                    emptyColumn(
                        icon: "moon.zzz",
                        title: "Nothing saved",
                        hint: "Drag servers here to turn them\noff without losing the config.",
                        color: Theme.red
                    )
                } else {
                    LazyVStack(spacing: 4) {
                        ForEach(config.storedServers) { server in
                            ServerRow(
                                server: server,
                                accentColor: Theme.red,
                                isSelected: selectedServer?.name == server.name && selectedServer?.source == .stored,
                                onSelect: { selectServer(server.name, source: .stored) }
                            )
                            .onDrag {
                                NSItemProvider(object: "stored:\(server.name)" as NSString)
                            }
                        }
                    }
                    .padding(8)
                }
            }
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(Theme.red, lineWidth: 2)
                    .shadow(color: Theme.red.opacity(0.4), radius: 8)
                    .opacity(inactiveDropTargeted ? 1 : 0)
                    .animation(.easeInOut(duration: 0.2), value: inactiveDropTargeted)
            )
            .onDrop(of: [UTType.plainText], isTargeted: $inactiveDropTargeted) { providers in
                handleDrop(providers, target: .stored)
            }
        }
        .glassCard(radius: 14, border: Theme.red.opacity(0.12), fill: Theme.red.opacity(0.02))
    }

    // MARK: - Detail Panel (editable)

    /// Look up the catalog entry this server was installed from (if any).
    private func linkedCatalogServer(_ name: String) -> CatalogServer? {
        guard let catalogId = config.catalogId(forServer: name) else { return nil }
        return catalogStore.server(withId: catalogId)
    }

    private func detailPanel(_ server: ServerEntry, isActive: Bool) -> some View {
        // Missing required secrets (only populated for catalog-linked servers).
        let missing = config.missingSecrets(forServer: server.name, catalog: catalogStore.catalog)
        let isReady = missing.isEmpty
        let catalogEntry = linkedCatalogServer(server.name)

        return VStack(alignment: .leading, spacing: 8) {
            // Header
            HStack(spacing: 10) {
                GlowDot(color: isActive ? Theme.green : Theme.red, size: 10)
                Text(server.name)
                    .font(.system(size: 15, weight: .semibold, design: .monospaced))
                Text(isActive ? "Active" : "Inactive")
                    .font(.caption.weight(.medium))
                    .foregroundStyle(isActive ? Theme.green : Theme.red)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)
                    .background(
                        (isActive ? Theme.green : Theme.red).opacity(0.1),
                        in: Capsule()
                    )

                if !isReady {
                    Label("Check \(missing.joined(separator: ", "))", systemImage: "exclamationmark.triangle.fill")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(Theme.amber)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background(Theme.amber.opacity(0.12), in: Capsule())
                        .help("These values may still be placeholders — double-check before turning on. We can't fully validate syntax or credentials.")
                }

                Spacer()

                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        selectedServer = nil
                    }
                } label: {
                    Image(systemName: "xmark")
                        .font(.caption2.weight(.bold))
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
            }

            // Catalog help strip — only for servers installed via Marketplace.
            if let entry = catalogEntry {
                catalogHelpStrip(entry: entry)
            }

            // Always-editable config
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
                // Save
                Button {
                    if let sel = selectedServer {
                        _ = config.updateServerConfig(sel.name, source: sel.source, newJSON: editingJSON)
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
                    withAnimation(.easeInOut(duration: 0.2)) {
                        if isActive {
                            config.moveToStored(server.name)
                            selectedServer = nil
                        } else {
                            // Warn if secrets look like placeholders, but never block.
                            config.moveToActiveWithWarning(server.name, catalog: catalogStore.catalog)
                            selectedServer = nil
                        }
                    }
                } label: {
                    Label(
                        isActive ? "Turn Off" : "Turn On",
                        systemImage: isActive ? "moon.fill" : "bolt.fill"
                    )
                    .font(.caption.weight(.medium))
                    .foregroundStyle(isActive ? Theme.red : Theme.green)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(
                        (isActive ? Theme.red : Theme.green).opacity(0.1),
                        in: RoundedRectangle(cornerRadius: 6)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 6)
                            .strokeBorder(
                                (canFlip ? (isActive ? Theme.red : Theme.green) : .gray).opacity(0.2),
                                lineWidth: 0.5
                            )
                    )
                }
                .buttonStyle(.plain)

                Button { copyToClipboard(server) } label: {
                    Label("Copy JSON", systemImage: "doc.on.doc")
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

                Button {
                    confirmDelete = (server.name, isActive ? .active : .stored)
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
        .padding(14)
        .glassCard(radius: 12)
    }

    /// A compact horizontal strip that surfaces everything the user needs to
    /// finish setting up a marketplace-installed server: repo/homepage links,
    /// setup notes, and a per-env-var button that opens its token docs page.
    @ViewBuilder
    private func catalogHelpStrip(entry: CatalogServer) -> some View {
        let requiredWithHelp = entry.requiredEnvVars.filter { $0.helpUrl != nil }

        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                Image(systemName: "info.circle.fill")
                    .font(.system(size: 10))
                    .foregroundStyle(Theme.blue.opacity(0.8))
                Text("From Marketplace · \(entry.publisher.name)")
                    .font(.system(size: 10, weight: .medium))
                    .foregroundStyle(.secondary)

                Spacer(minLength: 6)

                // Repo / homepage chips
                if let repo = entry.repository, let url = URL(string: repo) {
                    helpChip(
                        label: "Repo",
                        icon: "chevron.left.forwardslash.chevron.right",
                        url: url,
                        tooltip: "Open the server's GitHub repository"
                    )
                }
                if let home = entry.homepage,
                   home != entry.repository, // dedupe when both point to the same place
                   let url = URL(string: home) {
                    helpChip(
                        label: "Docs",
                        icon: "book",
                        url: url,
                        tooltip: "Open the server's homepage / docs"
                    )
                }
            }

            // Setup notes (if provided in the catalog)
            if let notes = entry.setupNotes, !notes.isEmpty {
                Text(notes)
                    .font(.system(size: 10.5))
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            // Per-envVar "Get token" chips — one button per required secret
            // that has an associated helpUrl.
            if !requiredWithHelp.isEmpty {
                HStack(spacing: 6) {
                    Image(systemName: "key.fill")
                        .font(.system(size: 9))
                        .foregroundStyle(Theme.amber.opacity(0.8))
                    Text("Get credentials:")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(.tertiary)

                    ForEach(requiredWithHelp) { env in
                        if let helpURL = env.helpUrl.flatMap(URL.init(string:)) {
                            Button {
                                NSWorkspace.shared.open(helpURL)
                            } label: {
                                Text(env.name)
                                    .font(.system(size: 9.5, design: .monospaced))
                                    .foregroundStyle(Theme.amber)
                                    .padding(.horizontal, 7)
                                    .padding(.vertical, 2)
                                    .background(Theme.amber.opacity(0.10), in: Capsule())
                                    .overlay(
                                        Capsule().strokeBorder(Theme.amber.opacity(0.25), lineWidth: 0.5)
                                    )
                            }
                            .buttonStyle(.plain)
                            .help("Open instructions for getting \(env.name)")
                        }
                    }

                    Spacer(minLength: 0)
                }
            }
        }
        .padding(10)
        .background(Theme.blue.opacity(0.06), in: RoundedRectangle(cornerRadius: 8))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .strokeBorder(Theme.blue.opacity(0.15), lineWidth: 0.5)
        )
    }

    private func helpChip(label: String, icon: String, url: URL, tooltip: String) -> some View {
        Button {
            NSWorkspace.shared.open(url)
        } label: {
            Label(label, systemImage: icon)
                .font(.system(size: 9.5, weight: .medium))
                .foregroundStyle(Theme.blue.opacity(0.9))
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(Theme.blue.opacity(0.10), in: Capsule())
                .overlay(
                    Capsule().strokeBorder(Theme.blue.opacity(0.25), lineWidth: 0.5)
                )
        }
        .buttonStyle(.plain)
        .help(tooltip)
    }

    // MARK: - Add Panel

    private var addPanel: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Image(systemName: "plus.circle.fill")
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.green)
                Text("Add a New Server")
                    .font(.headline)

                // Tab picker — Marketplace vs Paste JSON
                HStack(spacing: 4) {
                    ForEach(AddServerTab.allCases, id: \.self) { tab in
                        Button {
                            withAnimation(.easeInOut(duration: 0.12)) { addTab = tab }
                        } label: {
                            Text(tab.rawValue)
                                .font(.caption.weight(addTab == tab ? .semibold : .medium))
                                .foregroundStyle(addTab == tab ? .primary : .secondary)
                                .padding(.horizontal, 10)
                                .padding(.vertical, 4)
                                .background(
                                    RoundedRectangle(cornerRadius: 6)
                                        .fill(addTab == tab ? Theme.blue.opacity(0.15) : .clear)
                                )
                                .overlay(
                                    RoundedRectangle(cornerRadius: 6)
                                        .strokeBorder(
                                            addTab == tab ? Theme.blue.opacity(0.3) : .clear,
                                            lineWidth: 0.5
                                        )
                                )
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.leading, 8)

                Spacer()
                Button {
                    withAnimation(.easeInOut(duration: 0.2)) { showHelp.toggle() }
                } label: {
                    Label("What's an MCP server?", systemImage: "questionmark.circle")
                        .font(.caption)
                        .foregroundStyle(Theme.blue.opacity(0.7))
                }
                .buttonStyle(.plain)

                Button {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        clearInput()
                        showAddPanel = false
                    }
                } label: {
                    Image(systemName: "xmark")
                        .font(.caption2.weight(.bold))
                        .foregroundStyle(.tertiary)
                }
                .buttonStyle(.plain)
            }

            if showHelp {
                helpCard
            }

            // Tab content
            Group {
                switch addTab {
                case .marketplace:
                    MarketplaceView(
                        config: config,
                        catalogStore: catalogStore,
                        onDidAdd: { _ in
                            // Leave panel open so the user can install multiple.
                        }
                    )
                case .pasteJSON:
                    pasteJsonPanel
                }
            }
        }
        .padding(14)
        .glassCard(radius: 12, border: Theme.green.opacity(0.15))
    }

    // MARK: - Paste JSON sub-panel

    private var pasteJsonPanel: some View {
        VStack(alignment: .leading, spacing: 10) {
            ZStack(alignment: .topLeading) {
                TextEditor(text: $inputText)
                    .font(.system(size: 12, design: .monospaced))
                    .scrollContentBackground(.hidden)
                    .padding(8)

                if inputText.isEmpty {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Step 1:  Copy the JSON block from the MCP provider's docs")
                        Text("Step 2:  Paste it right here  (Cmd + V)")
                        Text("Step 3:  Click  Turn On  to add it to Claude")
                        Text("")
                        Text("Supports any format:")
                        Text("  { \"name\": { \"command\": \"...\", \"args\": [...] } }")
                        Text("  { \"command\": \"...\", \"args\": [...], \"env\": {...} }")
                        Text("  Full config with \"mcpServers\" wrapper")
                    }
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.white.opacity(0.2))
                    .padding(14)
                    .allowsHitTesting(false)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color.black.opacity(0.35))
            .clipShape(RoundedRectangle(cornerRadius: 10))
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(borderColor, lineWidth: 1)
            )
            .onChange(of: inputText) { _, newValue in validate(newValue) }

            if needsName {
                HStack(spacing: 10) {
                    Image(systemName: "tag.fill")
                        .font(.system(size: 11))
                        .foregroundStyle(Theme.green.opacity(0.6))
                    Text("Name:")
                        .font(.caption.weight(.medium))
                        .foregroundStyle(.secondary)
                    TextField("e.g. my-server", text: $serverName)
                        .textFieldStyle(.plain)
                        .font(.system(size: 13, design: .monospaced))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 5)
                        .background(Color.black.opacity(0.2), in: RoundedRectangle(cornerRadius: 6))
                        .frame(maxWidth: 260)
                }
            }

            HStack(spacing: 10) {
                if !inputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    validationPill
                }

                Spacer()

                Button("Cancel") {
                    withAnimation(.easeInOut(duration: 0.15)) {
                        clearInput()
                        showAddPanel = false
                    }
                }
                .buttonStyle(.plain)
                .font(.caption.weight(.medium))
                .foregroundStyle(Theme.red.opacity(0.7))

                Button("Clear") { clearInput() }
                    .buttonStyle(.plain)
                    .font(.caption)
                    .foregroundStyle(.secondary)

                Button { add(to: .stored) } label: {
                    Label("Save Only", systemImage: "moon.fill")
                        .font(.caption.weight(.medium))
                        .foregroundStyle(canAdd ? Theme.red.opacity(0.9) : .gray.opacity(0.4))
                        .padding(.horizontal, 12)
                        .padding(.vertical, 7)
                }
                .buttonStyle(.plain)
                .background(
                    RoundedRectangle(cornerRadius: 8)
                        .fill(Color.white.opacity(0.05))
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .strokeBorder(
                                    canAdd ? Theme.red.opacity(0.25) : Color.white.opacity(0.05),
                                    lineWidth: 1
                                )
                        )
                )
                .disabled(!canAdd)

                Button { add(to: .active) } label: {
                    Label("Turn On", systemImage: "bolt.fill")
                        .frame(width: 90)
                }
                .buttonStyle(GradientButtonStyle(
                    colors: [Color(hex: "22C55E"), Theme.green],
                    glowColor: Theme.green,
                    isEnabled: canAdd
                ))
                .disabled(!canAdd)
                .keyboardShortcut(.return, modifiers: .command)
            }
        }
    }

    // MARK: - Help Card

    private var helpCard: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("MCP (Model Context Protocol) servers give Claude extra powers -- like reading files, searching the web, or connecting to apps like GitHub, Figma, or databases.")
                .font(.caption)
            Text("Providers give you a JSON snippet to paste here. This tool handles all the formatting and puts it in the right place so you never have to edit JSON by hand.")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Theme.blue.opacity(0.06), in: RoundedRectangle(cornerRadius: 8))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .strokeBorder(Theme.blue.opacity(0.12), lineWidth: 0.5)
        )
    }

    // MARK: - Helpers

    private func emptyColumn(icon: String, title: String, hint: String, color: Color) -> some View {
        VStack(spacing: 10) {
            Spacer().frame(height: 20)
            ZStack {
                Circle()
                    .fill(color.opacity(0.06))
                    .frame(width: 50, height: 50)
                    .blur(radius: 8)
                Image(systemName: icon)
                    .font(.title2)
                    .foregroundStyle(color.opacity(0.35))
            }
            Text(title)
                .font(.callout.weight(.semibold))
                .foregroundStyle(.secondary)
            Text(hint)
                .font(.caption)
                .foregroundStyle(.tertiary)
                .multilineTextAlignment(.center)
            Spacer().frame(height: 20)
        }
        .frame(maxWidth: .infinity)
    }

    private var borderColor: Color {
        let trimmed = inputText.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return Theme.cardBorder }
        return parseError.isEmpty ? Theme.green.opacity(0.5) : Theme.red.opacity(0.5)
    }

    private var validationPill: some View {
        Group {
            if parseError.isEmpty {
                let n = pendingEntries.count
                Label(n == 1 ? "Ready to add" : "\(n) servers found",
                      systemImage: "checkmark.circle.fill")
                    .foregroundStyle(Theme.green)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 5)
                    .background(Theme.green.opacity(0.1), in: Capsule())
            } else {
                Label(parseError, systemImage: "xmark.circle.fill")
                    .foregroundStyle(Theme.red)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 5)
                    .background(Theme.red.opacity(0.1), in: Capsule())
            }
        }
        .font(.caption2.weight(.medium))
        .lineLimit(2)
    }

    // MARK: - Actions

    private func selectServer(_ name: String, source: ServerSource) {
        withAnimation(.easeInOut(duration: 0.15)) {
            if selectedServer?.name == name && selectedServer?.source == source {
                selectedServer = nil
            } else {
                selectedServer = (name, source)
                showAddPanel = false
                // Load config into editor immediately
                let entry: ServerEntry? = source == .active
                    ? config.activeServers.first { $0.name == name }
                    : config.storedServers.first { $0.name == name }
                editingJSON = entry?.configJSON ?? ""
            }
        }
    }

    private func validate(_ text: String) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            parseError = ""; pendingEntries = []; needsName = false; return
        }
        switch config.parseInput(text) {
        case .servers(let entries):
            pendingEntries = entries; needsName = false; parseError = ""
        case .needsName(let cfg):
            pendingEntries = [("", cfg)]; needsName = true; parseError = ""
        case .error(let msg):
            pendingEntries = []; needsName = false; parseError = msg
        }
    }

    private func add(to target: ServerSource) {
        var entries = pendingEntries
        if needsName {
            entries = entries.map { (serverName.trimmingCharacters(in: .whitespaces), $0.1) }
        }
        switch target {
        case .active: config.addToActive(entries)
        case .stored: config.addToStored(entries)
        }
        clearInput()
        withAnimation(.easeInOut(duration: 0.2)) { showAddPanel = false }
    }

    private func clearInput() {
        inputText = ""; serverName = ""; needsName = false
        pendingEntries = []; parseError = ""
    }

    private func handleDrop(_ providers: [NSItemProvider], target: ServerSource) -> Bool {
        guard let provider = providers.first else { return false }
        provider.loadObject(ofClass: NSString.self) { string, _ in
            guard let payload = string as? String else { return }
            let parts = payload.split(separator: ":", maxSplits: 1)
            guard parts.count == 2 else { return }
            let source = String(parts[0])
            let name = String(parts[1])
            DispatchQueue.main.async {
                switch (source, target) {
                case ("active", .stored): config.moveToStored(name)
                case ("stored", .active): config.moveToActive(name)
                default: break
                }
                selectedServer = nil
            }
        }
        return true
    }

    private func copyToClipboard(_ server: ServerEntry) {
        let indented = server.configJSON
            .split(separator: "\n", omittingEmptySubsequences: false)
            .joined(separator: "\n    ")
        let snippet = "    \"\(server.name)\": \(indented)"
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(snippet, forType: .string)
        config.setStatus("Copied \"\(server.name)\" to clipboard.", isError: false)
    }

    private func openConfigInFinder() {
        let url = config.configURL
        if FileManager.default.fileExists(atPath: url.path) {
            NSWorkspace.shared.activateFileViewerSelecting([url])
        } else {
            NSWorkspace.shared.open(url.deletingLastPathComponent())
        }
    }
}

// MARK: - Server Row

struct ServerRow: View {
    let server: ServerEntry
    let accentColor: Color
    let isSelected: Bool
    let onSelect: () -> Void

    @State private var isHovered = false

    var body: some View {
        HStack(spacing: 8) {
            GlowDot(color: accentColor, size: 7)

            Text(server.name)
                .font(.system(size: 13, weight: .medium, design: .monospaced))
                .lineLimit(1)

            Spacer()

            Image(systemName: isSelected ? "chevron.down" : "chevron.right")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
        .background(
            RoundedRectangle(cornerRadius: 8)
                .fill(
                    isSelected ? accentColor.opacity(0.08) :
                    isHovered ? Theme.cardHover : Theme.cardFill
                )
        )
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .strokeBorder(
                    isSelected ? accentColor.opacity(0.2) :
                    isHovered ? Theme.cardBorderHover : Theme.cardBorder,
                    lineWidth: 0.5
                )
        )
        .contentShape(Rectangle())
        .onHover { hovering in
            withAnimation(.easeInOut(duration: 0.12)) { isHovered = hovering }
        }
        .onTapGesture(perform: onSelect)
    }
}
