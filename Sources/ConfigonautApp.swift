import SwiftUI

@main
struct ConfigonautApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .preferredColorScheme(.dark)
        }
        .defaultSize(width: 960, height: 700)
    }
}
