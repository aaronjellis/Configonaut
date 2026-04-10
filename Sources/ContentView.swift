import SwiftUI
import AppKit

// MARK: - Sidebar Navigation

enum SidebarSection: String, Hashable, CaseIterable {
    case servers = "MCP Servers"
    case hooks = "Hooks"
    case agents = "Agents"
    case skills = "Skills"
    case backups = "Backups"

    var icon: String {
        switch self {
        case .servers: return "server.rack"
        case .hooks: return "arrow.triangle.branch"
        case .agents: return "person.3.fill"
        case .skills: return "star.fill"
        case .backups: return "clock.arrow.circlepath"
        }
    }

    var subtitle: String {
        switch self {
        case .servers: return "Add, remove & swap tools"
        case .hooks: return "Automation triggers"
        case .agents: return "Plugin agent configs"
        case .skills: return "Commands & slash skills"
        case .backups: return "Config history & restore"
        }
    }

    var accentColor: Color {
        switch self {
        case .servers: return Theme.green
        case .hooks: return Theme.blue
        case .agents: return Theme.purple
        case .skills: return Theme.amber
        case .backups: return Theme.cyan
        }
    }
}

// MARK: - Main Content View

struct ContentView: View {
    @State private var selection: SidebarSection = .servers
    @StateObject private var config = ConfigManager()

    /// Load the app icon. Tries the most reliable sources first so the sidebar
    /// logo renders whether we're running from the signed .app, from `swift run`,
    /// or from an unbundled build output.
    private var appIconImage: Image {
        // 1. Bundle.main flat resource (packaged via build.sh)
        if let url = Bundle.main.url(forResource: "AppIcon", withExtension: "png"),
           let nsImage = NSImage(contentsOf: url) {
            return Image(nsImage: nsImage)
        }
        // 2. Configonaut_Configonaut.bundle (SPM-generated resource bundle)
        if let bundle = Bundle.main.url(forResource: "Configonaut_Configonaut", withExtension: "bundle")
            .flatMap({ Bundle(url: $0) }),
           let url = bundle.url(forResource: "AppIcon", withExtension: "png"),
           let nsImage = NSImage(contentsOf: url) {
            return Image(nsImage: nsImage)
        }
        // 3. AppIcon.icns next to the binary (same folder as Contents/Resources)
        if let url = Bundle.main.url(forResource: "AppIcon", withExtension: "icns"),
           let nsImage = NSImage(contentsOf: url) {
            return Image(nsImage: nsImage)
        }
        // 4. macOS-resolved application icon (works if Info.plist + CFBundleIconFile are set)
        if let appIcon = NSApplication.shared.applicationIconImage,
           appIcon.size.width > 0 {
            return Image(nsImage: appIcon)
        }
        // 5. Last-resort SF Symbol
        return Image(systemName: "curlybraces")
    }

