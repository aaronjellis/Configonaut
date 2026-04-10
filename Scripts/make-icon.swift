#!/usr/bin/env swift
// Generates a macOS app icon (.icns) from a square source PNG.
// Applies the standard Big Sur+ squircle mask centered in a 1024x1024 canvas.
//
// Usage: swift Scripts/make-icon.swift Resources/AppIcon.png Resources/AppIcon.icns

import AppKit
import Foundation

// MARK: - Arguments

let args = CommandLine.arguments
guard args.count == 3 else {
    print("Usage: swift Scripts/make-icon.swift <source.png> <output.icns>")
    exit(1)
}
let sourcePath = args[1]
let outputPath = args[2]

// MARK: - Constants (macOS Big Sur icon grid)

/// Outer canvas size (1x). All other sizes scale from this.
let canvas: CGFloat = 1024
/// Icon art area inside the canvas. Big Sur uses an 824pt squircle centered on a 1024pt canvas.
let artSize: CGFloat = 824
/// Corner radius of the squircle. Big Sur uses ~185.4 for an 824 squircle (22.5%).
let cornerRadius: CGFloat = 185.4

// Sizes iconutil expects in a .iconset directory
struct IconSize {
    let points: Int
    let scale: Int
    var pixels: Int { points * scale }
    var filename: String {
        scale == 1 ? "icon_\(points)x\(points).png"
                   : "icon_\(points)x\(points)@\(scale)x.png"
    }
}

let sizes: [IconSize] = [
    IconSize(points: 16, scale: 1),
    IconSize(points: 16, scale: 2),
    IconSize(points: 32, scale: 1),
    IconSize(points: 32, scale: 2),
    IconSize(points: 128, scale: 1),
    IconSize(points: 128, scale: 2),
    IconSize(points: 256, scale: 1),
    IconSize(points: 256, scale: 2),
    IconSize(points: 512, scale: 1),
    IconSize(points: 512, scale: 2),
]

// MARK: - Load source image

guard let source = NSImage(contentsOfFile: sourcePath) else {
    print("Error: couldn't load \(sourcePath)")
    exit(1)
}

// MARK: - Render one masked size

func renderIcon(pixelSize: Int) -> Data? {
    let size = CGFloat(pixelSize)
    let scale = size / canvas

    // Art area scaled proportionally to the target pixel size
    let artRect = NSRect(
        x: (canvas - artSize) / 2 * scale,
        y: (canvas - artSize) / 2 * scale,
        width: artSize * scale,
        height: artSize * scale
    )

    let image = NSImage(size: NSSize(width: size, height: size))
    image.lockFocus()

    let ctx = NSGraphicsContext.current?.cgContext
    ctx?.setShouldAntialias(true)
    ctx?.interpolationQuality = .high

    // Clip to squircle (rounded rectangle) inside the art area
    let clip = NSBezierPath(
        roundedRect: artRect,
        xRadius: cornerRadius * scale,
        yRadius: cornerRadius * scale
    )
    clip.addClip()

    // Draw source image filling the clipped squircle
    source.draw(in: artRect,
                from: NSRect(origin: .zero, size: source.size),
                operation: .sourceOver,
                fraction: 1.0)

    image.unlockFocus()

    guard let tiff = image.tiffRepresentation,
          let rep = NSBitmapImageRep(data: tiff) else { return nil }
    return rep.representation(using: .png, properties: [:])
}

// MARK: - Build .iconset directory

let fm = FileManager.default
let iconsetDir = (outputPath as NSString).deletingPathExtension + ".iconset"
try? fm.removeItem(atPath: iconsetDir)
try fm.createDirectory(atPath: iconsetDir, withIntermediateDirectories: true)

for size in sizes {
    guard let data = renderIcon(pixelSize: size.pixels) else {
        print("Failed to render \(size.filename)")
        exit(1)
    }
    let filePath = (iconsetDir as NSString).appendingPathComponent(size.filename)
    try data.write(to: URL(fileURLWithPath: filePath))
    print("  \(size.filename) (\(size.pixels)x\(size.pixels))")
}

// MARK: - Run iconutil to produce .icns

let task = Process()
task.launchPath = "/usr/bin/iconutil"
task.arguments = ["-c", "icns", iconsetDir, "-o", outputPath]
try task.run()
task.waitUntilExit()

if task.terminationStatus == 0 {
    // Clean up the scratch iconset directory
    try? fm.removeItem(atPath: iconsetDir)
    print("\nWrote \(outputPath)")
} else {
    print("iconutil failed with status \(task.terminationStatus)")
    exit(1)
}
