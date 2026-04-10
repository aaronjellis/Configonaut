import Foundation

// MARK: - Catalog Root

/// Top-level shape of `catalog-baseline.json` / the remote `catalog.json`.
/// Matches the v1 JSON Schema at marketplace-catalog/schemas/catalog-v1.json.
struct Catalog: Codable, Equatable {
    let version: String
    let generatedAt: String?
    let source: String?
    let categories: [CatalogCategory]
    let servers: [CatalogServer]
}

struct CatalogCategory: Codable, Equatable, Identifiable {
    let id: String
    let label: String
    let icon: String?
    let description: String?
}

struct CatalogPublisher: Codable, Equatable {
    let name: String
    let type: String           // "official" | "vendor" | "community"
    let verified: Bool?
}

// MARK: - Server

struct CatalogServer: Codable, Equatable, Identifiable {
    let id: String
    let name: String
    let description: String
    let category: String
    let tags: [String]?
    let publisher: CatalogPublisher
    let homepage: String?
    let repository: String?
    let license: String?
    let popularity: Int?
    let config: CatalogConfig
    let setupNotes: String?
    let envVars: [CatalogEnvVar]?

    var requiredEnvVars: [CatalogEnvVar] {
        (envVars ?? []).filter { $0.required == true }
    }

    var isVerified: Bool {
        publisher.verified == true
    }
}

struct CatalogEnvVar: Codable, Equatable, Identifiable {
    let name: String
    let description: String?
    let required: Bool?
    let secret: Bool?
    let placeholder: String?
    let helpUrl: String?
    var id: String { name }

    var isRequired: Bool { required == true }
    var isSecret: Bool { secret == true }
}

// MARK: - Config (stdio | http)

/// A heterogeneous config block: either a local stdio launch (`command`/`args`/`env`)
/// or a remote HTTP endpoint (`url`/`headers`). Decodes gracefully from either shape.
struct CatalogConfig: Codable, Equatable {
    // stdio
    let command: String?
    let args: [String]?
    let env: [String: String]?

    // http / remote
    let url: String?
    let headers: [String: String]?

    /// Raw transport enum derived from which fields are populated.
    enum Transport: String, Codable, Equatable {
        case stdio
        case http
    }

    var transport: Transport {
        (url != nil && !(url?.isEmpty ?? true)) ? .http : .stdio
    }

    /// Convert to the same `[String: Any]` shape that `ConfigManager` persists
    /// into `mcpServers`. Caller may supply overrides for env/headers values
    /// once the user has filled in tokens.
    func toConfigDict(
        envOverrides: [String: String]? = nil,
        headerOverrides: [String: String]? = nil
    ) -> [String: Any] {
        switch transport {
        case .http:
            var dict: [String: Any] = [:]
            if let url { dict["url"] = url }
            var merged = headers ?? [:]
            if let headerOverrides {
                for (k, v) in headerOverrides { merged[k] = v }
            }
            if !merged.isEmpty { dict["headers"] = merged }
            return dict

        case .stdio:
            var dict: [String: Any] = [:]
            if let command { dict["command"] = command }
            if let args, !args.isEmpty { dict["args"] = args }
            var mergedEnv = env ?? [:]
            if let envOverrides {
                for (k, v) in envOverrides { mergedEnv[k] = v }
            }
            if !mergedEnv.isEmpty { dict["env"] = mergedEnv }
            return dict
        }
    }

    // MARK: Codable

    enum CodingKeys: String, CodingKey {
        case command, args, env, url, headers
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.command = try c.decodeIfPresent(String.self, forKey: .command)
        self.args = try c.decodeIfPresent([String].self, forKey: .args)
        self.env = try c.decodeIfPresent([String: String].self, forKey: .env)
        self.url = try c.decodeIfPresent(String.self, forKey: .url)
        self.headers = try c.decodeIfPresent([String: String].self, forKey: .headers)
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encodeIfPresent(command, forKey: .command)
        try c.encodeIfPresent(args, forKey: .args)
        try c.encodeIfPresent(env, forKey: .env)
        try c.encodeIfPresent(url, forKey: .url)
        try c.encodeIfPresent(headers, forKey: .headers)
    }
}
