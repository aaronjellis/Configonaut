// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Configonaut",
    platforms: [.macOS(.v14)],
    targets: [
        .executableTarget(
            name: "Configonaut",
            path: "Sources",
            resources: [
                .copy("../Resources/AppIcon.png")
            ]
        )
    ]
)
