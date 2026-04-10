import SwiftUI
import AppKit

/// The Marketplace tab inside the "Add Server" bottom panel.
///
/// Layout (inside the existing HDivider-resizable bottom panel):
///
///   ┌───────────────────────────────────────────────────────┐
///   │ [🔎 Search...]                                [Refresh]│
///   ├──────────────┬────────────────────────────────────────┤
///   │ Categories   │ Server list (filtered)                 │
///   │  All (64)    │ ● GitHub      vendor ✓          [Add]  │
///   │  Reference   │ ● Notion      vendor ✓          [Add]  │
///   │  Development │ ● Filesystem  official ✓        [Add]  │
///   │  ...         │ ...                                    │
///   └──────────────┴────────────────────────────────────────┘
///
/// Clicking [Add] calls `ConfigManager.addFromCatalog(...)` which installs the
/// server to the Inactive (Stored) column by default. Servers with required env
/// vars are still installed but the green "Turn On" button stays disabled until
/// the user fills in the tokens (enforced by `isReadyToEnable`).
struct MarketplaceView: View {
    @ObservedObject var config: ConfigManager
    @ObservedObject var catalogStore: CatalogStore
    var onDidAdd: (String) -> Void = { _ in }

    @State private var selectedServerId: String?
    @State private var hostWindow: NSWindow?
    @FocusState private var searchFieldFocused: Bool

    /// The live JSON snippet the user is currently editing in the expanded row.
    /// Reset to the server's default template on every new expansion — clicking
    /// off (deselecting) discards any pending edits so next open is fresh.
    @State private var editedJSON: String = ""
    @State private var editError: String?