    var body: some View {
        ZStack {
            GradientBackground(
                primaryOrb: selection.accentColor,
                secondaryOrb: Theme.blue
            )
            .animation(.easeInOut(duration: 1.2), value: selection)

            HStack(spacing: 0) {
                sidebarView
                    .frame(width: 220)

                Rectangle()
                    .fill(Theme.subtleBorder)
                    .frame(width: 1)

                detailView
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(minWidth: 920, minHeight: 660)
    }

    // MARK: - Sidebar

    private var sidebarView: some View {
        VStack(alignment: .leading, spacing: 2) {
            // App branding with icon
            HStack(spacing: 10) {
                ZStack {
                    // Neon glow behind icon
                    Circle()
                        .fill(Theme.green.opacity(0.3))
                        .frame(width: 48, height: 48)
                        .blur(radius: 16)

                    // App icon
                    appIconImage
                        .resizable()
                        .interpolation(.high)
                        .aspectRatio(contentMode: .fit)
                        .frame(width: 38, height: 38)
                        .clipShape(RoundedRectangle(cornerRadius: 9))
                        .overlay(
                            RoundedRectangle(cornerRadius: 9)
                                .strokeBorder(
                                    LinearGradient(
                                        colors: [Color.white.opacity(0.3), Color.white.opacity(0.05)],
                                        startPoint: .topLeading,
                                        endPoint: .bottomTrailing
                                    ),
                                    lineWidth: 0.5
                                )
                        )
                        .shadow(color: Theme.green.opacity(0.3), radius: 8)
                }

                VStack(alignment: .leading, spacing: 1) {
                    Text("Configonaut")
                        .font(.system(size: 15, weight: .bold))
                    Text(config.mode == .desktop ? "Desktop Config" : "CLI Config")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(config.mode == .desktop ? Theme.green.opacity(0.5) : Theme.blue.opacity(0.5))
                }
            }
            .padding(.horizontal, 14)
            .padding(.top, 14)
            .padding(.bottom, 18)

            // Desktop / CLI toggle
            HStack {
                Spacer()
                Picker("Mode", selection: $config.mode) {
                    ForEach(AppMode.allCases, id: \.self) { mode in
                        Text(mode.rawValue).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                Spacer()
            }
            .padding(.bottom, 12)

            // Tools section
            sectionLabel("TOOLS")
            sidebarItem(.servers)
            sidebarItem(.hooks)

            sectionLabel("EXTEND").padding(.top, 10)
            sidebarItem(.agents)
            sidebarItem(.skills)

            Spacer()

            // Gradient divider
            Rectangle()
                .fill(
                    LinearGradient(
                        colors: [Theme.cyan.opacity(0.0), Theme.cyan.opacity(0.15), Theme.cyan.opacity(0.0)],
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                )
                .frame(height: 1)
                .padding(.horizontal, 14)

            sidebarItem(.backups)

            // Version footer
            HStack {
                Spacer()
                Text("v1.2.0")
                    .font(.system(size: 9, weight: .medium, design: .monospaced))
                    .foregroundStyle(.quaternary)
                Spacer()
            }
            .padding(.bottom, 8)
        }
        .background(.ultraThinMaterial)
    }

    private func sectionLabel(_ text: String) -> some View {
        Text(text)
            .font(.system(size: 10, weight: .bold))
            .foregroundStyle(.tertiary)
            .tracking(1.5)
            .padding(.horizontal, 18)
            .padding(.bottom, 2)
    }

    private func sidebarItem(_ item: SidebarSection) -> some View {
        Button {
            withAnimation(.easeInOut(duration: 0.2)) { selection = item }
        } label: {
            HStack(spacing: 10) {
                Image(systemName: item.icon)
                    .font(.system(size: 14))
                    .foregroundStyle(selection == item ? item.accentColor : .secondary)
                    .frame(width: 22, height: 22)

                VStack(alignment: .leading, spacing: 0) {
                    Text(item.rawValue)
                        .font(.system(size: 13, weight: selection == item ? .semibold : .regular))
                        .foregroundStyle(selection == item ? .primary : .secondary)
                    Text(item.subtitle)
                        .font(.system(size: 10))
                        .foregroundStyle(.tertiary)
                }

                Spacer()

                // Badge count
                let count = badgeCount(for: item)
                if count > 0 {
                    Text("\(count)")
                        .font(.system(size: 9, weight: .bold, design: .rounded))
                        .foregroundStyle(.white)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(
                            item.accentColor.opacity(selection == item ? 0.7 : 0.4),
                            in: Capsule()
                        )
                        .shadow(
                            color: selection == item ? item.accentColor.opacity(0.4) : .clear,
                            radius: 6
                        )
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .contentShape(Rectangle())
            .background {
                if selection == item {
                    ZStack {
                        // Neon glow behind card
                        RoundedRectangle(cornerRadius: 8)
                            .fill(item.accentColor.opacity(0.08))
                            .blur(radius: 4)
                        // Glass fill
                        RoundedRectangle(cornerRadius: 8)
                            .fill(.ultraThinMaterial)
                            .environment(\.colorScheme, .dark)
                        RoundedRectangle(cornerRadius: 8)
                            .fill(item.accentColor.opacity(0.06))
                        // Top highlight edge
                        RoundedRectangle(cornerRadius: 8)
                            .strokeBorder(
                                LinearGradient(
                                    colors: [item.accentColor.opacity(0.25), Color.clear],
                                    startPoint: .top,
                                    endPoint: .bottom
                                ),
                                lineWidth: 0.5
                            )
                    }
                }
            }
            .overlay(alignment: .leading) {
                if selection == item {
                    UnevenRoundedRectangle(
                        topLeadingRadius: 2, bottomLeadingRadius: 2,
                        bottomTrailingRadius: 2, topTrailingRadius: 2
                    )
                    .fill(item.accentColor)
                    .frame(width: 3)
                    .padding(.vertical, 6)
                    .shadow(color: item.accentColor.opacity(0.7), radius: 6)
                    .shadow(color: item.accentColor.opacity(0.3), radius: 12)
                }
            }
        }
        .buttonStyle(.plain)
        .padding(.horizontal, 8)
    }

    private func badgeCount(for item: SidebarSection) -> Int {
        switch item {
        case .servers: return config.activeServers.count
        case .hooks: return config.hookRules.count
        case .agents: return config.agents.count
        case .skills: return config.skills.count
        case .backups: return config.backupFiles.count
        }
    }

    // MARK: - Detail

    @ViewBuilder
    private var detailView: some View {
        switch selection {
        case .servers:
            MCPView(config: config)
        case .hooks:
            HooksView(config: config)
        case .agents:
            AgentsView(config: config)
        case .skills:
            SkillsView(config: config)
        case .backups:
            BackupsView(config: config)
        }
    }
}
