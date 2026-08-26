"""Redraw the GPQL app icons from the logo geometry."""

from PIL import Image, ImageDraw

BOX = (150, 142, 1110, 1102)

GLYPH = [
    [(950, 437), (630, 252), (310, 437), (310, 807), (630, 992), (800, 894)],
    [(630, 622), (800, 720), (800, 992)],
    [(950, 769), (800, 856), (950, 943)],
]

CUBE = [
    [(630, 426), (800, 524), (800, 720), (630, 818), (460, 720), (460, 524), (630, 426)],
    [(630, 622), (800, 524)],
    [(630, 622), (460, 524)],
    [(630, 622), (630, 818)],
]

DOTS = [(630, 426), (800, 524), (800, 720), (630, 818), (460, 720), (460, 524)]

INK = (85, 85, 92, 255)
FAINT = (85, 85, 92, 110)

SIZES = {
    "32x32.png": 32,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 1024,
    "Square30x30Logo.png": 30,
    "Square44x44Logo.png": 44,
    "Square71x71Logo.png": 71,
    "Square89x89Logo.png": 89,
    "Square107x107Logo.png": 107,
    "Square142x142Logo.png": 142,
    "Square150x150Logo.png": 150,
    "Square284x284Logo.png": 284,
    "Square310x310Logo.png": 310,
    "StoreLogo.png": 50,
}


def draw(size, supersample=4):
    canvas = size * supersample
    span = BOX[2] - BOX[0]
    scale = canvas / span

    def place(point):
        return ((point[0] - BOX[0]) * scale, (point[1] - BOX[1]) * scale)

    image = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    pen = ImageDraw.Draw(image)

    thin = max(round(9 * scale), 1)
    thick = max(round(64 * scale), 2)

    for line in CUBE:
        pen.line([place(point) for point in line], fill=FAINT, width=thin, joint="curve")

    for spot in DOTS:
        x, y = place(spot)
        radius = 18 * scale
        pen.ellipse((x - radius, y - radius, x + radius, y + radius), fill=FAINT)

    for line in GLYPH:
        points = [place(point) for point in line]

        pen.line(points, fill=INK, width=thick, joint="curve")

        for x, y in points:
            radius = thick / 2
            pen.ellipse((x - radius, y - radius, x + radius, y + radius), fill=INK)

    return image.resize((size, size), Image.LANCZOS)


for name, size in SIZES.items():
    draw(size).save(name)

icon = draw(256)
icon.save("icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])

try:
    draw(1024).save("icon.icns")
except Exception as bother:
    print("icns skipped:", bother)

print("drew", len(SIZES) + 1, "files")
