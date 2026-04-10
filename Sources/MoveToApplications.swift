import AppKit
import Foundation

/// Checks whether the running app bundle is in /Applications.
/// If not, prompts the user to move it there, handles the copy,
/// and relaunches from the new location.
enum MoveToApplications {

    static func promptIfNeeded() {
        #if DEBUG
        return
        #else
        guard shouldPrompt() else { return }

        let alert = NSAlert()
        alert.messageText = "Move to your Applications folder?"
        alert.informativeText = "Configonaut is meant to run from your Applications folder. Move it there for the best experience."
        alert.addButton(withTitle: "Move to Applications Folder")
        alert.addButton(withTitle: "Do Not Move")
        alert.alertStyle = .informational

        let response = alert.runModal()
        if response == .alertFirstButtonReturn {
            performMove()
        }
        #endif
    }

    // MARK: - Helpers

    private static var bundlePath: String {
        Bundle.main.bundlePath
    }

    /// Skip the prompt when:
    /// - Already in /Applications (system or user)
    /// - Running from a translocated "AppTranslocation" sandbox (Gatekeeper randomized path)
    ///   because moving from there doesn't make sense — the user should re-download instead
    /// - Running from a read-only location (e.g., a mounted DMG)
    private static func shouldPrompt() -> Bool {
        if bundlePath.hasPrefix("/Applications/") { return false }
        if bundlePath.hasPrefix("\(NSHomeDirectory())/Applications/") { return false }
        if bundlePath.contains("/AppTranslocation/") { return false }

        // Don't nag if we're inside a mounted DMG — user should copy out first
        if bundlePath.hasPrefix("/Volumes/") { return false }

        return true
    }

    private static func currentParentDisplayPath() -> String {
        let parent = (bundlePath as NSString).deletingLastPathComponent
        let home = NSHomeDirectory()
        if parent.hasPrefix(home) {
            return "~" + parent.dropFirst(home.count)
        }
        return parent
    }

    private static func performMove() {
        let fm = FileManager.default
        let source = bundlePath
        let appName = (source as NSString).lastPathComponent
        let destination = "/Applications/\(appName)"

        // If something is already at the destination, ask whether to replace
        if fm.fileExists(atPath: destination) {
            let existingAlert = NSAlert()
            existingAlert.messageText = "An older copy of Configonaut already exists in Applications."
            existingAlert.informativeText = "Replace it with this version?"
            existingAlert.addButton(withTitle: "Replace")
            existingAlert.addButton(withTitle: "Cancel")
            existingAlert.alertStyle = .warning
            guard existingAlert.runModal() == .alertFirstButtonReturn else { return }

            do {
                try fm.removeItem(atPath: destination)
            } catch {
                showError("Couldn't remove the existing copy: \(error.localizedDescription)")
                return
            }
        }

        do {
            try fm.copyItem(atPath: source, toPath: destination)
        } catch {
            showError("Couldn't copy to Applications: \(error.localizedDescription)")
            return
        }

        // Schedule the original to be trashed after we relaunch.
        // We use a short-lived shell command so the child outlives this process.
        let trashCommand = """
            sleep 1
            /bin/rm -rf \"\(source)\"
            /usr/bin/open \"\(destination)\"
            """
        let task = Process()
        task.launchPath = "/bin/bash"
        task.arguments = ["-c", trashCommand]
        do {
            try task.run()
        } catch {
            // If launching the helper fails, still try to open the new copy directly.
            NSWorkspace.shared.open(URL(fileURLWithPath: destination))
        }

        NSApp.terminate(nil)
    }

    private static func showError(_ message: String) {
        let alert = NSAlert()
        alert.messageText = "Unable to move Configonaut"
        alert.informativeText = message
        alert.alertStyle = .warning
        alert.runModal()
    }
}
