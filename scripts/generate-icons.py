#!/usr/bin/env python3
"""Generates the Vana handwheel icons: the app icon source and the 12-frame tray animation."""
import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"
RED = (225, 29, 42, 255)
WHITE = (245, 245, 245, 255)
BLACK = (0, 0, 0, 255)
SS = 8  # supersampling for anti-aliasing


def wheel(size, rim, spoke, hub, hub_core, angle=0.0, scale=1.0):
    """Draws a three-spoke industrial valve handwheel with grip knobs."""
    s = size * SS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    c = s / 2
    outer = s * 0.40 * scale
    thick = s * 0.095 * scale

    def line(a, r0, r1, w, color):
        x0, y0 = c + r0 * math.cos(a), c + r0 * math.sin(a)
        x1, y1 = c + r1 * math.cos(a), c + r1 * math.sin(a)
        d.line((x0, y0, x1, y1), fill=color, width=int(w))

    # Grip knobs
    for i in range(9):
        a = angle + i * 2 * math.pi / 9
        line(a, outer - thick / 2, outer + s * 0.075 * scale, s * 0.085 * scale, rim)
    d.ellipse((c - outer, c - outer, c + outer, c + outer), outline=rim, width=int(thick))
    for i in range(3):
        a = angle + i * 2 * math.pi / 3 - math.pi / 2
        line(a, 0, outer - thick / 2, s * 0.085 * scale, spoke)
    r = s * 0.13 * scale
    d.ellipse((c - r, c - r, c + r, c + r), fill=hub)
    if hub_core:
        r2 = s * 0.055 * scale
        d.ellipse((c - r2, c - r2, c + r2, c + r2), fill=hub_core)
    return img.resize((size, size), Image.LANCZOS)


def app_icon():
    size = 1024
    s = size * 2
    base = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(base)
    d.rounded_rectangle((90, 90, s - 90, s - 90), radius=420, fill=(19, 20, 23, 255), outline=(52, 54, 60, 255), width=10)
    base = base.resize((size, size), Image.LANCZOS)
    w = wheel(size, RED, RED, WHITE, RED, angle=math.radians(12), scale=0.92)
    base.alpha_composite(w)
    out = ICONS / "app-icon-source.png"
    base.save(out)
    return out


def tray_frames():
    out = ICONS / "tray"
    out.mkdir(parents=True, exist_ok=True)
    for i in range(12):
        # The three-spoke wheel is 120° symmetric; 12 frames cover one symmetry step.
        angle = math.radians(i * 10)
        wheel(44, BLACK, BLACK, BLACK, None, angle, scale=1.1).save(out / f"mono-{i:02d}.png")
        wheel(44, RED, RED, WHITE, RED, angle, scale=1.1).save(out / f"color-{i:02d}.png")


ICONS.mkdir(parents=True, exist_ok=True)
tray_frames()
print(app_icon())