    var body: some View {
        VStack(spacing: 10) {
            header

            if catalogStore.catalog == nil {
                emptyState
            } else {
                HStack(alignment: .top, spacing: 12) {
                    categorySidebar
                        .frame(width: 195)
                    serverList
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(WindowAccessor(window: $hostWindow))
        .onAppear {
            // Delay a tick so SwiftUI finishes laying out the TextField
            // before we try to focus it — avoids the "no focus target" no-op.
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
                activateAndFocusSearch()
            }
        }
    }

    /// Bring the Configonaut window to front AND make it the key window so
    /// keystrokes route to our TextField instead of whatever app is behind us.
    /// Clicking a non-key window normally does this automatically, but when
    /// our SwiftUI tap gesture eats the click before AppKit can promote the
    /// window we have to do it by hand.
    private func activateAndFocusSearch() {
        // 1. App-level activation
        if #available(macOS 14.0, *) {
            NSApplication.shared.activate()
        } else {
            NSApplication.shared.activate(ignoringOtherApps: true)
        }

        // 2. Window-level: make our specific window key
        let window = hostWindow
            ?? NSApplication.shared.keyWindow
            ?? NSApplication.shared.mainWindow
            ?? NSApplication.shared.windows.first(where: { $0.isVisible })
        window?.makeKeyAndOrderFront(nil)

        // 3. SwiftUI-level: give the TextField focus
        searchFieldFocused = true
    }

    // MARK: Header (search + refresh)

    private var header: some View {
        HStack(spacing: 10) {
            HStack(spacing: 6) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 11))
                    .foregroundStyle(searchFieldFocused ? Theme.blue : Color.white.opacity(0.35))
                TextField("Search servers, tags, publishers…", text: $catalogStore.searchText)
                    .textFieldStyle(.plain)
                    .font(.system(size: 12))
                    .focused($searchFieldFocused)

                if !catalogStore.searchText.isEmpty {
                    Button {
                        catalogStore.searchText = ""
                        searchFieldFocused = true
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .font(.system(size: 11))
                            .foregroundStyle(.tertiary)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(Color.black.opacity(0.25), in: RoundedRectangle(cornerRadius: 7))
            .overlay(
                RoundedRectangle(cornerRadius: 7)
                    .strokeBorder(
                        searchFieldFocused ? Theme.blue.opacity(0.5) : Theme.cardBorder,
                        lineWidth: 0.5
                    )
            )
            // Make the whole rounded rectangle clickable (not just the narrow
            // .plain TextField baseline) and route clicks into focus.
            .contentShape(RoundedRectangle(cornerRadius: 7))
            .onTapGesture {
                activateAndFocusSearch()
            }

            if catalogStore.isRefreshing {
                ProgressView()
                    .controlSize(.small)
                    .scaleEffect(0.7)
            }

            Button {
                catalogStore.refreshRemote()
            } label: {
                Image(systemName: "arrow.clockwise")
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
            }
            .buttonStyle(.plain)
            .help("Refresh from GitHub")

            if let count = catalogStore.catalog?.servers.count {
                Text("\(count)")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(Theme.blue)
                    .padding(.horizontal, 7)
                    .padding(.vertical, 2)
                    .background(Theme.blue.opacity(0.12), in: Capsule())
            }
        }
    }

    // MARK: Category sidebar

    private var categorySidebar: some View {
        ScrollView {
            VStack(spacing: 2) {
                categoryRow(
                    label: "All",
                    icon: "square.grid.2x2",
                    count: catalogStore.catalog?.servers.count ?? 0,
                    isSelected: catalogStore.selectedCategoryId == nil
                ) {
                    catalogStore.selectedCategoryId = nil
                }

                ForEach(catalogStore.categoriesWithCounts, id: \.category.id) { (cat, count) in
                    categoryRow(
                        label: cat.label,
                        icon: cat.icon ?? "circle",
                        count: count,
                        isSelected: catalogStore.selectedCategoryId == cat.id
                    ) {
                        catalogStore.selectedCategoryId = cat.id
                    }
                }
            }
            .padding(.vertical, 2)
        }
        .frame(maxHeight: .infinity)
        .background(Color.black.opacity(0.18), in: RoundedRectangle(cornerRadius: 9))
        .overlay(
            RoundedRectangle(cornerRadius: 9)
                .strokeBorder(Theme.cardBorder, lineWidth: 0.5)
        )
    }

    private func categoryRow(
        label: String,
        icon: String,
        count: Int,
        isSelected: Bool,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 7) {
                Image(systemName: icon)
                    .font(.system(size: 10))
                    .frame(width: 14)
                    .foregroundStyle(isSelected ? Theme.blue : .secondary)
                Text(label)
                    .font(.system(size: 11.5, weight: isSelected ? .semibold : .regular))
                    .foregroundStyle(isSelected ? .primary : .secondary)
                    .lineLimit(1)
                    .truncationMode(.tail)
                Spacer(minLength: 4)
                Text("\(count)")
                    .font(.system(size: 10))
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 9)
            .padding(.vertical, 5)
            .background(
                RoundedRectangle(cornerRadius: 6)
                    .fill(isSelected ? Theme.blue.opacity(0.10) : .clear)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .padding(.horizontal, 4)
    }

    // MARK: Server list

    private var serverList: some View {
        let servers = catalogStore.filteredServers
        return Group {
            if servers.isEmpty {
                VStack(spacing: 6) {
                    Image(systemName: "magnifyingglass")
                        .font(.title3)
                        .foregroundStyle(.tertiary)
                    Text("No servers match your filters")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ScrollView {
                    LazyVStack(spacing: 6) {
                        ForEach(servers) { server in
                            MarketplaceServerRow(
                                server: server,
                                isSelected: selectedServerId == server.id,
                                isInstalled: isInstalled(server),
                                editedJSON: $editedJSON,
                                editError: $editError,
                                onToggleSelect: { toggleSelection(server) },
                                onSave: { save(server) },
                                onReset: { resetEdit(for: server) }
                            )
                        }
                    }
                    .padding(.vertical, 2)
                    .padding(.horizontal, 2)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onChange(of: selectedServerId) { _, _ in
            // Clicking a different row (or collapsing) discards pending edits
            // and repopulates the editor from the new server's default template.
            editError = nil
            if let id = selectedServerId,
               let server = catalogStore.catalog?.servers.first(where: { $0.id == id }) {
                editedJSON = defaultSnippet(for: server)
            } else {
                editedJSON = ""
            }
        }
    }

    /// Toggle selection: clicking an already-selected row collapses it (and
    /// discards any unsaved JSON edits via the `.onChange` above).
    private func toggleSelection(_ server: CatalogServer) {
        if selectedServerId == server.id {
            selectedServerId = nil
        } else {
            selectedServerId = server.id
        }
    }

    /// Reset the editor for the currently expanded server back to its default
    /// template. Called from the "Reset" button in the expanded view.
    private func resetEdit(for server: CatalogServer) {
        editError = nil
        editedJSON = defaultSnippet(for: server)
    }

    // MARK: Empty state (no catalog loaded at all)

    private var emptyState: some View {
        VStack(spacing: 10) {
            Image(systemName: "shippingbox")
                .font(.title)
                .foregroundStyle(.tertiary)
            Text("Marketplace catalog not loaded")
                .font(.callout.weight(.semibold))
                .foregroundStyle(.secondary)
            if let err = catalogStore.lastError {
                Text(err)
                    .font(.caption)
                    .foregroundStyle(Theme.red.opacity(0.8))
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 20)
            }
            Button("Retry refresh") { catalogStore.refreshRemote() }
                .buttonStyle(.bordered)
                .controlSize(.small)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: Install flow

    private func isInstalled(_ server: CatalogServer) -> Bool {
        let allNames = config.activeServers.map(\.name) + config.storedServers.map(\.name)
        // Either same catalog id is linked, or a server literally named after it.
        if allNames.contains(server.id) { return true }
        let links = config.loadCatalogLinks()
        return links.values.contains(server.id)
    }

    /// Save the user's edited JSON snippet as a new server. Parses the editor
    /// text, validates it has a config block, installs via ConfigManager, and
    /// collapses the row on success.
    ///
    /// Accepts either form of JSON:
    ///
    ///   { "github": { "command": "...", "args": [...] } }    // keyed (recommended)
    ///   { "command": "...", "args": [...] }                   // bare config dict
    ///
    /// The keyed form is what the default snippet renders, so editing the key
    /// lets the user rename the server before saving.
    private func save(_ server: CatalogServer) {
        guard let parsed = parseEditedSnippet(editedJSON, fallbackName: server.id) else {
            editError = "JSON looks invalid. Check for trailing commas or missing quotes."
            return
        }
        let (name, dict) = parsed

        // Require at least one of command/url so we never save an empty shell.
        if dict["command"] == nil && dict["url"] == nil {
            editError = "Config must include either a \"command\" (stdio) or a \"url\" (http)."
            return
        }

        editError = nil
        let needsSetup = !(server.requiredEnvVars.isEmpty)
        let target: ServerSource = needsSetup ? .stored : .active
        let installedName = config.addFromCatalog(
            server,
            customConfig: dict,
            customName: name,
            target: target
        )
        // Collapse the row — this triggers .onChange, which clears editedJSON.
        selectedServerId = nil
        onDidAdd(installedName)
    }

    // MARK: Snippet helpers

    /// Pretty-printed keyed snippet for a catalog server:
    ///
    ///   {
    ///     "github" : {
    ///       "command" : "docker",
    ///       "args"    : [ ... ],
    ///       "env"     : { "GITHUB_PERSONAL_ACCESS_TOKEN": "" }
    ///     }
    ///   }
    ///
    /// This is the shape users see in Claude's mcp config JSON, so pasting
    /// from docs / editing inline both feel natural. Any required env vars
    /// declared by the catalog are pre-injected as empty-string slots (with
    /// a `<placeholder>` value when available) so the user has obvious hooks
    /// to fill in — even for servers whose catalog config doesn't already
    /// carry an `env` block (like the `uvx` / `npx` helpers).
    private func defaultSnippet(for server: CatalogServer) -> String {
        var configDict = server.config.toConfigDict()
        mergeEnvVarPlaceholders(&configDict, for: server)

        let wrapped: [String: Any] = [server.id: configDict]
        guard
            let data = try? JSONSerialization.data(
                withJSONObject: wrapped,
                options: [.prettyPrinted, .sortedKeys]
            ),
            let str = String(data: data, encoding: .utf8)
        else {
            return "{}"
        }
        return str
    }

    /// Mutate a config dict so that every env var the catalog declares has a
    /// corresponding slot the user can edit. For stdio transports we inject
    /// into `env`; for http transports we use the var's placeholder (if any)
    /// as the value of an equally-named header as a best-effort fallback —
    /// most HTTP catalog entries already inline their `Authorization` header
    /// so this rarely fires.
    private func mergeEnvVarPlaceholders(
        _ dict: inout [String: Any],
        for server: CatalogServer
    ) {
        let declared = server.envVars ?? []
        guard !declared.isEmpty else { return }

        // Stdio: attach to the "env" sub-dict.
        if dict["command"] != nil {
            var envBlock = (dict["env"] as? [String: String]) ?? [:]
            for v in declared where v.isRequired {
                if envBlock[v.name] == nil {
                    envBlock[v.name] = v.placeholder ?? ""
                }
            }
            // Also surface optional vars as empty slots so users discover
            // them without having to read the "Required" hint list.
            for v in declared where !v.isRequired {
                if envBlock[v.name] == nil {
                    envBlock[v.name] = v.placeholder ?? ""
                }
            }
            if !envBlock.isEmpty {
                dict["env"] = envBlock
            }
            return
        }

        // HTTP: leave headers alone unless the catalog author explicitly
        // wants a placeholder there — we can't guess which header maps to
        // which env var. The "Required" hint list above the editor still
        // tells the user what to configure.
    }

    /// Parse the editor's current text. Accepts both the keyed wrapper form
    /// (`{"name": {config}}`) and a raw config dict. Returns `(name, dict)`
    /// on success, `nil` if the JSON is invalid or doesn't look like an MCP
    /// server config.
    private func parseEditedSnippet(
        _ text: String,
        fallbackName: String
    ) -> (String, [String: Any])? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard
            let data = trimmed.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }

        // Keyed wrapper: exactly one top-level key whose value is a config dict.
        if obj.count == 1,
           let key = obj.keys.first,
           let inner = obj[key] as? [String: Any],
           (inner["command"] != nil || inner["url"] != nil) {
            let cleanName = key.trimmingCharacters(in: .whitespacesAndNewlines)
            return (cleanName.isEmpty ? fallbackName : cleanName, inner)
        }

        // Bare config dict (user stripped the outer key).
        if obj["command"] != nil || obj["url"] != nil {
            return (fallbackName, obj)
        }

        return nil
    }
}

// MARK: - Server Row

private struct MarketplaceServerRow: View {
    let server: CatalogServer
    let isSelected: Bool
    let isInstalled: Bool
    @Binding var editedJSON: String
    @Binding var editError: String?
    let onToggleSelect: () -> Void
    let onSave: () -> Void
    let onReset: () -> Void

    @State private var isHovered = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Header — always visible, click to toggle expand/collapse.
            HStack(alignment: .top, spacing: 10) {
                GlowDot(color: accentColor, size: 8)
                    .padding(.top, 4)

                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 6) {
                        Text(server.name)
                            .font(.system(size: 12.5, weight: .semibold))
                            .lineLimit(1)

                        if server.isVerified {
                            Image(systemName: "checkmark.seal.fill")
                                .font(.system(size: 9))
                                .foregroundStyle(Theme.blue.opacity(0.85))
                        }

                        Text(server.publisher.name)
                            .font(.system(size: 10))
                            .foregroundStyle(.tertiary)

                        if !server.requiredEnvVars.isEmpty {
                            Label("\(server.requiredEnvVars.count) key\(server.requiredEnvVars.count == 1 ? "" : "s")", systemImage: "key.fill")
                                .font(.system(size: 9))
                                .foregroundStyle(Theme.amber.opacity(0.8))
                                .labelStyle(.titleAndIcon)
                        }
                    }
                    Text(server.description)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                        .lineLimit(isSelected ? nil : 2)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 6)

                if isInstalled {
                    Label("Installed", systemImage: "checkmark.circle.fill")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(Theme.green)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Theme.green.opacity(0.1), in: Capsule())
                } else {
                    Image(systemName: isSelected ? "chevron.up" : "chevron.down")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(.tertiary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 5)
                }
            }
            .contentShape(Rectangle())
            .onTapGesture { onToggleSelect() }

            // Expanded detail + inline JSON editor.
            if isSelected {
                detailBlock
                    .padding(.top, 8)
            }
        }
        .padding(10)
        .background(
            RoundedRectangle(cornerRadius: 9)
                .fill(
                    isSelected ? Theme.blue.opacity(0.07) :
                    isHovered ? Theme.cardHover : Theme.cardFill
                )
        )
        .overlay(
            RoundedRectangle(cornerRadius: 9)
                .strokeBorder(
                    isSelected ? Theme.blue.opacity(0.3) :
                    isHovered ? Theme.cardBorderHover : Theme.cardBorder,
                    lineWidth: 0.5
                )
        )
        .onHover { hovering in
            withAnimation(.easeInOut(duration: 0.1)) { isHovered = hovering }
        }
    }

    private var accentColor: Color {
        switch server.publisher.type {
        case "official": return Theme.green
        case "vendor":   return Theme.blue
        default:         return Theme.purple
        }
    }

    private var detailBlock: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Setup notes — helpful context while editing.
            if let notes = server.setupNotes, !notes.isEmpty {
                Text(notes)
                    .font(.system(size: 10.5))
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            // Required env vars — reminder of what to fill in the JSON below.
            if !server.requiredEnvVars.isEmpty {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Required")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundStyle(.tertiary)
                    ForEach(server.requiredEnvVars) { env in
                        HStack(spacing: 6) {
                            Image(systemName: env.isSecret ? "key.fill" : "circle.fill")
                                .font(.system(size: 8))
                                .foregroundStyle(Theme.amber.opacity(0.7))
                            Text(env.name)
                                .font(.system(size: 10, design: .monospaced))
                                .foregroundStyle(.primary)
                            if let desc = env.description {
                                Text("— \(desc)")
                                    .font(.system(size: 10))
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                            }
                            if let help = env.helpUrl, let url = URL(string: help) {
                                Button {
                                    NSWorkspace.shared.open(url)
                                } label: {
                                    Image(systemName: "arrow.up.right.square")
                                        .font(.system(size: 9))
                                        .foregroundStyle(Theme.blue)
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }
                .padding(8)
                .background(Color.black.opacity(0.18), in: RoundedRectangle(cornerRadius: 6))
            }

            // Editor label + reset link.
            HStack(spacing: 6) {
                Text("Config JSON")
                    .font(.system(size: 9, weight: .bold))
                    .foregroundStyle(.tertiary)
                Text("— edit then save to install")
                    .font(.system(size: 9))
                    .foregroundStyle(.tertiary)
                Spacer()
                Button {
                    onReset()
                } label: {
                    Label("Reset", systemImage: "arrow.uturn.backward")
                        .font(.system(size: 9, weight: .medium))
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
                .help("Restore the default template")
            }

            // Inline JSON editor.
            TextEditor(text: $editedJSON)
                .font(.system(size: 11, design: .monospaced))
                .scrollContentBackground(.hidden)
                .padding(8)
                .frame(minHeight: 140)
                .background(Color.black.opacity(0.28), in: RoundedRectangle(cornerRadius: 6))
                .overlay(
                    RoundedRectangle(cornerRadius: 6)
                        .strokeBorder(
                            (editError != nil) ? Theme.red.opacity(0.5) : Theme.cardBorder,
                            lineWidth: 0.5
                        )
                )
                .onChange(of: editedJSON) { _, _ in
                    // Clear error as user edits — lets them try again without
                    // having to click Reset to dismiss the banner.
                    if editError != nil { editError = nil }
                }

            // Inline error banner for parse / validation failures.
            if let err = editError {
                HStack(spacing: 6) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.system(size: 10))
                        .foregroundStyle(Theme.red)
                    Text(err)
                        .font(.system(size: 10.5))
                        .foregroundStyle(Theme.red.opacity(0.9))
                        .fixedSize(horizontal: false, vertical: true)
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 5)
                .background(Theme.red.opacity(0.1), in: RoundedRectangle(cornerRadius: 6))
            }

            // Footer: link chips on the left, Save button on the right.
            HStack(spacing: 10) {
                if let home = server.homepage, let url = URL(string: home) {
                    linkChip("Homepage", icon: "house", url: url)
                }
                if let repo = server.repository, let url = URL(string: repo) {
                    linkChip("Repo", icon: "chevron.left.forwardslash.chevron.right", url: url)
                }
                if let lic = server.license {
                    Text(lic)
                        .font(.system(size: 9))
                        .foregroundStyle(.tertiary)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.white.opacity(0.04), in: Capsule())
                }

                Spacer()

                Button {
                    onSave()
                } label: {
                    Label("Save to list", systemImage: "square.and.arrow.down")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(.white)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(Theme.blue, in: RoundedRectangle(cornerRadius: 6))
                }
                .buttonStyle(.plain)
                .disabled(isInstalled)
                .opacity(isInstalled ? 0.4 : 1)
            }
        }
    }

    private func linkChip(_ label: String, icon: String, url: URL) -> some View {
        Button {
            NSWorkspace.shared.open(url)
        } label: {
            Label(label, systemImage: icon)
                .font(.system(size: 9, weight: .medium))
                .foregroundStyle(Theme.blue.opacity(0.85))
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Window Accessor

/// Tiny NSViewRepresentable that captures the NSWindow hosting the SwiftUI
/// hierarchy so we can call makeKeyAndOrderFront(_:) on the exact window
/// containing our TextField (instead of guessing via NSApplication.windows).
private struct WindowAccessor: NSViewRepresentable {
    @Binding var window: NSWindow?

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        DispatchQueue.main.async {
            self.window = view.window
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async {
            if self.window !== nsView.window {
                self.window = nsView.window
            }
        }
    }
}
