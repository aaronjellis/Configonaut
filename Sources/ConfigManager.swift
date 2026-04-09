import Foundation

// MARK: - Data Models

enum AppMode: String, CaseIterable {
    case desktop = "Desktop"
    case cli = "CLI"
}

struct ServerEntry: Identifiable, Equatable {
    let name: String
    let configJSON: String
    var id: String { name }
}

enum ServerSource { case active, stored }

enum ParseResult {
    case servers([(String, [String: Any])])
    case needsName([String: Any])
    case error(String)
}

struct HookRule: Identifiable {
    let id = UUID()
    let event: String
    let matcher: String
    let commands: [String]
    var isEnabled: Bool
}

enum AgentSource: String {
    case personal = "Personal"
    case plugin = "Plugin"
}

struct AgentEntry: Identifiable {
    let name: String
    let description: String
    let tools: [String]
    let model: String
    let color: String
    let pluginName: String
    let filePath: URL
    let source: AgentSource
    var isPluginEnabled: Bool
    var id: String { filePath.path }
}

struct SkillEntry: Identifiable {
    let name: String
    let description: String
    let source: SkillSource
    let filePath: URL
    let isEnabled: Bool
    var id: String { filePath.path }
}

enum SkillSource: String {
    case command = "Custom Command"
    case skill = "Custom Skill"
    case plugin = "Plugin"
}

struct BackupFile: Identifiable {
    let url: URL
    let date: Date
    let sizeBytes: Int64
    var id: String { url.lastPathComponent }

    var formattedDate: String {
        let fmt = DateFormatter()
        fmt.dateStyle = .medium
        fmt.timeStyle = .short
        return fmt.string(from: date)
    }

    var formattedSize: String {
        ByteCountFormatter.string(fromByteCount: sizeBytes, countStyle: .file)
    }
}

// MARK: - Config Manager

class ConfigManager: ObservableObject {
    @Published var activeServers: [ServerEntry] = []
    @Published var storedServers: [ServerEntry] = []
    @Published var hookRules: [HookRule] = []
    @Published var backupFiles: [BackupFile] = []
    @Published var agents: [AgentEntry] = []
    @Published var skills: [SkillEntry] = []
    @Published var enabledPlugins: [String: Bool] = [:]
    @Published var statusMessage = ""
    @Published var statusIsError = false
    @Published var needsRestart = false
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
    private var lastBackupDate: Date?
    private var lastBackupHash: Int?

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

    init() {
        let saved = UserDefaults.standard.string(forKey: "appMode") ?? AppMode.desktop.rawValue
        self.mode = AppMode(rawValue: saved) ?? .desktop
        reloadAll()
    }

    func reloadAll() {
        loadActive()
        loadStored()
        loadHooks()
        loadBackups()
        loadEnabledPlugins()
        loadAgents()
        loadSkills()
    }

    // MARK: - Parse Input

    func parseInput(_ text: String) -> ParseResult {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return .error("Nothing to parse.") }

        if let obj = asJSON(trimmed) { return classify(obj) }
        if let obj = asJSON("{\(trimmed)}") { return classify(obj) }

