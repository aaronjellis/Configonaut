import Foundation
import SwiftUI

/// In-memory store for the MCP marketplace catalog.
///
/// Boot order:
///   1. Load the bundled baseline (`Resources/catalog-baseline.json`) synchronously
///      so the Marketplace tab is never empty.
///   2. Kick off an async refresh from the remote GitHub URL. On success the
///      result is cached to `~/Library/Application Support/Configonaut/catalog-cache.json`
///      and published.
///   3. On next launch the cache is preferred over the baseline if it's newer.
@MainActor
final class CatalogStore: ObservableObject {

    // MARK: Published state

    @Published private(set) var catalog: Catalog?
    @Published private(set) var isRefreshing = false
    @Published private(set) var lastError: String?
    @Published private(set) var lastUpdated: Date?
    @Published var searchText: String = ""
    @Published var selectedCategoryId: String? = nil   // nil = All

    // MARK: Config

    /// Override via UserDefaults key `catalogRemoteURL` if you ever rehome the repo.
    private var remoteURL: URL {
        if let override = UserDefaults.standard.string(forKey: "catalogRemoteURL"),
           let url = URL(string: override) {
            return url
        }
        return URL(string: "https://raw.githubusercontent.com/aaronellis/configonaut-catalog/main/catalog.json")!
    }

    private let cacheURL: URL = {
        let home = FileManager.default.homeDirectoryForCurrentUser
        return home
            .appendingPathComponent("Library/Application Support/Configonaut")
            .appendingPathComponent("catalog-cache.json")
    }()

    // MARK: Bootstrap

    init() {
        bootstrap()
    }

    /// Synchronously load from disk (cache > bundle). Safe to call at init.
    func bootstrap() {
        if let fromCache = loadFromCache() {
            self.catalog = fromCache
            self.lastUpdated = cacheModifiedDate()
            return
        }
        if let fromBundle = loadFromBundle() {
            self.catalog = fromBundle
        }
    }

    /// Fire-and-forget async refresh from the remote catalog URL.
    func refreshRemote() {
        guard !isRefreshing else { return }
        isRefreshing = true
        lastError = nil

        let remote = self.remoteURL
        let cache = self.cacheURL

        Task { @MainActor in
            do {
                var req = URLRequest(url: remote)
                req.cachePolicy = .reloadIgnoringLocalCacheData
                req.timeoutInterval = 10
                let (data, response) = try await URLSession.shared.data(for: req)
                if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
                    throw NSError(
                        domain: "CatalogStore",
                        code: http.statusCode,
                        userInfo: [NSLocalizedDescriptionKey: "HTTP \(http.statusCode)"]
                    )
                }
                let decoded = try JSONDecoder().decode(Catalog.self, from: data)
                try? FileManager.default.createDirectory(
                    at: cache.deletingLastPathComponent(),
                    withIntermediateDirectories: true
                )
                try? data.write(to: cache, options: .atomic)
                self.catalog = decoded
                self.lastUpdated = Date()
            } catch {
                self.lastError = error.localizedDescription
            }
            self.isRefreshing = false
        }
    }

    // MARK: Lookup

    func server(withId id: String) -> CatalogServer? {
        catalog?.servers.first { $0.id == id }
    }

    func category(withId id: String) -> CatalogCategory? {
        catalog?.categories.first { $0.id == id }
    }

    // MARK: Filtering

    /// Servers filtered by current `searchText` and `selectedCategoryId`,
    /// sorted by popularity (desc) then name.
    var filteredServers: [CatalogServer] {
        guard let all = catalog?.servers else { return [] }
        let query = searchText
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()

        let filtered = all.filter { server in
            if let catId = selectedCategoryId, server.category != catId { return false }
            if query.isEmpty { return true }
            if server.name.lowercased().contains(query) { return true }
            if server.description.lowercased().contains(query) { return true }
            if server.id.lowercased().contains(query) { return true }
            if let tags = server.tags, tags.contains(where: { $0.lowercased().contains(query) }) { return true }
            if server.publisher.name.lowercased().contains(query) { return true }
            return false
        }

        return filtered.sorted { lhs, rhs in
            let lp = lhs.popularity ?? 0
            let rp = rhs.popularity ?? 0
            if lp != rp { return lp > rp }
            return lhs.name.localizedCaseInsensitiveCompare(rhs.name) == .orderedAscending
        }
    }

    /// Returns all categories that have at least one matching server under the
    /// current filters (ignoring `selectedCategoryId` so the sidebar stays stable).
    var categoriesWithCounts: [(category: CatalogCategory, count: Int)] {
        guard let all = catalog else { return [] }
        let query = searchText
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()

        return all.categories.map { cat in
            let count = all.servers.filter { server in
                guard server.category == cat.id else { return false }
                if query.isEmpty { return true }
                return server.name.lowercased().contains(query)
                    || server.description.lowercased().contains(query)
                    || (server.tags ?? []).contains { $0.lowercased().contains(query) }
            }.count
            return (cat, count)
        }
    }

    // MARK: Private I/O

    private func loadFromBundle() -> Catalog? {
        // Executable SPM targets don't get a synthesized `Bundle.module`, so we
        // look up the baseline the same way ContentView finds AppIcon.png:
        //   1. Try Bundle.main directly (flat resources, e.g. when packaged by build.sh).
        //   2. Fall back to the SPM-generated sub-bundle `Configonaut_Configonaut.bundle`.
        if let url = Bundle.main.url(forResource: "catalog-baseline", withExtension: "json") {
            return decodeCatalog(at: url)
        }
        if let subBundleURL = Bundle.main.url(
                forResource: "Configonaut_Configonaut",
                withExtension: "bundle"
           ),
           let subBundle = Bundle(url: subBundleURL),
           let url = subBundle.url(forResource: "catalog-baseline", withExtension: "json") {
            return decodeCatalog(at: url)
        }
        return nil
    }

    private func loadFromCache() -> Catalog? {
        guard FileManager.default.fileExists(atPath: cacheURL.path) else { return nil }
        return decodeCatalog(at: cacheURL)
    }

    private func cacheModifiedDate() -> Date? {
        (try? FileManager.default.attributesOfItem(atPath: cacheURL.path)[.modificationDate]) as? Date
    }

    private func decodeCatalog(at url: URL) -> Catalog? {
        do {
            let data = try Data(contentsOf: url)
            return try JSONDecoder().decode(Catalog.self, from: data)
        } catch {
            self.lastError = "Failed to parse catalog: \(error.localizedDescription)"
            return nil
        }
    }
}
