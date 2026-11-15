"""Build the UsageDock wordmark lockups with the letters as outlined paths.

Wordmark: "USAGEDOCK" in Barlow SemiBold (SIL Open Font License, so logo use
is allowed), tracked +140/1000 like the app's labels. Lockup: mark on the
left, cap height = 0.42 × mark height, centred on the dial.
"""
import re
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

HERE = Path(__file__).parent
FONT = HERE.parents[1] / "node_modules/@fontsource/barlow/files/barlow-latin-600-normal.woff"
TEXT = "USAGEDOCK"
TRACK = 0.14  # em


def wordmark_path(cap_px: float):
    f = TTFont(FONT)
    gs = f.getGlyphSet()
    cmap = f.getBestCmap()
    upm = f["head"].unitsPerEm
    cap = f["OS/2"].sCapHeight or 700
    s = cap_px / cap
    x = 0.0
    d = []
    for ch in TEXT:
        name = cmap[ord(ch)]
        pen = SVGPathPen(gs)
        # flip y (font units are y-up), baseline at y = cap_px
        gs[name].draw(TransformPen(pen, (s, 0, 0, -s, x, cap_px)))
        d.append(pen.getCommands())
        x += gs[name].width * s + TRACK * upm * s
    width = x - TRACK * upm * s
    return " ".join(d), width


def mark_inner(color: str) -> str:
    body = (HERE / "mark.svg").read_text(encoding="utf8")
    inner = re.search(r"<svg[^>]*>(.*)</svg>", body, re.S).group(1)
    return re.sub(r"<!--.*?-->", "", inner, flags=re.S).replace("#000", color)


def lockup(color: str, out: str):
    # Mark content spans y 23..220 (dial top to dock bottom) in its 256 box.
    mark_h = 256
    cap = round(mark_h * 0.2)
    d, w = wordmark_path(cap)
    gap = 40
    ty = 121 - cap / 2  # centre caps on the dial
    total = 28 + 200 + gap + w + 8
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {total:.0f} 256" width="{total:.0f}" height="256">
  <g transform="translate(-0 0)">{mark_inner(color)}</g>
  <path transform="translate({28 + 200 + gap:.1f} {ty:.1f})" d="{d}" fill="{color}"/>
</svg>
'''
    (HERE / out).write_text(svg, encoding="utf8")


def wordmark(color: str, out: str):
    d, w = wordmark_path(100)
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:.0f} 100" width="{w:.0f}" height="100">
  <path d="{d}" fill="{color}"/>
</svg>
'''
    (HERE / out).write_text(svg, encoding="utf8")


if __name__ == "__main__":
    lockup("#000", "lockup-horizontal.svg")
    lockup("#fff", "lockup-horizontal-white.svg")
    wordmark("#000", "wordmark.svg")
    wordmark("#fff", "wordmark-white.svg")
    print("ok")
