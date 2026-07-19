import AppKit
import Foundation

guard CommandLine.arguments.count == 2 else {
    fputs("用法：GenerateAppIcon.swift <output.png>\n", stderr)
    exit(2)
}

let canvas = NSRect(x: 0, y: 0, width: 1024, height: 1024)
let image = NSImage(size: canvas.size)
image.lockFocus()

NSColor.clear.setFill()
canvas.fill()

let background = NSBezierPath(
    roundedRect: canvas.insetBy(dx: 56, dy: 56),
    xRadius: 220,
    yRadius: 220
)
NSGradient(
    colors: [
        NSColor(red: 0.08, green: 0.35, blue: 0.82, alpha: 1),
        NSColor(red: 0.02, green: 0.12, blue: 0.34, alpha: 1),
    ]
)!.draw(in: background, angle: -58)

func drawSpeaker(x: CGFloat) {
    let cabinet = NSBezierPath(
        roundedRect: NSRect(x: x, y: 214, width: 252, height: 596),
        xRadius: 74,
        yRadius: 74
    )
    NSColor(white: 0.96, alpha: 0.96).setFill()
    cabinet.fill()

    let tweeter = NSBezierPath(ovalIn: NSRect(x: x + 83, y: 650, width: 86, height: 86))
    NSColor(red: 0.08, green: 0.23, blue: 0.48, alpha: 1).setFill()
    tweeter.fill()

    let woofer = NSBezierPath(ovalIn: NSRect(x: x + 39, y: 316, width: 174, height: 174))
    NSColor(red: 0.04, green: 0.14, blue: 0.31, alpha: 1).setFill()
    woofer.fill()

    let cone = NSBezierPath(ovalIn: NSRect(x: x + 75, y: 352, width: 102, height: 102))
    NSColor(red: 0.18, green: 0.56, blue: 0.94, alpha: 1).setFill()
    cone.fill()
}

drawSpeaker(x: 218)
drawSpeaker(x: 554)

image.unlockFocus()

guard
    let tiff = image.tiffRepresentation,
    let bitmap = NSBitmapImageRep(data: tiff),
    let png = bitmap.representation(using: .png, properties: [:])
else {
    fputs("无法生成应用图标\n", stderr)
    exit(1)
}

try png.write(to: URL(fileURLWithPath: CommandLine.arguments[1]), options: .atomic)
