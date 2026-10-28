// Draws Napkin's app icon into an .iconset folder (all sizes iconutil expects).
// Usage: swift scripts/make-icon.swift <out.iconset>
//
// A sheet of warm paper on a soft background: ruled lines with "typed" strokes on the left and
// results in teal on the right, the way a Napkin note reads.

import AppKit
import CoreGraphics
import Foundation

func draw(size: CGFloat) -> CGImage? {
    let pixels = Int(size)
    guard let context = CGContext(
        data: nil, width: pixels, height: pixels, bitsPerComponent: 8, bytesPerRow: 0,
        space: CGColorSpace(name: CGColorSpace.sRGB)!,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else { return nil }
    let s = size / 1024
    func rect(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat) -> CGRect {
        CGRect(x: x * s, y: y * s, width: w * s, height: h * s)
    }
    func color(_ hex: UInt32, _ alpha: CGFloat = 1) -> CGColor {
        CGColor(
            srgbRed: CGFloat((hex >> 16) & 0xff) / 255, green: CGFloat((hex >> 8) & 0xff) / 255,
            blue: CGFloat(hex & 0xff) / 255, alpha: alpha)
    }

    // Background squircle (macOS icon grid: 824 px body with 100 px margins).
    let body = rect(100, 100, 824, 824)
    let bodyPath = CGPath(roundedRect: body, cornerWidth: 185 * s, cornerHeight: 185 * s, transform: nil)
    context.saveGState()
    context.setShadow(offset: CGSize(width: 0, height: -10 * s), blur: 28 * s, color: color(0x000000, 0.28))
    context.addPath(bodyPath)
    context.setFillColor(color(0x2b2d31))
    context.fillPath()
    context.restoreGState()
    context.saveGState()
    context.addPath(bodyPath)
    context.clip()
    let gradient = CGGradient(
        colorsSpace: CGColorSpace(name: CGColorSpace.sRGB), colors: [color(0x34373d), color(0x1f2124)] as CFArray,
        locations: [0, 1])!
    context.drawLinearGradient(gradient, start: CGPoint(x: 0, y: 924 * s), end: CGPoint(x: 0, y: 100 * s), options: [])
    context.restoreGState()

    // The napkin: a slightly rotated paper sheet.
    context.saveGState()
    context.translateBy(x: 512 * s, y: 512 * s)
    context.rotate(by: -0.06)
    context.translateBy(x: -512 * s, y: -512 * s)
    let sheet = rect(222, 196, 580, 632)
    context.setShadow(offset: CGSize(width: 0, height: -14 * s), blur: 30 * s, color: color(0x000000, 0.35))
    context.addPath(CGPath(roundedRect: sheet, cornerWidth: 34 * s, cornerHeight: 34 * s, transform: nil))
    context.setFillColor(color(0xfbf6ea))
    context.fillPath()
    context.setShadow(offset: .zero, blur: 0, color: nil)

    // Ruled lines with typed strokes and results.
    let rows: [(CGFloat, CGFloat)] = [(250, 0.0), (210, 1.0), (280, 1.0), (170, 1.0)]
    for (index, row) in rows.enumerated() {
        let y = 690 - CGFloat(index) * 118
        context.setFillColor(color(0xe6dcc6))
        context.fill(rect(262, y - 30, 500, 4))
        let typed = rect(270, y, row.0, 34)
        context.addPath(CGPath(roundedRect: typed, cornerWidth: 17 * s, cornerHeight: 17 * s, transform: nil))
        context.setFillColor(color(0x3a3d43))
        context.fillPath()
        if row.1 > 0 {
            let result = rect(640, y, 112, 34)
            context.addPath(CGPath(roundedRect: result, cornerWidth: 17 * s, cornerHeight: 17 * s, transform: nil))
            context.setFillColor(color(0x2f9d80))
            context.fillPath()
        }
    }
    context.restoreGState()
    return context.makeImage()
}

func writePNG(_ image: CGImage, to url: URL) throws {
    let rep = NSBitmapImageRep(cgImage: image)
    guard let data = rep.representation(using: .png, properties: [:]) else {
        throw NSError(domain: "icon", code: 1)
    }
    try data.write(to: url)
}

let arguments = CommandLine.arguments
guard arguments.count == 2 else {
    FileHandle.standardError.write("usage: make-icon.swift <out.iconset>\n".data(using: .utf8)!)
    exit(2)
}
let folder = URL(fileURLWithPath: arguments[1])
try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
for base in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = CGFloat(base * scale)
        let name = scale == 1 ? "icon_\(base)x\(base).png" : "icon_\(base)x\(base)@2x.png"
        guard let image = draw(size: pixels) else { exit(1) }
        try writePNG(image, to: folder.appendingPathComponent(name))
    }
}