        return .error("Invalid JSON. Check for trailing commas, missing quotes, or extra braces.")
    }

    // MARK: - MCP Server Mutations

    func addToActive(_ entries: [(String, [String: Any])]) {
        var root = loadConfigRoot()
        var mcp = (root["mcpServers"] as? [String: Any]) ?? [:]
        for (name, config) in entries { mcp[name] = config }
        root["mcpServers"] = mcp

        if saveConfig(root) {
            loadActive()
            needsRestart = true
            let names = entries.map(\.0).joined(separator: ", ")
            setStatus("Added to config: \(names)", isError: false)
        }
    }

    func addToStored(_ entries: [(String, [String: Any])]) {
        var stored = loadStoredRoot()
        for (name, config) in entries { stored[name] = config }

        if saveStored(stored) {
            loadStored()
            let names = entries.map(\.0).joined(separator: ", ")
            setStatus("Saved for later: \(names)", isError: false)
        }
    }

    func moveToStored(_ name: String) {
        var root = loadConfigRoot()
        var mcp = (root["mcpServers"] as? [String: Any]) ?? [:]
        guard let config = mcp[name] else { return }

        mcp.removeValue(forKey: name)
        root["mcpServers"] = mcp

        var stored = loadStoredRoot()
        stored[name] = config

        if saveConfig(root), saveStored(stored) {
            loadActive()
            loadStored()
            needsRestart = true
            setStatus("Turned off \"\(name)\" -- saved for later.", isError: false)
        }
    }

    func moveToActive(_ name: String) {
        var stored = loadStoredRoot()
        guard let config = stored[name] else { return }

        stored.removeValue(forKey: name)

        var root = loadConfigRoot()
        var mcp = (root["mcpServers"] as? [String: Any]) ?? [:]
        mcp[name] = config
        root["mcpServers"] = mcp

        if saveConfig(root), saveStored(stored) {
            loadActive()
            loadStored()
            needsRestart = true
            setStatus("Turned on \"\(name)\".", isError: false)
        }
    }

    func deleteServer(_ name: String, from source: ServerSource) {
        switch source {
        case .active:
            var root = loadConfigRoot()
            var mcp = (root["mcpServers"] as? [String: Any]) ?? [:]
            mcp.removeValue(forKey: name)
            root["mcpServers"] = mcp
            if saveConfig(root) {
                loadActive()
                needsRestart = true
                setStatus("Removed \"\(name)\" from config.", isError: false)
            }
        case .stored:
            var stored = loadStoredRoot()
            stored.removeValue(forKey: name)
            if saveStored(stored) {
                loadStored()
                setStatus("Deleted stored server \"\(name)\".", isError: false)
            }
        }
    }

    /// Update a server's JSON config in-place. Returns true on success.
    func updateServerConfig(_ name: String, source: ServerSource, newJSON: String) -> Bool {
        guard let data = newJSON.data(using: .utf8),
              let newConfig = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            setStatus("Invalid JSON -- check for syntax errors.", isError: true)
            return false
        }

        switch source {
        case .active:
            var root = loadConfigRoot()
            var mcp = (root["mcpServers"] as? [String: Any]) ?? [:]
            mcp[name] = newConfig
            root["mcpServers"] = mcp
            if saveConfig(root) {
                loadActive()
                needsRestart = true
                setStatus("Updated \"\(name)\" config.", isError: false)
                return true
            }
        case .stored:
            var stored = loadStoredRoot()
            stored[name] = newConfig
            if saveStored(stored) {
                loadStored()
                setStatus("Updated stored \"\(name)\" config.", isError: false)
                return true
            }
        }
        return false
    }

    // MARK: - Hooks Management

    func loadHooks() {
        guard let data = try? Data(contentsOf: Self.globalSettingsURL),
              let settings = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let hooks = settings["hooks"] as? [String: Any]
        else {
            hookRules = []
            return
        }

        var entries: [HookRule] = []
        for (event, rules) in hooks {
            guard let ruleArray = rules as? [[String: Any]] else { continue }
            for rule in ruleArray {
                let matcher = rule["matcher"] as? String ?? "*"
                let isDisabled = rule["disabled"] as? Bool ?? false
                var commands: [String] = []
                if let hookList = rule["hooks"] as? [[String: Any]] {
                    for hook in hookList {
                        if let cmd = hook["command"] as? String {
                            commands.append(cmd)
                        }
                    }
                }
                if !commands.isEmpty {
                    entries.append(HookRule(
                        event: event,
                        matcher: matcher,
                        commands: commands,
                        isEnabled: !isDisabled
                    ))
                }
            }
        }
        hookRules = entries.sorted { $0.event < $1.event }
    }

    /// Returns the full hooks JSON from settings.json as a pretty-printed string
    func hooksJSON() -> String {
        guard let settings = loadSettings(),
              let hooks = settings["hooks"]
        else { return "{\n  \n}" }
        return prettyJSON(hooks)
    }

    /// Returns the JSON for a single hook rule within an event
    func hookRuleJSON(_ hook: HookRule) -> String {
        guard let settings = loadSettings(),
              let hooks = settings["hooks"] as? [String: Any],
              let ruleArray = hooks[hook.event] as? [[String: Any]]
        else { return "{}" }

        for rule in ruleArray {
            let matcher = rule["matcher"] as? String ?? "*"
            if matcher == hook.matcher {
                return prettyJSON(rule)
            }
        }
        return "{}"
    }

    /// Update a single hook rule's JSON within settings.json
    func updateHookRule(_ hook: HookRule, newJSON: String) -> Bool {
        guard let data = newJSON.data(using: .utf8),
              let newRule = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            setStatus("Invalid JSON -- check for syntax errors.", isError: true)
            return false
        }

        guard var settings = loadSettings() else {
            setStatus("Could not read settings.json.", isError: true)
            return false
        }
        var hooks = (settings["hooks"] as? [String: Any]) ?? [:]
        guard var ruleArray = hooks[hook.event] as? [[String: Any]] else {
            setStatus("Hook event \"\(hook.event)\" not found.", isError: true)
            return false
        }

        for i in ruleArray.indices {
            let matcher = ruleArray[i]["matcher"] as? String ?? "*"
            if matcher == hook.matcher {
                ruleArray[i] = newRule
                break
            }
        }

        hooks[hook.event] = ruleArray
        settings["hooks"] = hooks
        saveSettings(settings)
        loadHooks()
        setStatus("Updated \(hook.event) hook (\(hook.matcher)).", isError: false)
        return true
    }

    /// Replace the entire hooks section in settings.json
    func updateAllHooksJSON(_ newJSON: String) -> Bool {
        guard let data = newJSON.data(using: .utf8),
              let newHooks = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            setStatus("Invalid JSON -- check for syntax errors.", isError: true)
            return false
        }

        guard var settings = loadSettings() else {
            setStatus("Could not read settings.json.", isError: true)
            return false
        }
        settings["hooks"] = newHooks
        saveSettings(settings)
        loadHooks()
        setStatus("Updated hooks configuration.", isError: false)
        return true
    }

    func toggleHook(_ hook: HookRule) {
        guard var settings = loadSettings() else { return }
        var hooks = (settings["hooks"] as? [String: Any]) ?? [:]
        guard var ruleArray = hooks[hook.event] as? [[String: Any]] else { return }

        for i in ruleArray.indices {
            let matcher = ruleArray[i]["matcher"] as? String ?? "*"
            if matcher == hook.matcher {
                if hook.isEnabled {
                    ruleArray[i]["disabled"] = true
                } else {
                    ruleArray[i].removeValue(forKey: "disabled")
                }
            }
        }

        hooks[hook.event] = ruleArray
        settings["hooks"] = hooks
        saveSettings(settings)
        loadHooks()
        let action = hook.isEnabled ? "Disabled" : "Enabled"
        setStatus("\(action) \(hook.event) hook (\(hook.matcher)).", isError: false)
    }

    // MARK: - Backup Management

    func loadBackups() {
        guard let files = try? FileManager.default.contentsOfDirectory(
            at: backupDir,
            includingPropertiesForKeys: [.fileSizeKey, .creationDateKey]
        ) else {
            backupFiles = []
            return
        }

        backupFiles = files
            .filter { $0.pathExtension == "json" }
            .compactMap { url -> BackupFile? in
                let attrs = try? url.resourceValues(forKeys: [.fileSizeKey, .creationDateKey])
                return BackupFile(
                    url: url,
                    date: attrs?.creationDate ?? Date.distantPast,
                    sizeBytes: Int64(attrs?.fileSize ?? 0)
                )
            }
            .sorted { $0.date > $1.date }
    }

    func restoreBackup(_ backup: BackupFile) -> Bool {
        do {
            // Always backup current config before restoring (bypass debounce)
            forceBackup()

            let data = try Data(contentsOf: backup.url)
            // Validate it's valid JSON before restoring
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

    func deleteBackup(_ backup: BackupFile) {
        try? FileManager.default.removeItem(at: backup.url)
        loadBackups()
        setStatus("Deleted backup.", isError: false)
    }

    // MARK: - Agents & Skills

    func loadEnabledPlugins() {
        guard let settings = loadSettings(),
              let ep = settings["enabledPlugins"] as? [String: Bool]
        else {
            enabledPlugins = [:]
            return
        }
        enabledPlugins = ep
    }

    func togglePlugin(_ pluginKey: String) {
        var settings = loadSettings() ?? [:]
        var ep = (settings["enabledPlugins"] as? [String: Bool]) ?? [:]
        ep[pluginKey] = !(ep[pluginKey] ?? false)
        settings["enabledPlugins"] = ep
        saveSettings(settings)
        loadEnabledPlugins()
        loadAgents()
        setStatus("Toggled plugin \"\(pluginKey.components(separatedBy: "@").first ?? pluginKey)\".", isError: false)
    }

    func loadAgents() {
        let fm = FileManager.default
        var entries: [AgentEntry] = []

        // 1. Personal agents from ~/.claude/agents/
        if let personalFiles = try? fm.contentsOfDirectory(
            at: Self.personalAgentsDir, includingPropertiesForKeys: nil
        ) {
            for file in personalFiles where file.pathExtension == "md" {
                guard let content = try? String(contentsOf: file, encoding: .utf8) else { continue }
                let meta = parseFrontmatter(content)
                let toolList = (meta["tools"] ?? "")
                    .components(separatedBy: ",")
                    .map { $0.trimmingCharacters(in: .whitespaces) }
                    .filter { !$0.isEmpty }

                entries.append(AgentEntry(
                    name: meta["name"] ?? file.deletingPathExtension().lastPathComponent,
                    description: meta["description"] ?? "",
                    tools: toolList,
                    model: meta["model"] ?? "",
                    color: meta["color"] ?? "blue",
                    pluginName: "Personal",
                    filePath: file,
                    source: .personal,
                    isPluginEnabled: true
                ))
            }
        }

        // 2. Plugin agents
        if let pluginDirs = try? fm.contentsOfDirectory(
            at: Self.pluginsDir, includingPropertiesForKeys: nil
        ) {
            for pluginDir in pluginDirs {
                let pluginName = pluginDir.lastPathComponent
                let agentsDir = pluginDir.appendingPathComponent("agents")
                guard let agentFiles = try? fm.contentsOfDirectory(
                    at: agentsDir, includingPropertiesForKeys: nil
                ) else { continue }

                let pluginKey = "\(pluginName)@claude-plugins-official"
                let isEnabled = enabledPlugins[pluginKey] ?? false

                for file in agentFiles where file.pathExtension == "md" {
                    guard let content = try? String(contentsOf: file, encoding: .utf8) else { continue }
                    let meta = parseFrontmatter(content)
                    let toolList = (meta["tools"] ?? "")
                        .components(separatedBy: ",")
                        .map { $0.trimmingCharacters(in: .whitespaces) }
                        .filter { !$0.isEmpty }

                    entries.append(AgentEntry(
                        name: meta["name"] ?? file.deletingPathExtension().lastPathComponent,
                        description: meta["description"] ?? "",
                        tools: toolList,
                        model: meta["model"] ?? "",
                        color: meta["color"] ?? "",
                        pluginName: pluginName,
                        filePath: file,
                        source: .plugin,
                        isPluginEnabled: isEnabled
                    ))
                }
            }
        }

        agents = entries.sorted {
            // Personal first, then by plugin name, then by agent name
            if $0.source != $1.source { return $0.source == .personal }
            if $0.pluginName != $1.pluginName { return $0.pluginName < $1.pluginName }
            return $0.name < $1.name
        }
    }

    func createAgent(name: String) -> AgentEntry? {
        let fm = FileManager.default
        let safeName = name.lowercased()
            .replacingOccurrences(of: " ", with: "-")
            .filter { $0.isLetter || $0.isNumber || $0 == "-" }
        guard !safeName.isEmpty else {
            setStatus("Invalid agent name.", isError: true)
            return nil
        }

        let filePath = Self.personalAgentsDir.appendingPathComponent("\(safeName).md")
        if fm.fileExists(atPath: filePath.path) {
            setStatus("Agent \"\(safeName)\" already exists.", isError: true)
            return nil
        }

        let template = """
        ---
        name: \(safeName)
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

        do {
            try fm.createDirectory(at: Self.personalAgentsDir, withIntermediateDirectories: true)
            try template.write(to: filePath, atomically: true, encoding: .utf8)
            loadAgents()
            setStatus("Created agent \"\(safeName)\".", isError: false)
            return agents.first { $0.filePath == filePath }
        } catch {
            setStatus("Create error: \(error.localizedDescription)", isError: true)
            return nil
        }
    }

    func deleteAgent(_ agent: AgentEntry) {
        guard agent.source == .personal else {
            setStatus("Can only delete personal agents.", isError: true)
            return
        }
        do {
            try FileManager.default.removeItem(at: agent.filePath)
            loadAgents()
            setStatus("Deleted agent \"\(agent.name)\".", isError: false)
        } catch {
            setStatus("Delete error: \(error.localizedDescription)", isError: true)
        }
    }

    func loadSkills() {
        let fm = FileManager.default
        var entries: [SkillEntry] = []

        // Helper to scan a directory for .md skills
        func scanDir(_ dir: URL, source: SkillSource, isEnabled: Bool) {
            guard let items = try? fm.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil) else { return }
            for item in items {
                // Skip .disabled subdirectory itself
                if item.lastPathComponent == ".disabled" { continue }
                let skillFile: URL
                if item.pathExtension == "md" {
                    skillFile = item
                } else {
                    let candidate = item.appendingPathComponent("SKILL.md")
                    if fm.fileExists(atPath: candidate.path) {
                        skillFile = candidate
                    } else {
                        continue
                    }
                }
                guard let content = try? String(contentsOf: skillFile, encoding: .utf8) else { continue }
                let meta = parseFrontmatter(content)
                entries.append(SkillEntry(
                    name: meta["name"] ?? item.deletingPathExtension().lastPathComponent,
                    description: meta["description"] ?? "",
                    source: source,
                    filePath: skillFile,
                    isEnabled: isEnabled
                ))
            }
        }

        // Custom commands (~/.claude/commands/*.md) + disabled
        scanDir(Self.commandsDir, source: .command, isEnabled: true)
        scanDir(Self.commandsDir.appendingPathComponent(".disabled"), source: .command, isEnabled: false)

        // Custom skills (~/.claude/skills/) + disabled
        scanDir(Self.skillsDir, source: .skill, isEnabled: true)
        scanDir(Self.skillsDir.appendingPathComponent(".disabled"), source: .skill, isEnabled: false)

        // Plugin skills (read-only, controlled by plugin toggle)
        if let pluginDirs = try? fm.contentsOfDirectory(at: Self.pluginsDir, includingPropertiesForKeys: nil) {
            for pluginDir in pluginDirs {
                let pluginName = pluginDir.lastPathComponent
                let pluginKey = "\(pluginName)@claude-plugins-official"
                let isEnabled = enabledPlugins[pluginKey] ?? false
                scanDir(pluginDir.appendingPathComponent("skills"), source: .plugin, isEnabled: isEnabled)
            }
        }

        skills = entries.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    func toggleSkill(_ skill: SkillEntry) {
        // Plugin skills are toggled via plugin enable/disable, not file moves
        guard skill.source != .plugin else { return }

        let fm = FileManager.default
        let file = skill.filePath
        let parentDir = file.deletingLastPathComponent()

        if skill.isEnabled {
            // Move to .disabled/ subdirectory
            let disabledDir: URL
            if skill.source == .command {
                disabledDir = Self.commandsDir.appendingPathComponent(".disabled")
            } else {
                disabledDir = Self.skillsDir.appendingPathComponent(".disabled")
            }
            try? fm.createDirectory(at: disabledDir, withIntermediateDirectories: true)

            // For single .md files, move just the file
            // For directories (skill with SKILL.md), move the parent folder
            let itemToMove: URL
            if file.lastPathComponent == "SKILL.md" {
                itemToMove = parentDir
            } else {
                itemToMove = file
            }
            let dest = disabledDir.appendingPathComponent(itemToMove.lastPathComponent)
            try? fm.moveItem(at: itemToMove, to: dest)
            setStatus("Disabled \"\(skill.name)\".", isError: false)
        } else {
            // Move back from .disabled/ to the active directory
            let activeDir: URL
            if skill.source == .command {
                activeDir = Self.commandsDir
            } else {
                activeDir = Self.skillsDir
            }

            let itemToMove: URL
            if file.lastPathComponent == "SKILL.md" {
                itemToMove = parentDir
            } else {
                itemToMove = file
            }
            let dest = activeDir.appendingPathComponent(itemToMove.lastPathComponent)
            try? fm.moveItem(at: itemToMove, to: dest)
            setStatus("Enabled \"\(skill.name)\".", isError: false)
        }

        loadSkills()
    }

    func createSkill(name: String, source: SkillSource) -> SkillEntry? {
        let safeName = name.lowercased()
            .replacingOccurrences(of: " ", with: "-")
            .filter { $0.isLetter || $0.isNumber || $0 == "-" }
        guard !safeName.isEmpty else {
            setStatus("Invalid skill name.", isError: true)
            return nil
        }

        let dir = source == .command ? Self.commandsDir : Self.skillsDir
        let filePath = dir.appendingPathComponent("\(safeName).md")
        let fm = FileManager.default

        if fm.fileExists(atPath: filePath.path) {
            setStatus("Skill \"\(safeName)\" already exists.", isError: true)
            return nil
        }

        let template: String
        if source == .command {
            template = """
            ---
            name: \(safeName)
            description: A custom slash command
            ---

            You are executing the /\(safeName) command.

            ## Instructions

            Describe what this command should do when invoked.
            """
        } else {
            template = """
            ---
            name: \(safeName)
            description: A custom skill
            ---

            You are a specialized skill.

            ## Instructions

            Describe what this skill does and when it should activate.
            """
        }

        do {
            try fm.createDirectory(at: dir, withIntermediateDirectories: true)
            try template.write(to: filePath, atomically: true, encoding: .utf8)
            loadSkills()
            setStatus("Created \(source.rawValue.lowercased()) \"\(safeName)\".", isError: false)
            return skills.first { $0.filePath == filePath }
        } catch {
            setStatus("Create error: \(error.localizedDescription)", isError: true)
            return nil
        }
    }

    // MARK: - Frontmatter Parser

    func parseFrontmatter(_ content: String) -> [String: String] {
        let lines = content.components(separatedBy: "\n")
        guard lines.first?.trimmingCharacters(in: .whitespaces) == "---" else { return [:] }
        var result: [String: String] = [:]
        for line in lines.dropFirst() {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed == "---" { break }
            guard let colonRange = trimmed.range(of: ": ") else { continue }
            let key = String(trimmed[trimmed.startIndex..<colonRange.lowerBound])
                .trimmingCharacters(in: .whitespaces)
            var value = String(trimmed[colonRange.upperBound...])
                .trimmingCharacters(in: .whitespaces)
            // Strip surrounding quotes
            if (value.hasPrefix("\"") && value.hasSuffix("\"")) ||
               (value.hasPrefix("'") && value.hasSuffix("'")) {
                value = String(value.dropFirst().dropLast())
            }
            result[key] = value
        }
        return result
    }

    // MARK: - Private Helpers

    private func asJSON(_ text: String) -> [String: Any]? {
        guard let data = text.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }
        return obj
    }

    private func classify(_ obj: [String: Any]) -> ParseResult {
        if let mcp = obj["mcpServers"] as? [String: Any] {
            let entries = mcp.compactMap { k, v -> (String, [String: Any])? in
                (v as? [String: Any]).map { (k, $0) }
            }
            return entries.isEmpty ? .error("mcpServers is empty.") : .servers(entries)
        }

        if obj["command"] != nil {
            return .needsName(obj)
        }

        let entries = obj.compactMap { k, v -> (String, [String: Any])? in
            (v as? [String: Any]).map { (k, $0) }
        }
        return entries.isEmpty ? .error("No valid server configs found.") : .servers(entries)
    }

    private func loadActive() {
        let root = loadConfigRoot()
        guard let mcp = root["mcpServers"] as? [String: Any] else {
            activeServers = []
            return
        }
        activeServers = mcp.keys.sorted().compactMap { key in
            guard let config = mcp[key] else { return nil }
            return ServerEntry(name: key, configJSON: prettyJSON(config))
        }
    }

    private func loadStored() {
        let stored = loadStoredRoot()
        // Exclude any stored server whose name also appears in active config (dedup)
        let activeNames = Set(activeServers.map(\.name))
        storedServers = stored.keys.sorted().compactMap { key in
            guard !activeNames.contains(key), let config = stored[key] else { return nil }
            return ServerEntry(name: key, configJSON: prettyJSON(config))
        }
    }

    private func loadConfigRoot() -> [String: Any] {
        guard let data = try? Data(contentsOf: configURL),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return [:] }
        return obj
    }

    private func loadStoredRoot() -> [String: Any] {
        guard let data = try? Data(contentsOf: storedURL),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return [:] }
        return obj
    }

    private func loadSettings() -> [String: Any]? {
        guard let data = try? Data(contentsOf: Self.globalSettingsURL),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }
        return obj
    }

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

    private func saveSettings(_ settings: [String: Any]) {
        do {
            let data = try JSONSerialization.data(
                withJSONObject: settings,
                options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
            )
            try data.write(to: Self.globalSettingsURL, options: .atomic)
        } catch {
            setStatus("Settings save error: \(error.localizedDescription)", isError: true)
        }
    }

    /// Creates a backup only if the config has changed AND at least 5 minutes
    /// have passed since the last backup. Use `forceBackup()` to bypass debounce.
    func createBackup() {
        guard FileManager.default.fileExists(atPath: configURL.path) else { return }

        // Check content hash -- skip if config hasn't changed
        guard let currentData = try? Data(contentsOf: configURL) else { return }
        let currentHash = currentData.hashValue
        if currentHash == lastBackupHash { return }

        // Debounce -- skip if last backup was less than 5 minutes ago
        if let last = lastBackupDate, Date().timeIntervalSince(last) < 300 { return }

        writeBackup(data: currentData, hash: currentHash)
    }

    /// Force a backup regardless of debounce (used by restore)
    func forceBackup() {
        guard FileManager.default.fileExists(atPath: configURL.path),
              let data = try? Data(contentsOf: configURL)
        else { return }
        writeBackup(data: data, hash: data.hashValue)
    }

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

            // Keep only last 30 backups
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

    // MARK: - File Security

    /// Creates a directory with owner-only permissions (700)
    static func ensureSecureDirectory(_ url: URL) throws {
        let fm = FileManager.default
        var isDir: ObjCBool = false
        if fm.fileExists(atPath: url.path, isDirectory: &isDir), isDir.boolValue {
            // Fix permissions if directory already exists
            try fm.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
            return
        }
        try fm.createDirectory(at: url, withIntermediateDirectories: true)
        try fm.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
    }

    /// Sets owner-only read/write (600) on a file
    static func lockFile(_ url: URL) {
        try? FileManager.default.setAttributes(
            [.posixPermissions: 0o600],
            ofItemAtPath: url.path
        )
    }

    func prettyJSON(_ obj: Any) -> String {
        guard let data = try? JSONSerialization.data(
            withJSONObject: obj,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        ) else { return "{}" }
        return String(data: data, encoding: .utf8) ?? "{}"
    }

    func setStatus(_ msg: String, isError: Bool) {
        statusMessage = msg
        statusIsError = isError
    }
}
