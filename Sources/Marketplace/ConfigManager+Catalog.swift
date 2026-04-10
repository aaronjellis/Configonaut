import Foundation

// MARK: - ConfigManager Catalog Integration
//
// This extension wires the marketplace catalog into ConfigManager:
//
//  • `addFromCatalog(_:)`     -- install a catalog server (defaults to inactive
//                                so the user can fill in secrets first).
//  • `catalogId(forServer:)`  -- look up the catalog link for a given server name.
//  • `isReadyToEnable(...)`   -- gate used by MCPView and `moveToActive` to refuse
//                                turning on a server that still has placeholder envVars.
//
// Catalog links are persisted to
//   ~/Library/Application Support/Configonaut/catalog_links_<mode>.json
// keyed by server name so the gate survives app restarts and mode toggles.

extension ConfigManager {

    // MARK: Link Storage

    /// On-disk file that records which stored/active server names originated from
    /// which catalog entry. Separate file per mode so desktop/CLI state stays distinct.
    var catalogLinksURL: URL {
        storageDir.appendingPathComponent("catalog_links_\(mode.rawValue.lowercased()).json")
    }

    /// Load the `name -> catalogId` map from disk.
    func loadCatalogLinks() -> [String: String] {
        guard let data = try? Data(contentsOf: catalogLinksURL),
              let dict = try? JSONSerialization.jsonObject(with: data) as? [String: String]
        else { return [:] }
        return dict
    }

    @discardableResult
    func saveCatalogLinks(_ links: [String: String]) -> Bool {
        do {
            try FileManager.default.createDirectory(
                at: storageDir,
                withIntermediateDirectories: true
            )
            let data = try JSONSerialization.data(
                withJSONObject: links,
                options: [.prettyPrinted, .sortedKeys]
            )
            try data.write(to: catalogLinksURL, options: .atomic)
            return true
        } catch {
            setStatus("Couldn't save catalog link: \(error.localizedDescription)", isError: true)
            return false
        }
    }

    /// Returns the catalog id linked to a given server name, if any.
    func catalogId(forServer name: String) -> String? {
        loadCatalogLinks()[name]
    }

    /// Register a catalog link for a server name. Replaces any existing link.
    func recordCatalogLink(serverName: String, catalogId: String) {
        var links = loadCatalogLinks()
        links[serverName] = catalogId
        saveCatalogLinks(links)
    }

    /// Drop a catalog link — called from `deleteServer` when the user removes
    /// a server entirely.
    func removeCatalogLink(forServer name: String) {
        var links = loadCatalogLinks()
        guard links.removeValue(forKey: name) != nil else { return }
        saveCatalogLinks(links)
    }

    // MARK: Install from catalog

    /// Install a catalog server into Configonaut. By default the server goes to
    /// stored (inactive) so the user can fill in tokens before enabling; pass
    /// `.active` to install straight into Claude's config.
    ///
    /// - Returns: the name used for the new server entry (usually `catalogServer.id`).
    @discardableResult
    func addFromCatalog(
        _ catalogServer: CatalogServer,
        target: ServerSource = .stored,
        customName: String? = nil
    ) -> String {
        let name = resolveUniqueName(base: customName ?? catalogServer.id)
        let configDict = catalogServer.config.toConfigDict()

        switch target {
        case .stored:
            addToStored([(name, configDict)])
        case .active:
            addToActive([(name, configDict)])
        }

        recordCatalogLink(serverName: name, catalogId: catalogServer.id)
        return name
    }

    /// Install a catalog server using a user-edited config dict (rather than the
    /// catalog's default template). Used by the Marketplace inline JSON editor
    /// so the user can tweak args/env values before saving.
    @discardableResult
    func addFromCatalog(
        _ catalogServer: CatalogServer,
        customConfig: [String: Any],
        customName: String,
        target: ServerSource = .stored
    ) -> String {
        let name = resolveUniqueName(base: customName)

        switch target {
        case .stored:
            addToStored([(name, customConfig)])
        case .active:
            addToActive([(name, customConfig)])
        }

        recordCatalogLink(serverName: name, catalogId: catalogServer.id)
        return name
    }

    /// Make sure we don't stomp an existing entry with the same name.
    /// Appends `-2`, `-3`, ... until a free slot is found.
    private func resolveUniqueName(base: String) -> String {
        let existing = Set(activeServers.map(\.name) + storedServers.map(\.name))
        if !existing.contains(base) { return base }
        var i = 2
        while existing.contains("\(base)-\(i)") { i += 1 }
        return "\(base)-\(i)"
    }

    // MARK: Ready-to-enable gate

    /// Is the server ready to be turned on? A server is ready if:
    ///
    ///   • it is NOT linked to a catalog entry (manual paste — trust the user), OR
    ///   • all its required env vars have been filled with values that don't match
    ///     the placeholder heuristics in `SecretValidator`.
    func isReadyToEnable(_ name: String, catalog: Catalog?) -> Bool {
        missingSecrets(forServer: name, catalog: catalog).isEmpty
    }

    /// List of required env var names that still look like placeholders for a
    /// given server. Empty list ⇒ ready to enable. Returns empty for servers with
    /// no catalog link (we can't know what they require).
    func missingSecrets(forServer name: String, catalog: Catalog?) -> [String] {
        guard let catalogId = catalogId(forServer: name),
              let catalogServer = catalog?.servers.first(where: { $0.id == catalogId })
        else { return [] }

        // Pull the current config block (prefer active, fall back to stored).
        guard let dict = configDict(forServer: name) else { return [] }
        return SecretValidator.missingSecrets(in: dict, for: catalogServer)
    }

    /// Parse a server's persisted JSON back into a `[String: Any]` dict.
    private func configDict(forServer name: String) -> [String: Any]? {
        let entry: ServerEntry? = activeServers.first(where: { $0.name == name })
            ?? storedServers.first(where: { $0.name == name })
        guard
            let json = entry?.configJSON,
            let data = json.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }
        return obj
    }

    // MARK: Gated move-to-active

    /// Same semantics as `moveToActive(_:)` but refuses to promote if the server
    /// still has unfilled required env vars. Returns `true` if the server was
    /// actually turned on.
    @discardableResult
    func moveToActiveIfReady(_ name: String, catalog: Catalog?) -> Bool {
        let missing = missingSecrets(forServer: name, catalog: catalog)
        if !missing.isEmpty {
            let joined = missing.joined(separator: ", ")
            setStatus(
                "Can't turn on \"\(name)\" -- still need: \(joined)",
                isError: true
            )
            return false
        }
        moveToActive(name)
        return true
    }
}
