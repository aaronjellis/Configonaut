import SwiftUI

// MARK: - Color Palette

extension Color {
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet(charactersIn: "#"))
        var rgb: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&rgb)
        self.init(
            red: Double((rgb >> 16) & 0xFF) / 255,
            green: Double((rgb >> 8) & 0xFF) / 255,
            blue: Double(rgb & 0xFF) / 255
        )
    }
}

enum Theme {
    // Background
    static let bgPrimary = Color(hex: "0A0E15")
    static let bgElevated = Color(hex: "111827")

    // Neon accents
    static let green = Color(hex: "00F5A0")
    static let red = Color(hex: "FF5C8A")
    static let blue = Color(hex: "00B4FF")
    static let purple = Color(hex: "B36CFF")
    static let amber = Color(hex: "FFD43B")
    static let cyan = Color(hex: "00E5FF")
    static let orange = Color(hex: "FF8A3D")

    // Surfaces
    static let cardFill = Color.white.opacity(0.04)
    static let cardHover = Color.white.opacity(0.08)
    static let cardBorder = Color.white.opacity(0.08)
    static let cardBorderHover = Color.white.opacity(0.14)
    static let subtleBorder = Color.white.opacity(0.06)
}

// MARK: - Glass Card Modifier

struct GlassCard: ViewModifier {
    var radius: CGFloat = 14
    var border: Color = Theme.cardBorder
    var fill: Color = Theme.cardFill

    func body(content: Content) -> some View {
        content
            .background {
                ZStack {
                    RoundedRectangle(cornerRadius: radius)
                        .fill(.ultraThinMaterial)
                        .environment(\.colorScheme, .dark)
                    RoundedRectangle(cornerRadius: radius)
                        .fill(fill)
                }
            }
            .overlay(
                RoundedRectangle(cornerRadius: radius)
                    .strokeBorder(
                        LinearGradient(
                            colors: [
                                Color.white.opacity(0.18),
                                border,
                                Color.white.opacity(0.04)
                            ],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        ),
                        lineWidth: 0.5
                    )
            )
    }
}

extension View {
    func glassCard(
        radius: CGFloat = 14,
        border: Color = Theme.cardBorder,
        fill: Color = Theme.cardFill
    ) -> some View {
        modifier(GlassCard(radius: radius, border: border, fill: fill))
    }
}

// MARK: - Gradient Button

struct GradientButtonStyle: ButtonStyle {
    let colors: [Color]
    let glowColor: Color
    var isEnabled: Bool = true

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.callout.weight(.semibold))
            .foregroundStyle(.white)
            .padding(.horizontal, 20)
            .padding(.vertical, 10)
            .background(
                LinearGradient(
                    colors: isEnabled ? colors : [Color.gray.opacity(0.3), Color.gray.opacity(0.2)],
                    startPoint: .leading,
                    endPoint: .trailing
                ),
                in: RoundedRectangle(cornerRadius: 10)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(
                        LinearGradient(
                            colors: [Color.white.opacity(0.3), Color.white.opacity(0.0)],
                            startPoint: .top,
                            endPoint: .bottom
                        ),
                        lineWidth: 0.5
                    )
            )
            .shadow(
                color: isEnabled ? glowColor.opacity(configuration.isPressed ? 0.6 : 0.4) : .clear,
                radius: configuration.isPressed ? 6 : 12,
                y: 2
            )
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

// MARK: - Glowing Status Dot

struct GlowDot: View {
    let color: Color
    var size: CGFloat = 8

    var body: some View {
        Circle()
            .fill(color)
            .frame(width: size, height: size)
            .shadow(color: color.opacity(0.7), radius: 4)
            .shadow(color: color.opacity(0.3), radius: 10)
    }
}

// MARK: - Section Header

struct GlassSectionHeader: View {
    let title: String
    let count: Int
    let icon: String
    let color: Color

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(color)
                .shadow(color: color.opacity(0.4), radius: 4)
            Text(title)
                .font(.headline)
            Text("\(count)")
                .font(.subheadline.weight(.medium))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 8)
                .padding(.vertical, 2)
                .background(color.opacity(0.12), in: Capsule())
            Spacer()
        }
    }
}

// MARK: - Background Gradient with Orbs

struct GradientBackground: View {
    var primaryOrb: Color = Theme.green
    var secondaryOrb: Color = Theme.blue

    var body: some View {
        ZStack {
            LinearGradient(
                colors: [Theme.bgPrimary, Theme.bgElevated, Theme.bgPrimary],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )

            Circle()
                .fill(primaryOrb.opacity(0.12))
                .frame(width: 600, height: 600)
                .blur(radius: 140)
                .offset(x: -180, y: -120)

            Circle()
                .fill(secondaryOrb.opacity(0.08))
                .frame(width: 500, height: 500)
                .blur(radius: 120)
                .offset(x: 250, y: 180)

            Circle()
                .fill(Theme.purple.opacity(0.06))
                .frame(width: 400, height: 400)
                .blur(radius: 100)
                .offset(x: 50, y: -250)

            Circle()
                .fill(Theme.cyan.opacity(0.05))
                .frame(width: 350, height: 350)
                .blur(radius: 90)
                .offset(x: -120, y: 300)
        }
        .ignoresSafeArea()
    }
}
