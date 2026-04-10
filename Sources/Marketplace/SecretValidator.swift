import Foundation

/// Heuristic detector for placeholder/unfilled secrets in an MCP server config.
///
/// The goal is to catch the common cases — `<YOUR_TOKEN>`, `your-api-key`,
/// `xxxxxx`, `paste-here` — without being so strict that a real (but weird-looking)
/// token gets flagged. When in doubt, this errs on the side of "looks real" so
/// users aren't blocked unnecessarily.
enum SecretValidator {

    // MARK: - Public API

    /// Check a single value against the known placeholder patterns.
    static func looksLikePlaceholder(_ raw: String, hint: String? = nil) -> Bool {
        let value = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if value.isEmpty { return true }

        // Dollar-style env reference — still unfilled.
        if value.hasPrefix("$") || value.hasPrefix("${") { return true }

        // Placeholder regexes.
        for pattern in Self.placeholderPatterns {
            if value.range(of: pattern, options: .regularExpression) != nil {
                return true
            }
        }

        // If the hint (catalog-provided placeholder) matches verbatim, that's still a placeholder.
        if let hint, !hint.isEmpty, hint == value {
            return true
        }

        return false
    }

    /// Inspect an `mcpServers` config block (the `[String: Any]` we persist) and
    /// report which catalog-declared env vars are still unfilled.
    ///
    /// - Parameters:
    ///   - configBlock: the dict stored under `mcpServers["server-name"]`.
    ///   - catalogServer: the linked catalog entry.
    /// - Returns: names of required env vars that still hold a placeholder / empty value.
    static func missingSecrets(
        in configBlock: [String: Any],
        for catalogServer: CatalogServer
    ) -> [String] {
        var missing: [String] = []
        let transport = catalogServer.config.transport
        let envDict = (configBlock["env"] as? [String: Any]) ?? [:]
        let headersDict = (configBlock["headers"] as? [String: Any]) ?? [:]
        let urlString = configBlock["url"] as? String

        for envVar in catalogServer.requiredEnvVars {
            let name = envVar.name
            let placeholder = envVar.placeholder

            switch transport {
            case .stdio:
                // Expect the token in `env[name]`.
                let raw = (envDict[name] as? String) ?? ""
                if looksLikePlaceholder(raw, hint: placeholder) {
                    missing.append(name)
                }
            case .http:
                // For remote servers, the "env var" is usually baked into a header
                // (e.g. `Authorization: Bearer <TOKEN>`) or into the URL itself.
                let headerJoined = headersDict.values
                    .compactMap { $0 as? String }
                    .joined(separator: " ")
                let haystack = [headerJoined, urlString ?? ""].joined(separator: " ")
                if haystack.isEmpty || looksLikePlaceholder(haystack, hint: placeholder) {
                    missing.append(name)
                    continue
                }
                // Also flag if the haystack still literally contains `<NAME>`.
                if haystack.contains("<\(name)>") || haystack.contains("${\(name)}") {
                    missing.append(name)
                }
            }
        }
        return missing
    }

    // MARK: - Patterns

    /// Regexes that indicate a value is a placeholder rather than a real secret.
    /// Keep these anchored or specific enough that they don't fire on real tokens.
    static let placeholderPatterns: [String] = [
        // <YOUR_TOKEN>, <token>, <changeme>
        #"^<[^>]+>$"#,
        // ${SOMETHING}
        #"^\$\{[^}]+\}$"#,
        // "your-api-key", "YOUR_TOKEN", "your_key_here"
        #"(?i)\byour[_\- ]?(?:api[_\- ]?)?(?:key|token|secret|pat)\b"#,
        // "xxxxxx..." or "xxxx-xxxx-xxxx"
        #"^x{4,}([_\- ]?x{2,})*$"#,
        // "paste-here", "paste_your_key_here", "replace_me"
        #"(?i)\b(paste|replace|insert|fill)[_\- ]?(here|me|token|key)?\b"#,
        // "changeme", "change-me"
        #"(?i)^change[_\- ]?me$"#,
        // Bearer-prefix with obvious placeholder inside.
        #"(?i)bearer\s+<[^>]+>"#,
        #"(?i)bearer\s+your[_\- ]?(?:api[_\- ]?)?(?:key|token)"#,
        // Common "todo" style placeholders.
        #"(?i)^(todo|tbd|fixme)$"#,
    ]
}
