#!/usr/bin/env python3
"""Gatecrasher's 2D menu art, drawn as TikZ and compiled, not generated as
pixels (Sam's diagrams-as-code method, as Pursuer's tools/art.py in
LargeFlatEscape is made, whose Fig class and checks this file follows).

Each picture is composed from sub-pictures defined once below (the
operative, a soldier side-on and face-on, the Memphis Mule, a portal, the
outpost wall, the guard tower, the comms tower, a warehouse, the
helicopter, the Fulton balloon) and placed by the picture's own function.
The look follows Sam_Comm_final.png in the vagrancy repository: a warm
yellow sky with outlined clouds, salmon and coral walls, sage and teal
greens, flat fills, everything outlined in one plum ink, and clutter
(sandbags, cardboard boxes, antennas, dishes, tanks).

    python3 tools/art.py              # every picture: web/art/*.png, tools/art/*.tex
    python3 tools/art.py title        # one picture

Facts each picture must get right (the list of what an image generator gets
wrong), and where this file enforces each:

- text: no words, letters or numbers in any picture; the page sets every
  word. `document` refuses any TeX that could typeset text (\\node, \\text,
  \\put, \\char ...). The alarm over the soldier's head is a drawn wedge and
  dot (`alarm`), not a character.
- sizes: title, failed, complete and graceland are 1920x1080 and opaque;
  mission1-4 are 960x540 and opaque; mule is 512x512 with a transparent
  background (`SIZE`; `check` reads every PNG's header and refuses a wrong
  size, a cutout without an alpha channel, an opaque picture with one, or a
  file over 1.5 MB). The HUD mule must clear all four edges, be centred and
  have transparent corners.
- calm left third: the page sets the title text over the left third of the
  four 1920x1080 pictures, so there only sky, sea and ground may be drawn.
  Those are drawn inside `Fig.quiet()`; every other shape records its
  leftmost point and `Fig` refuses one that reaches into the left third
  (`calm_x`). `check` then counts ink pixels in the left third of the
  rendered picture and refuses more than `CALM_INK`.
- colour is never the only cue: every filled shape has the plum outline
  (`Fig.render` draws every fill with `draw=ink`; there is no unoutlined
  fill). Fills are flat; glows and the sight cone are flat translucent
  fills, still outlined.
- the cast is original: the operative (a woman in a camouflage sneaking
  suit, dark hair in a bun, a green headband, the portal gun on her back),
  the soldiers (olive shirts, khaki trousers, helmets; elites with glowing
  goggles), the officer (red beret), the commando, and the Memphis Mule (a
  stocky brown mule, big ears, saddlebags, a red bandana with white dots).
  No real person and no known character is drawn. Nothing gory: the failed
  screen shows her lying still with the pistol dropped, nothing more.
- portals: blue (portalblue) and orange (portalorange) oval rings, drawn by
  one function, `portal`.
- repeatable: every random choice comes from a seeded generator, one seed
  per picture, so the same script draws the same pictures.
"""
import math, random, shutil, struct, subprocess, sys, tempfile, zlib
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "web" / "art"
SRC = ROOT / "tools" / "art"
PDFLATEX = shutil.which("pdflatex") or "/Library/TeX/texbin/pdflatex"
PDFTOCAIRO = shutil.which("pdftocairo") or "/opt/homebrew/bin/pdftocairo"

# Sam_Comm_final's palette (as Pursuer's art.py has it), plus Gatecrasher's own.
COLOURS = {
    "ink": "3B2340", "sky": "EFC46A", "skyhi": "F5DA92", "skylow": "E9B060", "cloud": "F2D58A",
    "wallA": "E88A73", "wallB": "EE9F7E", "wallC": "E47A7E", "wallD": "F0B07E", "wallE": "D9765F", "wallF": "E99A8A",
    "greenA": "8DBB7E", "greenB": "6FA77A", "greenC": "A9C97F", "greenD": "5E9C6A",
    "window": "F6E7A0", "windark": "8C5A64", "metal": "8D9A93", "metaldark": "5E6B66", "wood": "B9835A",
    "paving": "E2BD8C", "ground": "D3A77D", "stone": "E7C9A0", "paper": "F6E7C4",
    "cardboard": "C9A06A", "cardboarddark": "A47C4C", "red": "B3262E", "eyewhite": "FBF4E4",
    "skin": "E8B07C", "skinB": "C98B5E", "skinC": "F2C9A0", "hairdark": "2A1E33", "hairred": "A8553A",
    "leather": "33293D", "chrome": "C9D3CF", "chromedark": "7F8C88", "glass": "A9D4CB", "lamp": "FFE6A3",
    "smoke": "E8D3AE", "shingle": "C9604F", "gold": "F2B53A",
    # Gatecrasher
    "portalblue": "2FA8E8", "portalbluehi": "A6DDF5", "portalorange": "F2812E", "portalorangehi": "F9C891",
    "olive": "7A8450", "olivedark": "5C6539", "khaki": "C2A66E", "khakidark": "9E8350",
    "mule": "9A6A44", "muledark": "6B4730", "mulelight": "C99A6E", "bandana": "C8343A",
    "suit": "56704F", "camoA": "8DA867", "camoB": "3B5240",
    "lens": "8EF0C6", "lenshi": "E2FFF2",
    "sea": "5FB3B0", "seahi": "8FCFC0", "seadark": "3F8F92",
    "concrete": "E6C29A", "concretedark": "C9A07A",
    "heli": "4F8F86", "helidark": "3A6C66",
    "duskA": "7A5272", "duskB": "B06A78", "duskC": "DE8E72", "dirt": "C08A68", "dirtdark": "A87256", "beam": "FFF3BE",
    "sunA": "F3B86A", "sunB": "EE9268", "sunC": "E57766", "sundisc": "FBE3A0", "seadusk": "6F9E9C",
    "mech": "7C8C99", "mechdark": "55606E", "mechlight": "A3B0BA", "visor": "F25C3B",
    "balloon": "F8F0DE", "hay": "E9C46A", "haydark": "C99B45", "barn": "C9604F", "trim": "F6E7C4",
}

PREAMBLE = r"""\documentclass[tikz,border=0pt]{standalone}
\usepackage{xcolor}
% Palette: Sam_Comm_final.png by eye (as Pursuer's art.py), and Gatecrasher's own.
""" + "\n".join(f"\\definecolor{{{k}}}{{HTML}}{{{v}}}" for k, v in COLOURS.items()) + "\n"

# Canvas sizes in cm and in pixels: 60 px per cm for the scenes and cards.
SIZE = {"scene": ((32.0, 18.0), (1920, 1080)), "card": ((16.0, 9.0), (960, 540)), "hud": ((8.0, 8.0), (512, 512))}
OPAQUE = ("scene", "card")
CALM_INK = 0.04  # most of the left third's preview pixels that may be ink (a clutter-free sky and ground with a few outlined clouds measure 2-3%)


def f(x):
    return f"{x:.3f}".rstrip("0").rstrip(".") if abs(x) > 0.0005 else "0"


class Fig:
    """A list of inked shapes under an affine transform, so a character can
    be drawn once and placed anywhere (mirrored with sx=-1). Every shape
    records its leftmost x; outside `quiet()`, a shape left of `calm_x` is
    refused, so the left third the page writes over stays calm."""

    def __init__(self, ink=1.1, calm_x=None):
        self.items = []
        self.ink = ink
        self.T = (1, 0, 0, 1, 0, 0)
        self.calm_x = calm_x
        self._quiet = 0
        self.where = "?"

    # transforms -------------------------------------------------------
    def pt(self, x, y):
        a, b, c, d, e, g = self.T
        return (a * x + b * y + e, c * x + d * y + g)

    def k(self):
        a, b, c, d, _, _ = self.T
        return math.sqrt(abs(a * d - b * c))

    @contextmanager
    def frame(self, dx=0.0, dy=0.0, s=1.0, rot=0.0, sx=1.0, sy=1.0):
        old = self.T
        r = math.radians(rot)
        la, lb, lc, ld = s * sx * math.cos(r), -s * sy * math.sin(r), s * sx * math.sin(r), s * sy * math.cos(r)
        a, b, c, d, e, g = old
        self.T = (a * la + b * lc, a * lb + b * ld, c * la + d * lc, c * lb + d * ld, a * dx + b * dy + e, c * dx + d * dy + g)
        try:
            yield
        finally:
            self.T = old

    @contextmanager
    def quiet(self):
        """Sky, sea and ground: allowed in the calm left third."""
        self._quiet += 1
        try:
            yield
        finally:
            self._quiet -= 1

    @contextmanager
    def part(self, name):
        old = self.where
        self.where = name
        try:
            yield
        finally:
            self.where = old

    def _calm(self, xs):
        if self.calm_x is not None and not self._quiet and min(xs) < self.calm_x - 1e-6:
            raise SystemExit(f"{self.where}: a shape reaches x={min(xs):.2f}, into the calm left third (x < {self.calm_x:.2f})")

    def P(self, x, y):
        X, Y = self.pt(x, y)
        return f"({f(X)},{f(Y)})"

    # shapes ------------------------------------------------------------
    def _add(self, kind, path, colour, xs, opts="", lw=None):
        self._calm(xs)
        self.items.append({"kind": kind, "path": path, "colour": colour, "opts": opts, "lw": lw})

    def poly(self, pts, fill, opts="", lw=None):
        T = [self.pt(x, y) for x, y in pts]
        self._add("fill", " -- ".join(f"({f(X)},{f(Y)})" for X, Y in T) + " -- cycle", fill, [X for X, _ in T], opts, lw)

    def blob(self, pts, fill, tension=0.6, opts="", lw=None):
        T = [self.pt(x, y) for x, y in pts]
        coords = " ".join(f"({f(X)},{f(Y)})" for X, Y in T)
        self._add("fill", f"plot[smooth cycle, tension={tension}] coordinates {{{coords}}}", fill, [X for X, _ in T], opts, lw)

    def circle(self, x, y, r, fill, opts="", lw=None):
        X, Y = self.pt(x, y)
        R = r * self.k()
        self._add("fill", f"({f(X)},{f(Y)}) circle ({f(R)})", fill, [X - R], opts, lw)

    def ellipse(self, x, y, rx, ry, fill, n=24, **kw):
        self.blob([(x + rx * math.cos(2 * math.pi * i / n), y + ry * math.sin(2 * math.pi * i / n)) for i in range(n)], fill, tension=0.55, **kw)

    def rect(self, x0, y0, x1, y1, fill, **kw):
        self.poly([(x0, y0), (x1, y0), (x1, y1), (x0, y1)], fill, **kw)

    def stroke(self, pts, lw=1.0, colour="ink", smooth=False, opts=""):
        T = [self.pt(x, y) for x, y in pts]
        if smooth:
            path = "plot[smooth, tension=0.6] coordinates {" + " ".join(f"({f(X)},{f(Y)})" for X, Y in T) + "}"
        else:
            path = " -- ".join(f"({f(X)},{f(Y)})" for X, Y in T)
        self._add("stroke", path, colour, [X for X, _ in T], opts, lw * self.k())

    def raw(self, tex, minx):
        """Literal TikZ, with its leftmost x given so the calm check holds."""
        self._calm([minx])
        self.items.append({"kind": "raw", "tex": tex})

    # output ------------------------------------------------------------
    def render(self):
        out = []
        for it in self.items:
            if it["kind"] == "raw":
                out.append(it["tex"])
            elif it["kind"] == "fill":
                lw = self.ink * (it["lw"] or 1.0)
                out.append(f"\\filldraw[draw=ink, line width={f(lw)}pt, line join=round, fill={it['colour']}{it['opts']}] {it['path']};\n")
            else:
                out.append(f"\\draw[{it['colour']}, line width={f(it['lw'])}pt, line cap=round, line join=round{it['opts']}] {it['path']};\n")
        return "".join(out)


# ---- shared parts ---------------------------------------------------------

def limb(g, pts, w, fill, lw=None):
    """An arm, leg or strap: a polyline thickened to width w (a number or one
    per point), rounded."""
    ws = w if isinstance(w, (list, tuple)) else [w] * len(pts)
    left, right = [], []
    n = len(pts)
    for i, (x, y) in enumerate(pts):
        if i == 0:
            dx, dy = pts[1][0] - x, pts[1][1] - y
        elif i == n - 1:
            dx, dy = x - pts[i - 1][0], y - pts[i - 1][1]
        else:
            dx, dy = pts[i + 1][0] - pts[i - 1][0], pts[i + 1][1] - pts[i - 1][1]
        L = math.hypot(dx, dy) or 1
        nx, ny = -dy / L, dx / L
        left.append((x + nx * ws[i] / 2, y + ny * ws[i] / 2))
        right.append((x - nx * ws[i] / 2, y - ny * ws[i] / 2))
    g.blob(left + right[::-1], fill, tension=0.3, lw=lw)


def bar(g, pts, w, colour, smooth=False):
    """A rod or wire with its own ink edge: an ink stroke, then the colour."""
    g.stroke(pts, lw=w + 2 * g.ink, colour="ink", smooth=smooth)
    g.stroke(pts, lw=w, colour=colour, smooth=smooth)


def camo(g, rng, pts, w, n=3):
    """Camouflage patches along a limb: small irregular blobs inside it."""
    w = sum(w) / len(w) if isinstance(w, (list, tuple)) else w
    for _ in range(n):
        i = rng.randrange(len(pts) - 1)
        t = rng.uniform(0.2, 0.8)
        (x0, y0), (x1, y1) = pts[i], pts[i + 1]
        L = math.hypot(x1 - x0, y1 - y0) or 1
        nx, ny = -(y1 - y0) / L, (x1 - x0) / L
        o = rng.uniform(-0.18, 0.18) * w
        cx, cy = x0 + t * (x1 - x0) + nx * o, y0 + t * (y1 - y0) + ny * o
        r = w * rng.uniform(0.14, 0.22)
        m = 6
        a0 = rng.uniform(0, 6.28)
        g.blob([(cx + r * rng.uniform(0.6, 1.15) * math.cos(a0 + 2 * math.pi * j / m) * 1.4,
                 cy + r * rng.uniform(0.6, 1.15) * math.sin(a0 + 2 * math.pi * j / m)) for j in range(m)],
               rng.choice(["camoA", "camoB", "camoA"]), tension=0.5, lw=0.55)


def star(g, x, y, r, fill, points=5, inner=0.45, rot=90, lw=None):
    pts = []
    for i in range(points * 2):
        a = math.radians(rot + i * 180 / points)
        rr = r if i % 2 == 0 else r * inner
        pts.append((x + rr * math.cos(a), y + rr * math.sin(a)))
    g.poly(pts, fill, lw=lw)


def glow(g, x, y, r, colour="lens", rings=(2.6, 1.8)):
    """A light's glow: flat translucent discs, each still outlined."""
    for k in rings:
        g.circle(x, y, r * k, colour, opts=", fill opacity=0.28, draw opacity=0.35", lw=0.5)


def portal(g, x, y, rx, ry, which, rot=0):
    """A portal: an oval ring in its colour round a pale swirling middle,
    sparks flying off the rim."""
    c, hi = ("portalblue", "portalbluehi") if which == "blue" else ("portalorange", "portalorangehi")
    with g.frame(x, y, 1.0, rot):
        g.ellipse(0, 0, rx * 1.28, ry * 1.18, hi, opts=", fill opacity=0.85, draw opacity=0.6", lw=0.6)
        g.ellipse(0, 0, rx, ry, c, lw=1.4)
        g.ellipse(0, 0, rx * 0.72, ry * 0.8, hi)
        g.ellipse(0, 0, rx * 0.45, ry * 0.55, "white", opts=", fill opacity=0.85", lw=0.6)
        for k in range(3):
            a0 = 40 + k * 120
            pts = [((0.62 - 0.18 * t) * rx * math.cos(math.radians(a0 + 110 * t)), (0.7 - 0.2 * t) * ry * math.sin(math.radians(a0 + 110 * t))) for t in (0, 0.33, 0.66, 1)]
            g.stroke(pts, lw=1.6, colour=c, smooth=True)
        for a, d in ((20, 1.2), (75, 1.25), (160, 1.18), (215, 1.3), (300, 1.22)):
            px, py = d * rx * math.cos(math.radians(a)), d * ry * math.sin(math.radians(a))
            star(g, px, py, 0.13 * rx, c, points=4, inner=0.35, lw=0.6)


# ---- the operative ----------------------------------------------------------

def pistol(g):
    """The silenced pistol, grip at the origin, muzzle along +x."""
    g.poly([(-0.06, -0.02), (0.12, -0.02), (0.06, -0.38), (-0.14, -0.36)], "leather", lw=0.8)
    g.rect(-0.14, -0.02, 0.5, 0.15, "metaldark", lw=0.8)
    g.rect(0.5, 0.0, 1.12, 0.13, "hairdark", lw=0.8)
    for k in range(3):
        g.stroke([(0.62 + k * 0.16, 0.0), (0.62 + k * 0.16, 0.13)], lw=0.5, colour="metal")
    g.stroke([(0.02, -0.02), (0.08, -0.14), (0.16, -0.02)], lw=0.8)


def portal_gun(g):
    """The portal gun slung on her back: a chunky tube with a blue and an
    orange ring light at its two ends. Centre at the origin, along x."""
    g.rect(-0.75, -0.22, 0.75, 0.22, "metal", lw=0.9)
    g.rect(-0.4, -0.27, 0.4, 0.27, "chromedark", lw=0.8)
    g.circle(-0.82, 0, 0.24, "portalblue", lw=0.8)
    g.circle(-0.82, 0, 0.11, "portalbluehi", lw=0.6)
    g.circle(0.82, 0, 0.24, "portalorange", lw=0.8)
    g.circle(0.82, 0, 0.11, "portalorangehi", lw=0.6)


def her_head(g, x, y, r, rng, down=False):
    """Her head side-on, facing +x: skin, dark hair in a bun at the back, a
    green headband, an earpiece. `down` hides the face (lying face down)."""
    if down:
        g.rect(x - r - 0.3, y - 0.15, x - r + 0.12, y + 0.13, "suit", lw=0.8)
    else:
        g.rect(x - 0.17, y - r - 0.28, x + 0.13, y - r + 0.12, "suit", lw=0.8)
    g.circle(x, y, r, "skin")
    if not down:
        g.poly([(x + r * 0.92, y + 0.02), (x + r * 1.2, y - 0.1), (x + r * 0.95, y - 0.16)], "skin", lw=0.7)
        g.circle(x + r * 0.5, y + 0.06, 0.045, "ink", lw=0.3)
        g.stroke([(x + r * 0.32, y + 0.17), (x + r * 0.7, y + 0.2)], lw=1.2)
        g.stroke([(x + r * 0.62, y - 0.22), (x + r * 0.85, y - 0.2)], lw=0.9)
    hair = [(x - r * 1.02, y - r * 0.55), (x - r * 1.05, y + r * 0.3), (x - r * 0.6, y + r * 0.95), (x + r * 0.2, y + r * 1.08),
            (x + r * 0.85, y + r * 0.62), (x + r * 0.55, y + r * 0.42), (x + r * 0.05, y + r * 0.35), (x - r * 0.25, y - r * 0.1), (x - r * 0.45, y - r * 0.7)]
    g.blob(hair, "hairdark", tension=0.45)
    g.circle(x - r * 1.05, y + r * 0.62, r * 0.42, "hairdark")
    limb(g, [(x - r * 0.95, y + r * 0.42), (x + r * 0.2, y + r * 0.7), (x + r * 0.92, y + r * 0.45)], r * 0.26, "greenD", lw=0.7)
    limb(g, [(x - r * 1.0, y + r * 0.42), (x - r * 1.5, y + r * 0.05)], [r * 0.2, r * 0.12], "greenD", lw=0.7)
    if not down:
        g.circle(x - r * 0.15, y - r * 0.05, r * 0.13, "leather", lw=0.7)
        g.stroke([(x - r * 0.05, y - r * 0.2), (x + r * 0.45, y - r * 0.45)], lw=0.9)


def operative(g, rng, pose="crouch"):
    """The operative, side-on, facing +x, about 3.6 tall standing, feet on
    y=0. 'crouch': one knee down, the silenced pistol held out in both
    hands. 'prone': lying face down, head to +x, an arm flung out (the
    failed screen; the pistol is placed by the picture)."""
    if pose == "crouch":
        with g.frame(-0.35, 1.95, 1.0, 72):
            portal_gun(g)
        far_arm = [(0.18, 2.35), (0.55, 1.85), (1.12, 2.12)]
        limb(g, far_arm, [0.3, 0.26, 0.24], "camoB")
        back_thigh = [(-0.12, 1.3), (-0.3, 0.3)]
        limb(g, back_thigh, [0.62, 0.48], "suit")
        camo(g, rng, back_thigh, 0.55, 2)
        shin = [(-0.3, 0.26), (-1.05, 0.26)]
        limb(g, shin, [0.46, 0.38], "suit")
        camo(g, rng, shin, 0.42, 1)
        g.blob([(-0.9, 0.02), (-1.45, 0.05), (-1.5, 0.42), (-1.2, 0.55), (-0.95, 0.45)], "leather", tension=0.3)
        thigh = [(0.0, 1.25), (0.78, 1.12)]
        limb(g, thigh, [0.64, 0.5], "suit")
        camo(g, rng, thigh, 0.55, 2)
        shin2 = [(0.78, 1.12), (0.72, 0.3)]
        limb(g, shin2, [0.5, 0.42], "suit")
        camo(g, rng, shin2, 0.45, 1)
        g.blob([(0.45, 0.0), (1.2, 0.0), (1.18, 0.22), (0.98, 0.42), (0.5, 0.42)], "leather", tension=0.25)
        torso = [(-0.1, 1.2), (0.12, 1.9), (0.28, 2.5)]
        limb(g, torso, [0.78, 0.86, 0.84], "suit")
        camo(g, rng, torso, 0.8, 4)
        g.rect(-0.45, 1.38, 0.08, 1.6, "leather", lw=0.7)
        g.rect(-0.38, 1.05, 0.02, 1.42, "olivedark", lw=0.7)
        her_head(g, 0.45, 2.98, 0.34, rng)
        near_arm = [(0.22, 2.38), (0.62, 1.82), (1.15, 2.08)]
        limb(g, near_arm, [0.34, 0.28, 0.25], "suit")
        camo(g, rng, near_arm, 0.3, 2)
        with g.frame(1.14, 2.16, 0.95, 4):
            pistol(g)
        g.circle(1.13, 2.1, 0.15, "leather", lw=0.8)
    else:
        far_leg = [(-1.35, 0.42), (-2.15, 0.62), (-2.95, 0.45)]
        limb(g, far_leg, [0.5, 0.42, 0.38], "camoB")
        g.blob([(-2.9, 0.2), (-3.35, 0.25), (-3.4, 0.72), (-3.0, 0.75)], "leather", tension=0.3)
        under_arm = [(-0.55, 0.45), (-0.95, 0.18), (-0.5, 0.1)]
        limb(g, under_arm, 0.26, "camoB")
        leg = [(-1.4, 0.3), (-2.3, 0.26), (-3.1, 0.24)]
        limb(g, leg, [0.52, 0.44, 0.4], "suit")
        camo(g, rng, leg, 0.46, 3)
        g.blob([(-3.05, 0.02), (-3.55, 0.05), (-3.6, 0.5), (-3.12, 0.48)], "leather", tension=0.3)
        with g.frame(-0.9, 0.98, 0.9, 4):
            portal_gun(g)
        torso = [(-1.55, 0.36), (-0.9, 0.42), (-0.3, 0.42)]
        limb(g, torso, [0.66, 0.78, 0.8], "suit")
        camo(g, rng, torso, 0.75, 4)
        limb(g, [(-0.4, 0.8), (-1.3, 0.72)], 0.09, "leather", lw=0.6)
        arm = [(-0.4, 0.5), (0.5, 0.26), (1.3, 0.16)]
        limb(g, arm, [0.32, 0.27, 0.24], "suit")
        camo(g, rng, arm, 0.28, 2)
        g.blob([(1.25, 0.02), (1.62, 0.04), (1.66, 0.3), (1.3, 0.36)], "leather", tension=0.4)
        her_head(g, 0.3, 0.4, 0.36, rng, down=True)


# ---- soldiers -----------------------------------------------------------------

def rifle(g):
    """A rifle, grip at the origin, muzzle along +x."""
    g.poly([(-0.85, -0.12), (-0.22, 0.0), (-0.22, 0.2), (-0.85, 0.1)], "wood", lw=0.8)
    g.rect(-0.22, -0.02, 0.55, 0.2, "leather", lw=0.8)
    g.poly([(0.12, -0.02), (0.32, -0.02), (0.38, -0.38), (0.2, -0.4)], "leather", lw=0.8)
    g.rect(0.45, 0.02, 0.95, 0.18, "olivedark", lw=0.8)
    g.rect(0.95, 0.07, 1.3, 0.13, "hairdark", lw=0.7)
    g.rect(0.05, 0.2, 0.3, 0.3, "hairdark", lw=0.7)


def helmet_side(g, x, y, r, elite=False):
    g.blob([(x - r * 1.18, y + r * 0.05), (x - r * 1.05, y + r * 0.85), (x, y + r * 1.32), (x + r * 1.0, y + r * 0.92), (x + r * 1.22, y + r * 0.25),
            (x + r * 1.3, y + r * 0.12), (x - r * 1.2, y - r * 0.05)], "olive", tension=0.45)
    g.stroke([(x - r * 0.8, y + r * 0.5), (x + r * 0.2, y + r * 1.0)], lw=0.8, colour="olivedark")
    if elite:
        limb(g, [(x - r * 1.05, y - r * 0.05), (x + r * 0.6, y + r * 0.05)], r * 0.22, "leather", lw=0.6)
        glow(g, x + r * 0.85, y + r * 0.02, r * 0.3)
        g.circle(x + r * 0.85, y + r * 0.02, r * 0.3, "leather", lw=0.7)
        g.circle(x + r * 0.88, y + r * 0.02, r * 0.2, "lens", lw=0.6)
        g.circle(x + r * 0.82, y + r * 0.08, r * 0.06, "lenshi", lw=0.3)


def soldier_side(g, rng, elite=False):
    """A guard side-on, facing +x, feet on y=0, about 3.6 tall: helmet,
    olive shirt and vest, khaki trousers, boots, a rifle held ready."""
    limb(g, [(0.0, 2.05), (0.65, 2.0), (0.95, 2.3)], [0.28, 0.25, 0.22], "olivedark")
    for fx, hx in ((-0.25, -0.05), (0.32, 0.12)):
        limb(g, [(hx, 1.75), ((hx + fx) / 2 + 0.03, 0.95), (fx, 0.25)], [0.5, 0.42, 0.38], "khaki" if fx > 0 else "khakidark")
        g.blob([(fx - 0.22, 0.0), (fx + 0.42, 0.0), (fx + 0.38, 0.18), (fx + 0.15, 0.36), (fx - 0.2, 0.36)], "leather", tension=0.25)
    g.poly([(-0.36, 1.62), (0.42, 1.62), (0.45, 2.85), (-0.36, 2.88)], "olive")
    g.poly([(0.05, 1.7), (0.47, 1.72), (0.48, 2.55), (0.08, 2.6)], "olivedark")
    for y in (1.82, 2.18):
        g.rect(0.18, y, 0.48, y + 0.28, "olive", lw=0.7)
    g.rect(-0.38, 1.6, 0.45, 1.76, "leather", lw=0.8)
    g.rect(-0.02, 2.85, 0.24, 2.98, "skin", lw=0.7)
    g.circle(0.1, 3.2, 0.3, "skin")
    g.poly([(0.38, 3.2), (0.48, 3.1), (0.37, 3.06)], "skin", lw=0.6)
    if not elite:
        g.circle(0.27, 3.22, 0.04, "ink", lw=0.3)
        g.stroke([(0.2, 3.32), (0.33, 3.31)], lw=1.0)
    g.stroke([(0.24, 3.0), (0.36, 3.0)], lw=0.8)
    helmet_side(g, 0.08, 3.25, 0.33, elite)
    with g.frame(0.55, 2.28, 0.95, 4):
        rifle(g)
    limb(g, [(-0.02, 2.68), (0.2, 2.1), (0.6, 2.25)], [0.32, 0.27, 0.24], "olive")
    g.circle(0.6, 2.25, 0.12, "skin", lw=0.7)
    g.circle(0.98, 2.33, 0.11, "skin", lw=0.7)


def soldier_front(g, rng, pose="stand", hat="helmet", shirt="olive"):
    """A soldier face-on, feet on y=0, centre line x=0, about 3.6 tall.
    pose: 'stand', 'wave' (right arm up), 'wave2' (both up), 'rail' (hands
    on a railing at waist height), 'rifle' (rifle across the chest).
    hat: 'helmet', 'cap', 'beret' (the officer), 'bandage', 'elite'."""
    for s in (-1, 1):
        limb(g, [(s * 0.24, 1.75), (s * 0.3, 0.95), (s * 0.32, 0.28)], [0.48, 0.42, 0.38], "khaki")
        boot = [(s * 0.1, 0.0), (s * 0.62, 0.0), (s * 0.58, 0.2), (s * 0.5, 0.4), (s * 0.14, 0.4)]
        g.blob(boot, "leather", tension=0.25)
        g.stroke([(s * 0.3, 1.25), (s * 0.3, 0.85)], lw=0.8, colour="khakidark")
    g.poly([(-0.52, 1.62), (0.52, 1.62), (0.6, 2.88), (-0.6, 2.88)], shirt)
    g.rect(-0.54, 1.6, 0.54, 1.78, "leather", lw=0.8)
    g.rect(-0.1, 1.6, 0.1, 1.78, "gold" if hat == "beret" else "chromedark", lw=0.6)
    for s in (-1, 1):
        g.rect(s * 0.14, 2.3, s * 0.44, 2.6, shirt, lw=0.7)
        g.stroke([(s * 0.14, 2.5), (s * 0.44, 2.5)], lw=0.6)
    g.poly([(-0.2, 2.88), (0, 2.6), (0.2, 2.88)], "skin", lw=0.7)
    g.stroke([(0, 2.6), (0, 1.8)], lw=0.7)
    if hat == "beret":
        for s in (-1, 1):
            g.rect(s * 0.38, 2.8, s * 0.62, 2.92, "gold", lw=0.6)
        for k, col in enumerate(("red", "gold", "portalblue")):
            g.rect(-0.46 + k * 0.1, 2.2, -0.38 + k * 0.1, 2.36, col, lw=0.5)

    def arm(pts, w=(0.3, 0.27, 0.25)):
        limb(g, pts, list(w), shirt)
        g.circle(pts[-1][0], pts[-1][1], 0.15, "skin", lw=0.8)

    def wave_arm(s):
        limb(g, [(s * 0.55, 2.72), (s * 0.95, 3.15), (s * 1.02, 3.75)], [0.3, 0.27, 0.25], shirt)
        hx, hy = s * 1.04, 3.95
        g.blob([(hx - 0.16, hy - 0.18), (hx + 0.16, hy - 0.18), (hx + 0.18, hy + 0.08), (hx - 0.16, hy + 0.08)], "skin", tension=0.4, lw=0.8)
        for k in range(4):
            fx = hx - 0.13 + k * 0.088
            limb(g, [(fx, hy + 0.04), (fx + (k - 1.5) * 0.04, hy + 0.3)], [0.08, 0.07], "skin", lw=0.6)
        limb(g, [(hx - s * 0.15, hy - 0.05), (hx - s * 0.3, hy + 0.1)], [0.08, 0.07], "skin", lw=0.6)
        for k in range(2):
            g.stroke([(hx + s * (0.3 + k * 0.12), hy + 0.1 + k * 0.1), (hx + s * (0.34 + k * 0.12), hy + 0.32 + k * 0.1)], lw=1.0, smooth=False)

    if pose == "rifle":
        arm([(-0.56, 2.72), (-0.7, 2.15), (-0.3, 2.1)])
        with g.frame(-0.4, 2.15, 0.9, 32):
            rifle(g)
        arm([(0.56, 2.72), (0.7, 2.2), (0.35, 2.55)])
    else:
        for s in (-1, 1):
            if pose == "wave2" or (pose == "wave" and s == 1):
                wave_arm(s)
            elif pose == "rail":
                arm([(s * 0.56, 2.72), (s * 0.72, 2.2), (s * 0.85, 1.85)])
            else:
                arm([(s * 0.56, 2.72), (s * 0.7, 2.15), (s * 0.68, 1.6)])
    # head
    g.rect(-0.13, 2.82, 0.13, 2.98, "skin", lw=0.7)
    for s in (-1, 1):
        g.circle(s * 0.31, 3.2, 0.08, "skin", lw=0.7)
    g.ellipse(0, 3.22, 0.3, 0.36, "skin")
    if hat == "elite":
        limb(g, [(-0.32, 3.26), (0.32, 3.26)], 0.08, "leather", lw=0.5)
        for s in (-1, 1):
            glow(g, s * 0.12, 3.26, 0.1, rings=(2.2,))
            g.circle(s * 0.12, 3.26, 0.11, "leather", lw=0.6)
            g.circle(s * 0.12, 3.26, 0.075, "lens", lw=0.5)
    elif hat == "beret":
        for s in (-1, 1):
            g.blob([(s * 0.02, 3.25), (s * 0.24, 3.27), (s * 0.24, 3.17), (s * 0.05, 3.16)], "hairdark", tension=0.4, lw=0.6)
        g.stroke([(-0.03, 3.24), (0.03, 3.24)], lw=0.8)
        g.blob([(-0.17, 2.98), (-0.05, 3.03), (0.05, 3.03), (0.17, 2.98), (0.05, 2.99), (0, 3.01), (-0.05, 2.99)], "hairdark", tension=0.3, lw=0.6)
        g.stroke([(-0.08, 2.92), (0.08, 2.92)], lw=0.8)
    else:
        for s in (-1, 1):
            g.circle(s * 0.12, 3.25, 0.035, "ink", lw=0.3)
        g.stroke([(-0.1, 3.04), (0, 3.0), (0.1, 3.04)], lw=0.9, smooth=True)
    if hat in ("helmet", "elite"):
        g.blob([(-0.42, 3.3), (-0.36, 3.58), (0, 3.74), (0.36, 3.58), (0.42, 3.3), (0.48, 3.26), (-0.48, 3.26)], "olive", tension=0.45)
        g.stroke([(-0.25, 3.5), (0.05, 3.62)], lw=0.8, colour="olivedark")
    elif hat == "cap":
        g.blob([(-0.33, 3.36), (-0.3, 3.6), (0, 3.66), (0.3, 3.6), (0.33, 3.36)], "olive", tension=0.4)
        g.blob([(-0.36, 3.36), (0.36, 3.36), (0.3, 3.29), (-0.3, 3.29)], "olivedark", tension=0.3, lw=0.7)
    elif hat == "bandage":
        g.blob([(-0.3, 3.3), (-0.26, 3.52), (0, 3.6), (0.26, 3.52), (0.3, 3.3)], "hairred", tension=0.4)
        limb(g, [(-0.31, 3.38), (0.31, 3.4)], 0.12, "paper", lw=0.6)
        g.stroke([(0.18, 3.46), (0.3, 3.62)], lw=1.4, colour="paper")
    elif hat == "beret":
        g.blob([(-0.34, 3.36), (-0.2, 3.62), (0.25, 3.7), (0.5, 3.55), (0.38, 3.36)], "bandana", tension=0.5)
        g.rect(-0.34, 3.34, 0.36, 3.42, "red", lw=0.6)
        g.circle(-0.15, 3.5, 0.06, "gold", lw=0.5)


def boots_closeup(g, x, s=1.0):
    """A guard standing over you, seen from the dirt: two khaki trouser legs
    running up off the top of the frame into heavy laced boots. (x, 0) is
    between the boots."""
    with g.frame(x, 0, s):
        for dx, ftw in ((-1.1, -1), (1.0, 1)):
            limb(g, [(dx * 0.7, 16), (dx * 0.9, 6.5), (dx, 1.45)], [0.95, 1.5, 1.45], "khaki" if dx > 0 else "khakidark")
            g.stroke([(dx * 0.7 + 0.2, 15), (dx * 0.9 + 0.3, 6.5), (dx + 0.35, 3.3)], lw=1.0, colour="khakidark")
            g.poly([(dx - 0.85, 1.5), (dx + 0.85, 1.5), (dx + 0.82, 2.0), (dx - 0.82, 2.0)], "khaki" if dx > 0 else "khakidark")
            boot = [(dx - 0.85, 0.0), (dx + 0.85 + ftw * 0.0, 0.0), (dx + 0.82, 1.6), (dx - 0.82, 1.6)]
            g.poly(boot, "leather")
            g.blob([(dx - 1.15, 0.0), (dx + 1.15, 0.0), (dx + 1.1, 0.45), (dx + 0.6, 0.62), (dx - 0.6, 0.62), (dx - 1.1, 0.45)], "leather", tension=0.25)
            g.rect(dx - 1.15, -0.05, dx + 1.15, 0.18, "hairdark", lw=0.8)
            for k in range(4):
                y = 0.75 + k * 0.22
                g.stroke([(dx - 0.3, y), (dx + 0.3, y + 0.12)], lw=1.0, colour="khaki")
                g.stroke([(dx + 0.3, y), (dx - 0.3, y + 0.12)], lw=1.0, colour="khaki")


def alarm(g, x, y, s=1.0):
    """The alarm over a soldier's head: a drawn exclamation mark (a tapering
    wedge and a dot) on a spiky burst, with shock lines. A path, not text."""
    with g.frame(x, y, s):
        star(g, 0, 0.95, 1.05, "lamp", points=10, inner=0.72, rot=90)
        g.poly([(-0.24, 1.75), (0.24, 1.75), (0.11, 0.62), (-0.11, 0.62)], "bandana")
        g.circle(0, 0.32, 0.16, "bandana")
        for a in (25, 55, 125, 155):
            r = math.radians(a)
            g.stroke([(1.2 * math.cos(r), 0.95 + 1.2 * math.sin(r)), (1.6 * math.cos(r), 0.95 + 1.6 * math.sin(r))], lw=1.8)


def commando(g, rng):
    """The captured commando, sitting on the floor face-on, knees up, wrists
    tied with rope round his shins. Centre x=0, seat at y=0.35."""
    for s in (-1, 1):
        limb(g, [(s * 0.28, 0.45), (s * 0.5, 1.3)], [0.48, 0.42], "greenD")
        limb(g, [(s * 0.5, 1.3), (s * 0.52, 0.25)], [0.42, 0.36], "greenB")
        g.blob([(s * 0.25, 0.0), (s * 0.82, 0.0), (s * 0.8, 0.2), (s * 0.7, 0.32), (s * 0.3, 0.32)], "leather", tension=0.25)
    g.poly([(-0.5, 0.45), (0.5, 0.45), (0.56, 1.75), (-0.56, 1.75)], "greenD")
    g.poly([(-0.34, 0.6), (0.34, 0.6), (0.38, 1.65), (-0.38, 1.65)], "paper", lw=0.8)
    for s in (-1, 1):
        limb(g, [(s * 0.55, 1.65), (s * 0.78, 1.15), (s * 0.18, 0.95)], [0.28, 0.25, 0.22], "skinB")
        for k in range(3):
            g.stroke([(s * 0.85 - 0.06, 1.0 + k * 0.1), (s * 0.85 + 0.06, 1.05 + k * 0.1)], lw=0.6)
    g.circle(0, 0.95, 0.15, "skinB", lw=0.7)
    bar(g, [(-0.42, 0.95), (0.42, 0.95)], 1.6, "wood")
    bar(g, [(-0.12, 0.85), (0.12, 1.05)], 1.4, "wood")
    bar(g, [(0.05, 0.9), (0.18, 0.55), (0.32, 0.4)], 1.4, "wood", smooth=True)
    g.rect(-0.12, 1.72, 0.12, 1.88, "skinB", lw=0.7)
    for s in (-1, 1):
        g.circle(s * 0.3, 2.1, 0.08, "skinB", lw=0.7)
    g.ellipse(0, 2.12, 0.29, 0.34, "skinB")
    g.blob([(-0.29, 2.05), (-0.29, 2.32), (-0.1, 2.5), (0.2, 2.48), (0.3, 2.28), (0.3, 2.06), (0.2, 2.3), (-0.2, 2.3)], "hairred", tension=0.45)
    g.blob([(-0.27, 2.02), (-0.15, 1.82), (0, 1.76), (0.15, 1.82), (0.27, 2.02), (0.12, 1.95), (-0.12, 1.95)], "hairred", tension=0.4, lw=0.7)
    for s in (-1, 1):
        g.circle(s * 0.11, 2.16, 0.035, "ink", lw=0.3)
        g.stroke([(s * 0.05, 2.25), (s * 0.18, 2.27)], lw=0.9)
    g.stroke([(-0.06, 2.0), (0.06, 2.0)], lw=0.8)
    g.rect(0.1, 2.32, 0.26, 2.38, "paper", lw=0.5)


# ---- the Memphis Mule -------------------------------------------------------

def mule(g, rng, legs="stand"):
    """The Memphis Mule, side-on, facing +x, hooves on y=0: stocky, brown,
    long ears, a saddle blanket and two saddlebags, a red bandana with white
    dots at his throat. About 4.5 long and 4.7 to the ear tips. legs:
    'stand' or 'dangle' (hanging under the Fulton balloon)."""
    dang = legs == "dangle"
    # tail
    limb(g, [(-1.62, 2.05), (-1.95, 1.6), (-2.0, 1.15)], [0.2, 0.15, 0.12], "muledark")
    g.blob([(-2.12, 1.25), (-1.88, 1.25), (-1.85, 0.85), (-2.0, 0.65), (-2.15, 0.85)], "hairdark", tension=0.5)

    def leg(pts, w, col):
        limb(g, pts, w, col)
        x, y = pts[-1]
        g.rect(x - 0.2, y - 0.16, x + 0.2, y + 0.08, "hairdark", lw=0.8)

    if dang:
        leg([(-1.05, 1.5), (-1.25, 0.85), (-1.15, 0.4)], [0.55, 0.32, 0.28], "muledark")
        leg([(1.0, 1.4), (1.2, 0.85), (1.3, 0.4)], [0.5, 0.3, 0.28], "muledark")
    else:
        leg([(-1.05, 1.5), (-1.15, 0.8), (-1.05, 0.16)], [0.55, 0.32, 0.28], "muledark")
        leg([(1.0, 1.4), (1.08, 0.8), (1.12, 0.16)], [0.5, 0.3, 0.28], "muledark")
    g.ellipse(-0.05, 1.78, 1.72, 0.75, "mule")
    g.ellipse(0.05, 1.32, 1.05, 0.25, "mulelight", lw=0.7)
    if dang:
        leg([(-1.35, 1.6), (-1.55, 0.95), (-1.5, 0.45)], [0.7, 0.36, 0.3], "mule")
        leg([(0.8, 1.5), (0.95, 0.95), (1.0, 0.45)], [0.55, 0.34, 0.3], "mule")
    else:
        leg([(-1.35, 1.6), (-1.42, 0.85), (-1.3, 0.16)], [0.7, 0.36, 0.3], "mule")
        leg([(0.8, 1.5), (0.76, 0.8), (0.8, 0.16)], [0.55, 0.34, 0.3], "mule")
    # saddle blanket, strap and saddlebags
    g.blob([(-0.95, 2.48), (0.35, 2.5), (0.42, 1.75), (-1.02, 1.72)], "greenB", tension=0.2)
    g.stroke([(-0.98, 1.9), (0.39, 1.92)], lw=1.4, colour="portalorange")
    g.stroke([(-0.98, 2.0), (0.39, 2.02)], lw=1.4, colour="portalblue")
    g.rect(-0.4, 1.05, -0.2, 2.55, "leather", lw=0.7)
    g.blob([(-0.82, 2.2), (0.22, 2.2), (0.25, 1.55), (0.1, 1.38), (-0.68, 1.38), (-0.85, 1.55)], "cardboarddark", tension=0.25)
    g.blob([(-0.86, 2.25), (0.26, 2.25), (0.22, 1.85), (-0.82, 1.85)], "wood", tension=0.2, lw=0.8)
    g.rect(-0.38, 1.7, -0.22, 1.92, "chrome", lw=0.6)
    g.rect(-0.95, 2.45, 0.4, 2.6, "leather", lw=0.7)
    # neck, far ear, head
    g.poly([(0.55, 2.4), (1.38, 3.32), (1.98, 3.02), (1.62, 1.6), (0.9, 1.3)], "mule")
    g.blob([(1.72, 3.5), (1.9, 4.75), (2.12, 4.72), (2.1, 3.45)], "muledark", tension=0.5)
    limb(g, [(0.6, 2.55), (1.0, 3.05), (1.45, 3.42)], [0.22, 0.24, 0.2], "hairdark")
    for k in range(4):
        x = 0.7 + k * 0.22
        g.stroke([(x, 2.7 + k * 0.17), (x - 0.08, 2.92 + k * 0.17)], lw=1.2, colour="hairdark")
    g.blob([(1.32, 3.48), (1.75, 3.7), (2.18, 3.4), (2.8, 2.58), (2.75, 2.2), (2.35, 2.12), (1.75, 2.62), (1.4, 3.0)], "mule", tension=0.4)
    g.blob([(2.25, 2.68), (2.78, 2.6), (2.78, 2.2), (2.35, 2.1), (2.12, 2.35)], "mulelight", tension=0.4)
    g.ellipse(2.62, 2.48, 0.07, 0.1, "hairdark", lw=0.5)
    g.stroke([(2.4, 2.25), (2.72, 2.24)], lw=1.0)
    g.circle(1.98, 3.15, 0.11, "eyewhite", lw=0.7)
    g.circle(2.01, 3.14, 0.06, "hairdark", lw=0.3)
    g.stroke([(1.85, 3.3), (2.1, 3.27)], lw=1.0)
    # near ear, long
    g.blob([(1.38, 3.42), (1.05, 4.6), (1.22, 4.78), (1.48, 4.5), (1.72, 3.6)], "mule", tension=0.5)
    g.blob([(1.42, 3.62), (1.2, 4.45), (1.3, 4.55), (1.55, 3.7)], "mulelight", tension=0.5, lw=0.6)
    # the bandana: a band round the throat and a triangle hanging forward
    g.poly([(1.28, 2.85), (1.48, 3.02), (2.02, 2.62), (1.82, 2.44)], "bandana", lw=0.9)
    g.poly([(1.45, 2.75), (2.0, 2.52), (1.62, 1.82)], "bandana")
    g.circle(1.42, 2.98, 0.11, "bandana", lw=0.8)
    for px, py in ((1.62, 2.5), (1.78, 2.35), (1.66, 2.15), (1.52, 2.68), (1.83, 2.6), (1.68, 2.82)):
        g.circle(px, py, 0.035, "eyewhite", lw=0.3)


def mule_head(g):
    """The Mule's head face-on for the HUD, centred at (4, 4.1) on an 8x8
    canvas: two big ears, a long face with a pale muzzle, a dark forelock
    and the red bandana knotted under his chin."""
    c = 4.0
    for s in (-1, 1):
        ear = [(c + s * 0.5, 5.55), (c + s * 1.45, 7.45), (c + s * 1.95, 7.55), (c + s * 1.85, 6.6), (c + s * 1.35, 5.4)]
        g.blob(ear, "mule", tension=0.45, lw=1.1)
        inner = [(c + s * 0.8, 5.75), (c + s * 1.55, 7.1), (c + s * 1.75, 7.1), (c + s * 1.6, 6.4), (c + s * 1.25, 5.65)]
        g.blob(inner, "mulelight", tension=0.45, lw=0.8)
    face = [(c, 6.25), (c + 1.25, 5.95), (c + 1.55, 5.0), (c + 1.35, 3.6), (c + 1.25, 2.4), (c + 0.85, 1.75), (c, 1.6),
            (c - 0.85, 1.75), (c - 1.25, 2.4), (c - 1.35, 3.6), (c - 1.55, 5.0), (c - 1.25, 5.95)]
    g.blob(face, "mule", tension=0.5, lw=1.2)
    g.blob([(c - 0.2, 6.35), (c + 0.45, 6.05), (c + 0.3, 5.45), (c + 0.05, 5.65), (c - 0.35, 5.35), (c - 0.5, 5.95)], "hairdark", tension=0.5)
    g.ellipse(c, 2.45, 1.18, 0.82, "mulelight", lw=1.1)
    for s in (-1, 1):
        g.ellipse(c + s * 0.48, 2.65, 0.14, 0.22, "muledark", lw=0.8)
        g.circle(c + s * 1.0, 4.55, 0.3, "eyewhite", lw=1.0)
        g.circle(c + s * 0.98, 4.5, 0.18, "muledark", lw=0.6)
        g.circle(c + s * 0.98, 4.5, 0.09, "hairdark", lw=0.4)
        g.circle(c + s * 0.93, 4.58, 0.04, "eyewhite", lw=0.2)
        g.stroke([(c + s * 0.7, 4.98), (c + s * 1.0, 5.08), (c + s * 1.3, 4.95)], lw=1.4, smooth=True)
    g.stroke([(c - 0.45, 2.05), (c, 1.95), (c + 0.45, 2.05)], lw=1.4, smooth=True)
    # bandana
    g.blob([(c - 1.35, 1.95), (c - 0.4, 1.55), (c + 0.4, 1.55), (c + 1.35, 1.95), (c + 1.3, 1.45), (c + 0.4, 1.12), (c - 0.4, 1.12), (c - 1.3, 1.45)], "bandana", tension=0.35, lw=1.1)
    g.poly([(c - 0.75, 1.35), (c + 0.75, 1.35), (c, 0.35)], "bandana", lw=1.1)
    g.poly([(c + 0.95, 1.55), (c + 1.55, 1.05), (c + 1.35, 0.85), (c + 1.0, 1.25)], "bandana", lw=1.0)
    g.circle(c + 0.95, 1.5, 0.18, "bandana", lw=1.0)
    for px, py in ((c - 0.4, 1.1), (c, 0.8), (c + 0.35, 1.15), (c - 0.15, 0.6), (c + 0.15, 1.0), (c - 0.9, 1.6), (c + 0.6, 1.4), (c - 0.6, 1.35)):
        g.circle(px, py, 0.06, "eyewhite", lw=0.5)


# ---- the outpost ------------------------------------------------------------

def sky(g, W, H, bands):
    """Flat sky bands, bottom first: [(colour, from_y), ...]."""
    with g.quiet():
        for col, y in bands:
            g.raw(f"\\fill[{col}] (0,{f(y)}) rectangle ({f(W)},{f(H)});\n", 0)


def cloud(g, x, y, s, fill="cloud"):
    """Puffs, outlined, as the clouds over Sam_Comm_final's city."""
    with g.quiet():
        for dx, dy, r in [(-1.0, 0, 0.55), (-0.45, 0.25, 0.7), (0.2, 0.35, 0.8), (0.85, 0.15, 0.6), (1.35, -0.05, 0.45)]:
            g.circle(x + dx * s, y + dy * s, r * s, fill)
        X0, Y0 = g.pt(x - 1.0 * s, y - 0.15 * s)
        X1, Y1 = g.pt(x + 1.35 * s, y + 0.05 * s)
        g.raw(f"\\fill[{fill}] ({f(X0)},{f(Y0)}) rectangle ({f(X1)},{f(Y1)});\n", X0)
        g.stroke([(x - 1.25 * s, y - 0.15 * s), (x + 1.6 * s, y - 0.15 * s)], lw=1.1)


def band(g, W, y0, y1, fill):
    """A full-width strip (ground, sea), outlined along its top."""
    with g.quiet():
        g.raw(f"\\filldraw[draw=ink, line width={f(g.ink)}pt, fill={fill}] (-0.5,{f(y0)}) rectangle ({f(W + 0.5)},{f(y1)});\n", -0.5)


def tuft(g, x, y, s, fill):
    g.poly([(x - 0.3 * s, y), (x - 0.22 * s, y + 0.55 * s), (x - 0.08 * s, y + 0.2 * s), (x, y + 0.75 * s), (x + 0.1 * s, y + 0.22 * s),
            (x + 0.25 * s, y + 0.6 * s), (x + 0.3 * s, y)], fill, lw=0.8)


def bush(g, rng, x, y, s):
    for dx, dy, r in [(-0.7, 0.35, 0.5), (0.65, 0.35, 0.55), (-0.25, 0.75, 0.62), (0.3, 0.85, 0.6), (0, 0.4, 0.6)]:
        g.circle(x + dx * s, y + dy * s, r * s, rng.choice(["greenA", "greenB", "greenD"]))
    for _ in range(3):
        a = rng.uniform(0.6, 2.5)
        g.stroke([(x + 0.3 * s * math.cos(a), y + 0.6 * s), (x + 0.5 * s * math.cos(a), y + 0.75 * s)], lw=0.8)


def palm(g, x, y, h, s=1.0):
    pts = [(x, y), (x + 0.25 * s, y + h * 0.5), (x + 0.15 * s, y + h)]
    limb(g, pts, [0.32 * s, 0.26 * s, 0.22 * s], "wood")
    for k in range(5):
        g.stroke([(x + 0.05 * s, y + h * (0.15 + k * 0.17)), (x + 0.35 * s, y + h * (0.18 + k * 0.17))], lw=0.8)
    tx, ty = pts[-1]
    for a, L in ((165, 1.9), (130, 1.6), (90, 1.2), (45, 1.7), (10, 2.0), (200, 1.4), (-25, 1.5)):
        r = math.radians(a)
        ex, ey = tx + L * s * math.cos(r), ty + L * s * math.sin(r) - 0.35 * s
        mx, my = tx + 0.55 * L * s * math.cos(r), ty + 0.55 * L * s * math.sin(r) + 0.2 * s
        nx, ny = -math.sin(r) * 0.25 * s, math.cos(r) * 0.25 * s
        g.poly([(tx, ty), (mx + nx, my + ny), (ex, ey), (mx - nx * 0.6, my - ny * 0.6)], "greenB" if a % 2 else "greenD", lw=0.8)
    for dx in (-0.12, 0.12):
        g.circle(tx + dx * s, ty - 0.1 * s, 0.13 * s, "wood", lw=0.7)


def sandbags(g, x, y, cols, rows, s=1.0):
    for j in range(rows):
        off = 0.3 * s if j % 2 else 0
        for i in range(cols - (j % 2)):
            bx = x + off + i * 0.62 * s
            by = y + j * 0.3 * s
            g.blob([(bx, by), (bx + 0.6 * s, by), (bx + 0.66 * s, by + 0.17 * s), (bx + 0.6 * s, by + 0.35 * s), (bx, by + 0.35 * s), (bx - 0.06 * s, by + 0.17 * s)],
                   "khaki", tension=0.4, lw=0.8)
            g.stroke([(bx + 0.48 * s, by + 0.06 * s), (bx + 0.48 * s, by + 0.29 * s)], lw=0.6, colour="khakidark")


def box(g, x, y, w, h, d=0.35):
    """A cardboard box, a little 3/4: front, top, taped seam."""
    g.rect(x, y, x + w, y + h, "cardboard")
    g.poly([(x, y + h), (x + w, y + h), (x + w + d * 0.8, y + h + d), (x + d * 0.8, y + h + d)], "stone")
    g.poly([(x + w, y), (x + w + d * 0.8, y + d), (x + w + d * 0.8, y + h + d), (x + w, y + h)], "cardboarddark")
    g.rect(x + w * 0.42, y + h * 0.55, x + w * 0.58, y + h, "paper", lw=0.6)
    g.poly([(x + w * 0.42 + d * 0.0, y + h), (x + w * 0.58, y + h), (x + w * 0.58 + d * 0.8, y + h + d), (x + w * 0.42 + d * 0.8, y + h + d)], "paper", lw=0.6)
    g.stroke([(x + w * 0.12, y + h * 0.25), (x + w * 0.3, y + h * 0.25)], lw=0.8)
    g.poly([(x + w * 0.7, y + h * 0.18), (x + w * 0.86, y + h * 0.18), (x + w * 0.78, y + h * 0.4)], "cardboarddark", lw=0.6)


def barrel(g, x, y, s=1.0, col="olive"):
    g.rect(x - 0.42 * s, y, x + 0.42 * s, y + 1.2 * s, col)
    for yy in (0.3, 0.9):
        g.stroke([(x - 0.42 * s, y + yy * s), (x + 0.42 * s, y + yy * s)], lw=1.0)
    g.ellipse(x, y + 1.2 * s, 0.42 * s, 0.1 * s, "olivedark" if col == "olive" else "metaldark", lw=0.8)


def razor_wire(g, x0, x1, y):
    """Posts leaning out with coiled wire along the wall top."""
    n = int((x1 - x0) / 1.7) + 1
    for i in range(n):
        px = x0 + 0.2 + i * 1.7
        g.stroke([(px, y), (px, y + 0.55), (px - 0.2, y + 0.7)], lw=1.6, colour="metaldark")
    x = x0 + 0.05
    while x < x1 - 0.2:
        g.raw(f"\\draw[metaldark, line width=0.9pt] {g.P(x + 0.18, y + 0.4)} ellipse ({f(0.2 * g.k())} and {f(0.28 * g.k())});\n", g.pt(x, 0)[0])
        x += 0.24


def concrete_wall(g, rng, x0, x1, y0, h):
    """The outpost wall: poured panels with seams and stains, a coping on
    top, razor wire above."""
    with g.part("wall"):
        g.rect(x0, y0, x1, y0 + h, "concrete")
        x = x0 + 1.8
        while x < x1 - 0.2:
            g.stroke([(x, y0), (x, y0 + h)], lw=1.0)
            for yy in (0.6, h - 0.6):
                g.circle(x - 0.25, y0 + yy, 0.05, "concretedark", lw=0.4)
                g.circle(x + 0.25, y0 + yy, 0.05, "concretedark", lw=0.4)
            if rng.random() < 0.5:
                sx = x + rng.uniform(0.3, 1.0)
                g.poly([(sx, y0 + h - 0.25), (sx + 0.35, y0 + h - 0.25), (sx + 0.25, y0 + h - rng.uniform(0.9, 1.8)), (sx + 0.1, y0 + h - rng.uniform(0.6, 1.4))], "concretedark", lw=0.5)
            x += 1.8
        g.rect(x0 - 0.1, y0 + h, x1 + 0.1, y0 + h + 0.3, "concretedark")
        razor_wire(g, x0, x1, y0 + h + 0.3)


def guard_tower(g, x, y0, inside=None, light=True):
    """A concrete watchtower: a tapering shaft with a slit, a cabin with a
    wide window (whatever `inside` draws stands in it), a flat roof with a
    searchlight. Shaft x..x+3; the cabin floor is at y0+7.4."""
    top = y0 + 7.4
    with g.part("guard tower"):
        g.poly([(x - 0.2, y0), (x + 3.2, y0), (x + 3.0, top), (x, top)], "concrete")
        for k in range(1, 5):
            yy = y0 + k * 1.5
            g.stroke([(x - 0.2 + 0.03 * k, yy), (x + 3.2 - 0.03 * k, yy)], lw=0.7, colour="concretedark")
        g.rect(x + 1.3, y0 + 5.6, x + 1.7, y0 + 6.6, "windark", lw=0.8)
        g.rect(x + 1.2, y0, x + 1.9, y0 + 1.6, "metaldark")
        g.circle(x + 1.75, y0 + 0.8, 0.06, "chrome", lw=0.4)
        g.rect(x - 0.7, top, x + 3.7, top + 0.4, "concretedark")
        g.rect(x - 0.4, top + 0.4, x + 3.4, top + 3.2, "windark")
        if inside:
            inside(x + 1.5, top + 0.4)
        g.rect(x - 0.4, top + 0.4, x + 3.4, top + 1.3, "concrete")
        for px in (x - 0.4, x + 3.15):
            g.rect(px, top + 1.3, px + 0.25, top + 3.2, "concrete", lw=0.9)
        g.rect(x - 0.75, top + 3.2, x + 3.75, top + 3.7, "wallE")
        g.stroke([(x + 3.3, top + 3.7), (x + 3.3, top + 5.2)], lw=1.2)
        for k in range(1, 3):
            g.stroke([(x + 3.1, top + 3.7 + k * 0.5), (x + 3.5, top + 3.7 + k * 0.5)], lw=0.8)
        g.circle(x + 3.3, top + 5.25, 0.1, "bandana", lw=0.6)
        if light:
            g.rect(x + 0.6, top + 3.7, x + 0.8, top + 4.1, "metaldark", lw=0.7)
            with g.frame(x + 0.7, top + 4.35, 1.0, -20):
                g.poly([(-0.45, -0.32), (0.25, -0.38), (0.45, 0.4), (-0.45, 0.32)], "metal")
                g.ellipse(0.38, 0.01, 0.12, 0.42, "lamp", lw=0.8)
    return top


def lattice_tower(g, x, y0, y1, w0, w1, band=False, dishes=True, rng=None):
    """A comms tower: two legs tapering from w0 wide at y0 to w1 at y1,
    struts and cross-braces, red and white bands, dishes and antennas."""
    with g.part("comms tower"):
        n = max(3, int((y1 - y0) / 1.0))

        def lx(t, s):
            return x + s * (w0 + (w1 - w0) * t) / 2
        for i in range(n):
            t0, t1 = i / n, (i + 1) / n
            ya, yb = y0 + (y1 - y0) * t0, y0 + (y1 - y0) * t1
            bar(g, [(lx(t0, -1), ya), (lx(t1, 1), yb)], 0.9, "metal")
            bar(g, [(lx(t0, 1), ya), (lx(t1, -1), yb)], 0.9, "metal")
            bar(g, [(lx(t1, -1), yb), (lx(t1, 1), yb)], 1.2, "metal")
        for s in (-1, 1):
            for i in range(n):
                t0, t1 = i / n, (i + 1) / n
                col = ("bandana" if i % 2 else "paper") if band else "metaldark"
                bar(g, [(lx(t0, s), y0 + (y1 - y0) * t0), (lx(t1, s), y0 + (y1 - y0) * t1)], 2.6, col)
        g.stroke([(x, y1), (x, y1 + 2.2)], lw=1.6)
        for k in range(3):
            g.stroke([(x - 0.3 + k * 0.05, y1 + 0.6 + k * 0.5), (x + 0.3 - k * 0.05, y1 + 0.6 + k * 0.5)], lw=1.0)
        g.circle(x, y1 + 2.3, 0.14, "bandana", lw=0.7)
        if dishes:
            for (t, s, r) in ((0.78, 1, 0.55), (0.62, -1, 0.45), (0.9, -1, 0.35)):
                px, py = lx(t, s), y0 + (y1 - y0) * t
                with g.frame(px + s * 0.3, py, 1.0, 0, sx=s):
                    g.stroke([(-0.3, 0), (0.15, 0)], lw=1.2)
                    g.ellipse(0.25, 0, 0.16, r, "paper", lw=0.9)
                    g.stroke([(0.27, 0), (0.6, 0)], lw=0.9)
                    g.circle(0.62, 0, 0.05, "metaldark", lw=0.4)


def warehouse(g, x, y0, w, h, fill="wallA", door=True):
    """A quonset warehouse end-on: a rounded corrugated roof, a roll door."""
    with g.part("warehouse"):
        g.rect(x, y0, x + w, y0 + h, fill)
        pts = [(x - 0.15, y0 + h)] + [(x + w / 2 - (w / 2 + 0.15) * math.cos(math.pi * i / 12), y0 + h + 1.2 * math.sin(math.pi * i / 12)) for i in range(13)]
        g.poly(pts, "wallE")
        for i in range(1, 6):
            xx = x + w * i / 6
            g.stroke([(xx, y0 + h + 0.05), (xx, y0 + h + 1.2 * math.sqrt(max(0, 1 - ((xx - x - w / 2) / (w / 2 + 0.15)) ** 2)) - 0.08)], lw=0.7)
        for i in range(1, int(w / 0.45)):
            g.stroke([(x + i * 0.45, y0 + 0.05), (x + i * 0.45, y0 + h - 0.05)], lw=0.5, colour="wallE")
        if door:
            g.rect(x + w * 0.25, y0, x + w * 0.75, y0 + h * 0.72, "metal")
            for k in range(1, 6):
                yy = y0 + h * 0.72 * k / 6
                g.stroke([(x + w * 0.25, yy), (x + w * 0.75, yy)], lw=0.6)
        g.rect(x + w * 0.1, y0 + h * 0.75, x + w * 0.2, y0 + h * 0.9, "lamp", lw=0.7)


def helicopter(g):
    """The extraction helicopter side-on, nose +x, skids on y=-0.35: a
    rounded cabin with the door open, a tail boom and fin, a tail rotor, a
    main rotor blurred into a flat disc."""
    g.poly([(-4.3, 0.75), (-0.6, 0.6), (-0.6, 1.4), (-4.3, 1.0)], "heli")
    g.poly([(-4.45, 0.8), (-3.85, 0.88), (-4.0, 2.05), (-4.5, 2.1)], "helidark")
    g.circle(-4.15, 1.45, 0.62, "chrome", opts=", fill opacity=0.35", lw=0.6)
    g.stroke([(-4.15, 0.88), (-4.15, 2.02)], lw=1.4)
    g.stroke([(-4.7, 1.45), (-3.6, 1.45)], lw=1.4)
    g.circle(-4.15, 1.45, 0.08, "metaldark", lw=0.5)
    for sx_, sx2 in ((-0.55, -0.75), (1.45, 1.65)):
        g.stroke([(sx_, 0.35), (sx2, -0.3)], lw=2.2, colour="metaldark")
    limb(g, [(-1.5, -0.35), (2.5, -0.35), (2.75, -0.2)], 0.14, "metaldark", lw=0.7)
    body = [(-1.25, 0.3), (1.8, 0.25), (2.6, 0.55), (2.95, 1.25), (2.35, 2.0), (0.4, 2.25), (-1.0, 2.05), (-1.45, 1.25)]
    g.blob(body, "heli", tension=0.45)
    g.blob([(1.5, 1.35), (2.0, 2.0), (2.45, 1.92), (2.88, 1.25), (2.6, 0.95), (1.7, 0.98)], "glass", tension=0.35)
    g.stroke([(2.0, 1.95), (2.15, 1.0)], lw=0.9)
    g.stroke([(2.3, 1.75), (2.55, 1.35)], lw=1.4, colour="white", opts=", opacity=0.75")
    g.rect(-0.45, 0.62, 0.95, 1.85, "leather")
    g.stroke([(-1.3, 0.75), (2.6, 0.72)], lw=2.2, colour="portalorange")
    g.stroke([(-1.4, 0.95), (-0.45, 0.95)], lw=2.2, colour="portalblue")
    g.rect(-0.65, 2.15, 1.1, 2.6, "helidark")
    g.rect(0.2, 2.6, 0.38, 3.0, "metaldark", lw=0.8)
    g.ellipse(0.3, 3.05, 4.9, 0.42, "chrome", opts=", fill opacity=0.3", lw=0.6)
    g.blob([(-4.6, 3.0), (5.2, 3.02), (5.2, 3.1), (-4.6, 3.12)], "metaldark", tension=0.1, lw=0.8)
    g.circle(0.3, 3.06, 0.14, "metal", lw=0.6)


def fulton(g, x, y, s=1.0):
    """The Fulton balloon: a white balloon with seams, its line hanging
    from (x, y) the bottom of the balloon."""
    with g.frame(x, y, s):
        g.blob([(0, 0), (0.8, 0.55), (1.15, 1.6), (0.75, 2.6), (0, 2.85), (-0.75, 2.6), (-1.15, 1.6), (-0.8, 0.55)], "balloon", tension=0.55)
        for dx in (-0.5, 0.5):
            g.stroke([(0, 0.05), (dx, 1.5), (0, 2.82)], lw=0.8, smooth=True)
        g.stroke([(-0.9, 2.0), (-0.7, 2.35)], lw=2.0, colour="white")
        g.circle(0, 0, 0.1, "metaldark", lw=0.5)


# ---- pictures --------------------------------------------------------------

def title():
    """The outpost wall on a warm coast: a guard in the watchtower, his
    sight cone a pale fan swept down onto the grass just short of the
    operative crouched there with the silenced pistol; the Memphis Mule
    beside her behind a bush; a blue portal on the wall and an orange one on
    the tower. Behind the wall, warehouses, a palm and the comms tower.
    The left third is sky, sea and grass only."""
    W, H = 32.0, 18.0
    rng = random.Random(2026)
    g = Fig(ink=1.1, calm_x=W / 3)
    sky(g, W, H, [("skylow", 0), ("sky", 9.0), ("skyhi", 14.4)])
    for cx, cy, cs in [(3.2, 15.4, 1.1), (8.0, 12.6, 0.7), (16.5, 16.2, 0.95), (30.2, 13.3, 0.8)]:
        cloud(g, cx, cy, cs)
    band(g, W, 5.2, 6.0, "sea")
    with g.quiet():
        for wx, wy in ((2.0, 5.6), (5.8, 5.45), (8.6, 5.75)):
            g.stroke([(wx, wy), (wx + 0.6, wy)], lw=0.9, colour="seahi")
    band(g, W, -0.5, 5.2, "greenC")
    with g.part("behind the wall"):
        lattice_tower(g, 22.2, 8.0, 16.6, 2.6, 0.5, band=True)
        warehouse(g, 16.8, 8.0, 3.6, 1.2, "wallA", door=False)
        warehouse(g, 26.8, 8.0, 3.0, 1.0, "wallB", door=False)
        palm(g, 20.5, 8.5, 3.0, 0.9)
        palm(g, 31.2, 8.5, 2.6, 0.8)
    concrete_wall(g, rng, 11.6, 32.6, 4.8, 4.2)
    with g.part("wall corner"):
        g.rect(11.2, 4.6, 12.0, 9.6, "concretedark")
        g.rect(11.0, 9.6, 12.2, 9.9, "concrete")
    with g.part("blue portal"):
        portal(g, 16.3, 7.2, 0.85, 1.45, "blue")
    with g.part("dirt strip"):
        g.poly([(11.0, 4.6), (32.6, 4.6), (32.6, 4.0), (11.4, 4.2)], "ground", lw=0.8)

    def lookout(cx, cy):
        with g.frame(cx + 0.2, cy, 0.75, sx=-1):
            soldier_side(g, rng, elite=True)
    top = guard_tower(g, 26.8, 3.9, inside=lookout)
    with g.part("orange portal"):
        portal(g, 28.3, 7.4, 0.85, 1.45, "orange")
    with g.part("wall clutter"):
        sandbags(g, 23.2, 4.3, 5, 3, 1.0)
        box(g, 19.4, 4.2, 1.1, 0.85)
        box(g, 20.6, 4.2, 0.8, 0.65)
        barrel(g, 25.3, 4.25, 0.85)
    # the sight cone, from the lookout's goggles down onto the grass
    with g.part("sight cone"):
        eye = top + 0.4 + 0.75 * 3.27
        ex = 26.8 + 1.5 + 0.2 - 0.75 * 0.36
        far = [(20.6 + 5.6 * t, 2.7 + 1.2 * t - 0.35 * math.sin(math.pi * t)) for t in [i / 8 for i in range(9)]]
        g.poly([(ex, eye)] + far, "beam", opts=", fill opacity=0.42, draw opacity=0.6", lw=0.9)
    # the foreground: the Mule behind a bush, the operative crouched in the grass
    with g.part("mule"):
        with g.frame(13.2, 1.0, 1.05):
            mule(g, rng)
    with g.part("bushes"):
        bush(g, rng, 12.6, 0.4, 1.0)
        bush(g, rng, 15.1, 0.2, 0.9)
    with g.part("operative"):
        with g.frame(17.7, 0.75, 1.3):
            operative(g, rng, "crouch")
    with g.part("grass"):
        for k in range(26):
            tx = 11.6 + k * 0.8 + rng.uniform(-0.2, 0.2)
            tuft(g, tx, rng.uniform(-0.1, 0.9), rng.uniform(0.8, 1.3), rng.choice(["greenA", "greenB", "greenD"]))
    return g, "scene"


def failed():
    """Mission failed, from the dirt at dusk: the operative lies face down
    in the searchlight's pool, her silenced pistol dropped out of reach and
    her cardboard box knocked over; a guard's boots stand over her at the
    right edge and another guard has the alarm burst over his head. Nothing
    gory. The left third is sky and dirt only."""
    W, H = 32.0, 18.0
    rng = random.Random(404)
    g = Fig(ink=1.1, calm_x=W / 3)
    sky(g, W, H, [("duskC", 0), ("duskB", 9.0), ("duskA", 13.5)])
    for cx, cy, cs in [(4.0, 15.0, 1.0), (9.2, 12.0, 0.7), (18.0, 16.0, 0.9)]:
        cloud(g, cx, cy, cs, fill="duskB")
    band(g, W, -0.5, 5.6, "dirt")
    with g.part("the outpost"):
        lattice_tower(g, 30.0, 8.6, 16.4, 2.4, 0.5, band=True)
        concrete_wall(g, rng, 11.8, 32.6, 5.6, 3.0)
        g.rect(11.4, 5.4, 12.2, 9.2, "concretedark")
    top = guard_tower(g, 19.6, 5.4, inside=None, light=False)
    with g.part("searchlight"):
        lx, ly = 22.2, top + 4.2
        g.rect(lx - 0.1, top + 3.7, lx + 0.1, ly - 0.3, "metaldark", lw=0.7)
        pool = [(18.3 + 5.4 * math.cos(2 * math.pi * i / 24), 1.9 + 1.25 * math.sin(2 * math.pi * i / 24)) for i in range(24)]
        g.poly([(lx - 0.35, ly - 0.25), (12.9, 1.75), (18.3, 0.65), (23.7, 1.75), (lx + 0.35, ly - 0.25)], "beam", opts=", fill opacity=0.38, draw opacity=0.55", lw=0.9)
        g.blob(pool, "beam", tension=0.55, opts=", fill opacity=0.55, draw opacity=0.6", lw=0.8)
        with g.frame(lx, ly, 1.0, 240):
            g.poly([(-0.5, -0.38), (0.3, -0.45), (0.5, 0.45), (-0.5, 0.38)], "metal")
            g.ellipse(0.42, 0.0, 0.14, 0.48, "lamp", lw=0.8)
    with g.part("ruts"):
        for pts in ([(12.0, 4.6), (16.0, 4.1), (22.0, 4.4)], [(13.0, 3.3), (17.5, 2.95)], [(24.5, 3.8), (28.5, 3.1)]):
            g.stroke(pts, lw=1.1, colour="dirtdark", smooth=True)
    with g.part("the alarmed guard"):
        with g.frame(25.4, 3.0, 1.2):
            soldier_front(g, rng, "rifle", "helmet")
        alarm(g, 25.4, 7.75, 0.9)
    with g.part("the box"):
        with g.frame(13.9, 2.3, 1.0, 168):
            box(g, -0.8, -0.6, 1.6, 1.15)
    with g.part("the operative"):
        with g.frame(17.6, 1.2, 1.3):
            operative(g, rng, "prone")
    with g.part("the dropped pistol"):
        with g.frame(20.8, 1.2, 1.25, 158):
            pistol(g)
    with g.part("the boots"):
        boots_closeup(g, 30.6, 1.05)
    with g.part("pebbles"):
        for _ in range(16):
            px, py = rng.uniform(12.0, 26.0), rng.uniform(-0.2, 4.6)
            g.ellipse(px, py, rng.uniform(0.08, 0.18), rng.uniform(0.05, 0.1), "dirtdark", lw=0.6)
    return g, "scene"


def complete():
    """Extraction at sundown: the helicopter lifts off the helipad, dust
    blowing; the Memphis Mule rides up under a Fulton balloon in his
    harness; the outpost sits on its headland behind across the water. The
    left third is sky and sea only."""
    W, H = 32.0, 18.0
    rng = random.Random(77)
    g = Fig(ink=1.1, calm_x=W / 3)
    sky(g, W, H, [("sunC", 0), ("sunB", 6.6), ("sunA", 9.2), ("sky", 12.5), ("skyhi", 15.4)])
    for cx, cy, cs in [(3.6, 14.6, 1.1), (8.8, 16.6, 0.7), (27.8, 15.4, 1.2)]:
        cloud(g, cx, cy, cs, fill="sunA")
    with g.quiet():
        g.circle(17.6, 6.0, 2.2, "sundisc")
    band(g, W, -0.5, 6.0, "seadusk")
    with g.quiet():
        for k, (y, w) in enumerate(((5.6, 2.4), (5.1, 1.8), (4.6, 1.3), (4.1, 0.8))):
            g.stroke([(17.6 - w / 2, y), (17.6 + w / 2, y)], lw=1.6, colour="sundisc")
    with g.part("headland"):
        g.blob([(22.5, 5.9), (24.5, 6.9), (28.0, 7.3), (32.8, 7.2), (32.8, 5.9)], "greenD", tension=0.3)
        g.rect(24.6, 6.9, 32.6, 8.2, "wallE", lw=0.9)
        for x in range(25, 33, 1):
            g.stroke([(x + 0.4, 6.95), (x + 0.4, 8.15)], lw=0.7)
        razor_wire(g, 24.6, 32.6, 8.2)
        g.rect(28.4, 6.9, 29.4, 10.8, "wallE", lw=0.9)
        g.rect(28.0, 10.8, 29.8, 11.9, "wallE", lw=0.9)
        g.rect(28.25, 11.0, 29.55, 11.6, "windark", lw=0.7)
        g.rect(27.9, 11.9, 29.9, 12.2, "shingle", lw=0.8)
    lattice_tower(g, 31.2, 8.2, 13.6, 1.4, 0.3, band=True, dishes=False)
    with g.part("helipad deck"):
        g.poly([(12.0, -0.5), (32.6, -0.5), (32.6, 2.4), (12.4, 2.4)], "concretedark")
        for x in (14.5, 18.5, 22.5, 26.5, 30.5):
            g.rect(x - 0.2, -0.5, x + 0.2, 2.0, "concrete", lw=0.8)
        g.poly([(12.4, 2.4), (32.6, 2.4), (32.6, 4.6), (14.2, 4.6)], "concrete")
        g.ellipse(22.8, 3.45, 5.0, 0.85, "concretedark", lw=0.9)
        g.ellipse(22.8, 3.45, 4.2, 0.68, "lamp", lw=0.9)
        g.ellipse(22.8, 3.45, 3.6, 0.56, "concretedark", lw=0.7)
        for k in range(12):
            a_ = 2 * math.pi * k / 12
            g.circle(22.8 + 5.0 * math.cos(a_), 3.45 + 0.85 * math.sin(a_), 0.1, "lamp", lw=0.5)
        g.ellipse(22.4, 3.45, 3.2, 0.36, "ink", opts=", fill opacity=0.25, draw opacity=0.3", lw=0.5)
        sandbags(g, 14.6, 3.4, 4, 2, 0.9)
        box(g, 30.0, 3.3, 1.1, 0.9)
    with g.part("dust"):
        for (px, py, r) in [(17.4, 3.4, 0.55), (18.4, 3.7, 0.45), (27.6, 3.4, 0.55), (26.6, 3.7, 0.45)]:
            for dx, dy, kk in ((-0.6, 0, 0.7), (0, 0.3, 0.9), (0.6, 0.05, 0.75)):
                g.circle(px + dx * r, py + dy * r, kk * r, "smoke")
        for s in (-1, 1):
            for k in range(3):
                g.stroke([(22.8 + s * (5.3 + k * 0.6), 3.9 + k * 0.4), (22.8 + s * (6.5 + k * 0.6), 3.7 + k * 0.4)], lw=1.3)
    with g.part("helicopter"):
        with g.frame(22.0, 5.3, 1.25, -4):
            helicopter(g)
        for k in range(5):
            g.stroke([(19.8 + k * 1.1, 4.65), (19.6 + k * 1.1, 3.95)], lw=1.0, opts=", dash pattern=on 3pt off 3pt")
    with g.part("fulton"):
        bx, by = 13.6, 14.4
        fulton(g, bx, by, 1.0)
        mx, my = 13.9, 9.3
        g.stroke([(bx, by), (mx, my + 3.0)], lw=1.4, colour="metaldark")
        with g.frame(mx, my, 0.85, 8):
            mule(g, rng, "dangle")
            harness = [g.pt(-0.72, 2.62), g.pt(0.58, 2.62)]
            for px in (-0.75, 0.55):
                limb(g, [(px, 1.02), (px + 0.03, 2.62)], 0.1, "portalorange", lw=0.6)
        top_hook = (mx + 0.1, my + 3.0)
        for hx, hy in harness:
            g.stroke([(hx, hy), top_hook], lw=1.1, colour="metaldark")
        g.circle(*top_hook, 0.1, "chrome", lw=0.6)
        for k in range(3):
            g.stroke([(12.2 + k * 0.45, 7.6 - k * 0.15), (12.15 + k * 0.45, 6.9 - k * 0.15)], lw=1.2)
    return g, "scene"


def graceland():
    """Graceland, the home base: an offshore platform on stilts in a calm
    sea under a sunny sky, a red ranch stable with a white-trimmed door, a
    fence, hay bales, a water tank and palm; recruited soldiers in their
    old uniforms wave from the deck and the Memphis Mule stands at his
    stable. The left third is sky and sea only."""
    W, H = 32.0, 18.0
    rng = random.Random(1956)
    g = Fig(ink=1.1, calm_x=W / 3)
    sky(g, W, H, [("skylow", 0), ("sky", 8.5), ("skyhi", 14.6)])
    with g.quiet():
        g.circle(5.2, 14.2, 1.3, "sundisc")
    for cx, cy, cs in [(2.8, 11.8, 0.8), (8.8, 16.2, 0.9), (19.0, 16.4, 1.1), (29.5, 15.8, 0.9)]:
        cloud(g, cx, cy, cs)
    band(g, W, -0.5, 6.4, "sea")
    with g.quiet():
        for wx, wy in ((1.5, 4.8), (4.2, 2.6), (7.8, 5.2), (2.8, 1.2), (8.2, 0.8)):
            g.stroke([(wx, wy), (wx + 0.35, wy + 0.12), (wx + 0.7, wy)], lw=1.0, colour="seahi", smooth=True)
    with g.part("platform legs"):
        for x in (12.6, 16.6, 20.6, 24.6, 28.6, 32.2):
            g.rect(x - 0.35, -0.5, x + 0.35, 3.0, "metal")
            g.ellipse(x, 0.6, 0.75, 0.16, "seahi", lw=0.7)
        for x0 in (12.6, 16.6, 20.6, 24.6, 28.6):
            bar(g, [(x0 + 0.3, 0.9), (x0 + 3.7, 2.7)], 1.2, "metaldark")
            bar(g, [(x0 + 0.3, 2.7), (x0 + 3.7, 0.9)], 1.2, "metaldark")
    with g.part("deck"):
        g.rect(11.6, 3.0, 32.6, 3.8, "wallE")
        for x in range(12, 33, 1):
            g.stroke([(x + 0.0, 3.05), (x + 0.0, 3.75)], lw=0.6)
    with g.part("stable"):
        x0, x1, yb, yt = 21.5, 30.5, 3.8, 8.6
        g.rect(x0, yb, x1, yt, "barn")
        for k in range(1, 22):
            g.stroke([(x0 + k * (x1 - x0) / 22, yb + 0.05), (x0 + k * (x1 - x0) / 22, yt - 0.05)], lw=0.5, colour="wallE")
        g.poly([(x0 - 0.5, yt), (x1 + 0.5, yt), ((x0 + x1) / 2 + 2.6, yt + 1.6), ((x0 + x1) / 2, yt + 2.8), ((x0 + x1) / 2 - 2.6, yt + 1.6)], "wallE")
        g.poly([(x0 - 0.5, yt), (x1 + 0.5, yt), (x1 + 0.5, yt - 0.25), (x0 - 0.5, yt - 0.25)], "trim")
        cx = (x0 + x1) / 2
        g.rect(cx - 0.75, yt + 0.6, cx + 0.75, yt + 1.8, "trim")
        g.rect(cx - 0.55, yt + 0.75, cx + 0.55, yt + 1.65, "windark", lw=0.8)
        g.rect(cx - 1.8, yb, cx + 1.8, yb + 3.4, "trim")
        for s in (-1, 1):
            dx0, dx1 = cx + (s - 1) * 0.83 + (0 if s < 0 else 0.0), cx + (s + 1) * 0.83
            a, b = min(cx, cx + s * 1.65), max(cx, cx + s * 1.65)
            g.rect(a, yb + 0.1, b, yb + 3.25, "barn", lw=0.9)
            g.stroke([(a, yb + 0.1), (b, yb + 3.25)], lw=2.6, colour="trim")
            g.stroke([(a, yb + 3.25), (b, yb + 0.1)], lw=2.6, colour="trim")
            g.stroke([(a, yb + 1.68), (b, yb + 1.68)], lw=2.6, colour="trim")
        for wx in (x0 + 0.8, x1 - 2.2):
            g.rect(wx, yb + 1.9, wx + 1.4, yb + 3.1, "trim")
            g.rect(wx + 0.15, yb + 2.05, wx + 1.25, yb + 2.95, "window", lw=0.8)
            g.stroke([(wx + 0.7, yb + 2.05), (wx + 0.7, yb + 2.95)], lw=0.8)
        # the weathervane: a little mule
        g.stroke([(cx, yt + 2.8), (cx, yt + 3.9)], lw=1.2)
        with g.frame(cx - 0.5, yt + 3.6, 0.22):
            g.ellipse(0, 1.6, 1.4, 0.55, "hairdark", lw=0.7)
            g.poly([(1.0, 1.8), (2.1, 2.6), (2.4, 2.1), (1.4, 1.3)], "hairdark", lw=0.7)
            g.poly([(1.7, 2.5), (1.6, 3.5), (1.9, 2.6)], "hairdark", lw=0.7)
            for lx in (-1.0, 1.0):
                g.rect(lx - 0.12, 0.4, lx + 0.12, 1.4, "hairdark", lw=0.6)
    with g.part("tank and palm"):
        g.rect(31.0, 3.8, 31.15, 8.6, "metaldark", lw=0.7)
        palm(g, 20.6, 3.8, 4.6, 1.0)
        g.rect(29.8, 8.6, 32.6, 10.6, "metal")
        g.ellipse(31.2, 10.6, 1.4, 0.25, "chrome", lw=0.8)
        for yy in (9.2, 10.0):
            g.stroke([(29.8, yy), (32.6, yy)], lw=0.8)
    with g.part("flag"):
        g.stroke([(12.4, 3.8), (12.4, 11.5)], lw=1.6, colour="metaldark")
        g.blob([(12.45, 11.4), (13.4, 11.55), (14.4, 11.3), (14.35, 10.2), (13.4, 10.4), (12.45, 10.25)], "portalorange", tension=0.3)
        g.blob([(12.45, 10.85), (13.4, 11.0), (14.38, 10.75), (14.36, 10.25), (13.4, 10.42), (12.45, 10.27)], "portalblue", tension=0.3, lw=0.8)
        g.circle(12.4, 11.6, 0.12, "gold", lw=0.6)
    with g.part("hay"):
        for (hx, hy, r) in ((28.6, 3.8, 0.6), (29.9, 3.8, 0.6), (29.25, 4.85, 0.6)):
            g.rect(hx - 0.6, hy, hx + 0.6, hy + 1.0, "hay", lw=0.9)
            for k in range(1, 4):
                g.stroke([(hx - 0.6, hy + k * 0.25), (hx + 0.6, hy + k * 0.25)], lw=0.5, colour="haydark")
            g.stroke([(hx - 0.25, hy), (hx - 0.25, hy + 1.0)], lw=1.0, colour="wallE")
            g.stroke([(hx + 0.25, hy), (hx + 0.25, hy + 1.0)], lw=1.0, colour="wallE")
    with g.part("fence"):
        g.rect(11.8, 4.75, 21.4, 4.95, "trim", lw=0.8)
        g.rect(11.8, 5.45, 21.4, 5.65, "trim", lw=0.8)
        for x in (11.9, 14.3, 16.7, 19.1, 21.2):
            g.rect(x - 0.12, 3.8, x + 0.12, 5.95, "trim", lw=0.8)
    with g.part("the mule"):
        with g.frame(25.6, 3.85, 1.1, sx=-1):
            mule(g, rng)
    with g.part("recruits"):
        for x, pose, hat, sc in ((14.4, "wave", "helmet", 1.15), (17.0, "wave2", "cap", 1.2), (19.5, "wave", "bandage", 1.12)):
            with g.frame(x, 3.85, sc):
                soldier_front(g, rng, pose, hat)
    with g.part("life ring"):
        g.circle(13.4, 1.5, 0.55, "bandana")
        g.circle(13.4, 1.5, 0.28, "concretedark")
        for a in (45, 135, 225, 315):
            r = math.radians(a)
            g.stroke([(13.4 + 0.3 * math.cos(r), 1.5 + 0.3 * math.sin(r)), (13.4 + 0.53 * math.cos(r), 1.5 + 0.53 * math.sin(r))], lw=3.0, colour="paper")
    return g, "scene"


def mission1():
    """Mission one: inside a warehouse, a captured commando sits tied in a
    cage under a hanging lamp, among crates, boxes and barrels."""
    W, H = 16.0, 9.0
    rng = random.Random(1)
    g = Fig(ink=1.0)
    g.rect(-0.2, -0.2, W + 0.2, H + 0.2, "wallA")
    for k in range(1, 40):
        g.stroke([(k * 0.4, 2.2), (k * 0.4, 7.2)], lw=0.5, colour="wallE")
    g.rect(-0.2, 7.2, W + 0.2, H + 0.2, "wallE")
    for x in (2.0, 8.0, 14.0):
        bar(g, [(x - 3.0, 7.2), (x, 8.9), (x + 3.0, 7.2)], 2.2, "metaldark")
        bar(g, [(x, 7.2), (x, 8.9)], 1.6, "metaldark")
    for wx in (1.2, 12.2):
        g.rect(wx, 5.2, wx + 2.6, 6.6, "metaldark")
        g.rect(wx + 0.15, 5.35, wx + 2.45, 6.45, "sky", lw=0.8)
        for k in (1, 2):
            g.stroke([(wx + k * 0.87, 5.35), (wx + k * 0.87, 6.45)], lw=1.0)
    g.rect(-0.2, -0.2, W + 0.2, 2.2, "paving")
    for x in (-2, 2, 6, 10, 14, 18):
        g.stroke([(x, 0), (x + 1.0, 2.2)], lw=0.6, colour="ground")
    # crates and boxes, left; barrels, right
    g.rect(0.4, 2.0, 3.0, 4.2, "wood")
    g.stroke([(0.4, 2.0), (3.0, 4.2)], lw=1.4)
    g.stroke([(0.4, 4.2), (3.0, 2.0)], lw=1.4)
    g.rect(0.6, 4.2, 2.5, 5.4, "wood")
    g.stroke([(0.6, 4.8), (2.5, 4.8)], lw=1.0)
    box(g, 3.2, 1.6, 1.4, 1.1)
    box(g, 3.4, 2.7, 1.0, 0.8)
    barrel(g, 13.0, 1.7, 1.2)
    barrel(g, 14.3, 1.5, 1.2, col="bandana")
    sandbags(g, 11.4, 1.3, 3, 2, 0.9)
    # the lamp and its light
    g.stroke([(8.0, 9.0), (8.0, 7.6)], lw=1.2)
    g.poly([(7.55, 7.2), (8.45, 7.2), (8.25, 7.6), (7.75, 7.6)], "metaldark")
    g.poly([(7.6, 7.2), (8.4, 7.2), (11.2, 0.9), (4.8, 0.9)], "beam", opts=", fill opacity=0.35, draw opacity=0.5", lw=0.8)
    g.circle(8.0, 7.15, 0.18, "lamp", lw=0.8)
    # the cage, the commando inside
    cx, x0, x1, y0, y1 = 8.0, 5.6, 10.4, 1.0, 5.6
    g.rect(x0, y0 - 0.1, x1, y0 + 0.2, "metaldark")
    for k in range(9):
        x = x0 + 0.1 + k * (x1 - x0 - 0.2) / 8
        g.stroke([(x + 0.25, y0 + 0.4), (x + 0.25, y1 + 0.3)], lw=1.2, colour="metaldark", opts=", opacity=0.7")
    with g.frame(cx, y0 + 0.15, 1.35):
        commando(g, rng)
    for k in range(9):
        x = x0 + 0.1 + k * (x1 - x0 - 0.2) / 8
        bar(g, [(x, y0), (x, y1)], 2.0, "metal")
    g.rect(x0 - 0.1, y1, x1 + 0.1, y1 + 0.25, "metal")
    g.rect(x0 - 0.1, y0 + 2.0, x1 + 0.1, y0 + 2.2, "metal", lw=0.8)
    g.rect(cx + 0.75, y0 + 1.5, cx + 1.15, y0 + 1.9, "gold", lw=0.8)
    g.stroke([(cx + 0.82, y0 + 1.9), (cx + 0.82, y0 + 2.1), (cx + 1.08, y0 + 2.1), (cx + 1.08, y0 + 1.9)], lw=1.6, colour="chromedark")
    g.circle(cx + 0.95, y0 + 1.68, 0.05, "ink", lw=0.3)
    return g, "card"


def mission2():
    """Mission two: the officer in the red beret on his balcony, seen
    through a sniper scope: a round view, dark outside, crosshair lines with
    dots and thick outer posts. Lines only, no numbers."""
    W, H = 16.0, 9.0
    rng = random.Random(2)
    g = Fig(ink=1.0)
    g.rect(-0.2, -0.2, W + 0.2, H + 0.2, "wallB")
    for y in (1.2, 4.6, 8.0):
        g.rect(-0.2, y, W + 0.2, y + 0.2, "wallA", lw=0.8)
    for x in (2.0, 13.0):
        g.rect(x, 5.4, x + 1.4, 7.4, "windark")
        g.rect(x - 0.15, 5.25, x + 1.55, 5.4, "trim", lw=0.8)
        g.rect(x + 0.1, 5.5, x + 0.65, 7.3, "wallF", lw=0.7)
        g.rect(x + 0.75, 5.5, x + 1.3, 7.3, "wallF", lw=0.7)
    # the doorway behind him, curtains
    g.rect(5.8, 2.6, 10.2, 8.0, "windark")
    g.blob([(5.8, 8.0), (6.9, 8.0), (6.6, 5.0), (6.9, 2.6), (5.8, 2.6)], "wallC", tension=0.3)
    g.blob([(10.2, 8.0), (9.1, 8.0), (9.4, 5.0), (9.1, 2.6), (10.2, 2.6)], "wallC", tension=0.3)
    g.rect(5.5, 8.0, 10.5, 8.4, "trim")
    # the officer
    with g.frame(8.0, 0.6, 1.7):
        soldier_front(g, rng, "rail", "beret", shirt="olive")
    # the balcony: slab and balusters
    for k in range(13):
        x = 3.9 + k * 0.68
        g.blob([(x - 0.15, 0.6), (x + 0.15, 0.6), (x + 0.24, 1.9), (x + 0.11, 3.4), (x - 0.11, 3.4), (x - 0.24, 1.9)], "paper", tension=0.4, lw=0.8)
    g.rect(3.6, 3.35, 12.4, 3.75, "trim")
    g.rect(3.4, 0.2, 12.6, 0.65, "trim")
    for sgn in (-1, 1):
        g.circle(8.0 + sgn * 1.445, 3.72, 0.26, "skin")
        for k in range(3):
            g.stroke([(8.0 + sgn * 1.445 - 0.12 + k * 0.12, 3.5), (8.0 + sgn * 1.445 - 0.12 + k * 0.12, 3.62)], lw=0.6)
    for px in (4.1, 11.9):
        g.rect(px - 0.4, 3.75, px + 0.4, 4.35, "wallE", lw=0.8)
        for dx, dy, r in ((-0.25, 4.6, 0.35), (0.2, 4.7, 0.4), (0, 5.0, 0.35)):
            g.circle(px + dx, dy, r, rng.choice(["greenA", "greenD"]))
    # the scope: dark all round a round view, the reticle in it
    cx, cy, R = 8.0, 5.5, 3.55
    g.raw(f"\\fill[ink, fill opacity=0.9, even odd rule] (-0.2,-0.2) rectangle ({f(W + 0.2)},{f(H + 0.2)}) ({f(cx)},{f(cy)}) circle ({f(R)});\n", 0)
    g.raw(f"\\draw[hairdark, line width=7pt] ({f(cx)},{f(cy)}) circle ({f(R + 0.08)});\n", 0)
    g.raw(f"\\draw[ink, line width=1.2pt] ({f(cx)},{f(cy)}) circle ({f(R - 0.05)});\n", 0)
    for s in (-1, 1):
        g.stroke([(cx + s * R, cy), (cx + s * R * 0.42, cy)], lw=4.0)
        g.stroke([(cx, cy + s * R), (cx, cy + s * R * 0.42)], lw=4.0)
        g.stroke([(cx + s * R * 0.42, cy), (cx + s * 0.12, cy)], lw=1.0)
        g.stroke([(cx, cy + s * R * 0.42), (cx, cy + s * 0.12)], lw=1.0)
        for k in range(1, 4):
            g.circle(cx + s * k * 0.35, cy, 0.045, "ink", lw=0.3)
            g.circle(cx, cy + s * k * 0.35, 0.045, "ink", lw=0.3)
    g.circle(cx, cy, 0.04, "bandana", lw=0.3)
    g.raw(f"\\draw[white, line width=2pt, opacity=0.35] ({f(cx - 2.2)},{f(cy + 2.0)}) arc (135:160:{f(3.0)});\n", 0)
    return g, "card"


def mission3():
    """Mission three: the top of the comms tower above the coast, the open
    case with the glowing prototype goggles on the deck, the lattice mast
    with dishes above, a blue portal on the tower's concrete side."""
    W, H = 16.0, 9.0
    rng = random.Random(3)
    g = Fig(ink=1.0)
    g.rect(-0.2, -0.2, W + 0.2, H + 0.2, "sky")
    g.rect(-0.2, 6.6, W + 0.2, H + 0.2, "skyhi", lw=0.6)
    g.rect(-0.2, -0.2, W + 0.2, 2.2, "sea")
    for wx, wy in ((1.0, 1.4), (12.5, 0.8), (14.2, 1.6), (2.6, 0.5)):
        g.stroke([(wx, wy), (wx + 0.5, wy)], lw=0.9, colour="seahi")
    # the outpost far below on its headland
    g.blob([(-0.3, 2.2), (2.5, 2.55), (4.0, 2.4), (4.4, 2.2)], "greenD", tension=0.4, lw=0.8)
    g.rect(0.3, 2.3, 3.4, 2.6, "concrete", lw=0.7)
    g.rect(2.4, 2.3, 2.7, 3.2, "concrete", lw=0.7)
    for cx, cy, cs in ((2.6, 7.2, 0.7), (13.6, 5.4, 0.55), (11.5, 8.2, 0.5)):
        cloud(g, cx, cy, cs)
    # the mast above the deck
    lattice_tower(g, 9.6, 4.6, 8.0, 3.2, 1.0, band=True, dishes=True)
    # the tower's concrete shaft, the portal on its side
    g.poly([(5.0, -0.2), (12.2, -0.2), (11.8, 4.2), (5.4, 4.2)], "concrete")
    g.poly([(10.4, -0.2), (12.2, -0.2), (11.8, 4.2), (10.2, 4.2)], "concretedark")
    for y in (1.2, 2.6):
        g.stroke([(5.2, y), (10.3, y)], lw=0.7, colour="concretedark")
    portal(g, 7.4, 1.9, 0.9, 1.35, "blue")
    g.rect(11.0, 0.4, 11.4, 3.8, "metaldark", lw=0.7)
    for k in range(7):
        g.stroke([(11.0, 0.6 + k * 0.5), (11.4, 0.6 + k * 0.5)], lw=0.8)
    # the deck and its railing
    g.rect(4.2, 4.2, 13.0, 4.6, "metal")
    for k in range(12):
        x = 4.4 + k * 0.75
        g.stroke([(x, 4.6), (x, 5.6)], lw=1.2, colour="metaldark")
    g.rect(4.2, 5.55, 13.0, 5.7, "metaldark", lw=0.8)
    # the case on the deck, open, the goggles glowing in it
    with g.frame(0, 0.4, 1.0):
        x0, x1, d = 5.9, 9.1, 0.45
        g.poly([(x0 + d, 5.35), (x1 + d, 5.35), (x1 + d + 0.1, 7.0), (x0 + d + 0.1, 7.0)], "leather")
        g.poly([(x0 + d + 0.18, 5.45), (x1 + d - 0.15, 5.45), (x1 + d - 0.07, 6.85), (x0 + d + 0.26, 6.85)], "bandana", lw=0.8)
        for k in range(1, 4):
            g.stroke([(x0 + d + 0.2 + k * 0.78, 5.5), (x0 + d + 0.28 + k * 0.78, 6.8)], lw=0.6, colour="red")
        g.poly([(x0, 4.6), (x1, 4.6), (x1 + d, 5.35), (x0 + d, 5.35)], "red")
        g.poly([(x0, 4.6), (x1, 4.6), (x1, 4.2 + 0.0), (x0, 4.2)], "leather")
        g.poly([(x1, 4.2), (x1 + d, 4.95), (x1 + d, 5.35), (x1, 4.6)], "leather")
        g.rect((x0 + x1) / 2 - 0.25, 4.32, (x0 + x1) / 2 + 0.25, 4.5, "chrome", lw=0.6)
        gx, gy = (x0 + x1 + d) / 2, 5.25
        g.ellipse(gx, gy + 0.15, 1.65, 0.8, "lens", opts=", fill opacity=0.22, draw opacity=0.3", lw=0.5)
        for a in (20, 45, 70, 90, 110, 135, 160):
            r = math.radians(a)
            g.stroke([(gx + 1.45 * math.cos(r), gy + 0.25 + 0.85 * math.sin(r)), (gx + 2.0 * math.cos(r), gy + 0.25 + 1.25 * math.sin(r))], lw=1.6, colour="lens")
        limb(g, [(gx - 1.25, gy - 0.05), (gx - 0.9, gy + 0.22), (gx + 0.9, gy + 0.22), (gx + 1.25, gy - 0.05)], 0.12, "leather", lw=0.6)
        for sgn in (-1, 1):
            g.circle(gx + sgn * 0.45, gy, 0.4, "leather")
            g.circle(gx + sgn * 0.45, gy, 0.29, "lens", lw=0.8)
            g.circle(gx + sgn * 0.38, gy + 0.08, 0.08, "lenshi", lw=0.4)
        g.rect(gx - 0.1, gy - 0.04, gx + 0.1, gy + 0.1, "leather", lw=0.6)
    # an antenna box on the deck
    g.rect(11.0, 4.6, 12.4, 5.5, "metal")
    g.rect(11.15, 4.75, 11.65, 5.3, "lens", opts=", fill opacity=0.6", lw=0.6)
    g.stroke([(11.9, 5.5), (12.2, 6.5)], lw=1.2)
    return g, "card"


def mission4():
    """Mission four: the Deterrence Unit seen from low in the yard, a
    walking platform as tall as a building: stepped legs, the hull and its
    deck, the railgun reaching out over the head, the red visor. A soldier
    stands on the deck; the operative hangs from the edge of a leg's tier."""
    W, H = 16.0, 9.0
    rng = random.Random(4)
    g = Fig(ink=1.0)
    g.rect(-0.2, -0.2, W + 0.2, H + 0.2, "sky")
    g.rect(-0.2, 6.9, W + 0.2, H + 0.2, "skyhi", lw=0.6)
    for cx, cy, cs in ((1.8, 7.6, 0.6), (14.2, 6.4, 0.55), (11.6, 8.3, 0.45)):
        cloud(g, cx, cy, cs)
    # the far wall and a guard tower, small under it
    g.rect(-0.2, 0.9, W + 0.2, 1.7, "concrete", lw=0.8)
    g.rect(0.3, 1.7, 1.5, 2.5, "wallA", lw=0.8)
    g.rect(0.2, 2.5, 1.6, 2.65, "shingle", lw=0.7)
    g.rect(14.3, 1.7, 15.6, 2.3, "wallC", lw=0.8)
    g.rect(-0.2, -0.2, W + 0.2, 1.0, "paving", lw=0.8)
    # the legs: tiers stepping in toward the hull
    tiers = [(0.0, 0.85, 2.3), (0.85, 1.75, 2.0), (1.75, 2.65, 1.75), (2.65, 3.55, 1.5), (3.55, 4.45, 1.3)]
    for side, cx in ((-1, 4.6), (1, 11.4)):
        for k, (y0, y1, half) in enumerate(tiers):
            g.rect(cx - half, y0 + 0.2, cx + half, y1 + 0.2, "mech" if k % 2 == 0 else "mechdark")
            g.stroke([(cx - half + 0.15, y1 + 0.05), (cx + half - 0.15, y1 + 0.05)], lw=0.6, colour="mechlight")
        # toes
        for t in (-1, 1):
            g.rect(cx + t * 1.4 - 0.45, 0.2, cx + t * 1.4 + 0.45, 0.55, "mechdark", lw=0.8)
    # the hull across the legs
    g.poly([(3.0, 4.4), (13.0, 4.4), (13.4, 6.2), (2.6, 6.2)], "mech")
    g.rect(2.6, 6.2, 13.4, 6.45, "mechdark")
    for x in (4.2, 6.2, 9.8, 11.8):
        g.stroke([(x, 4.55), (x - 0.1, 6.05)], lw=0.6, colour="mechdark")
    # the head under the front of the hull, its visor glowing
    g.poly([(6.6, 3.2), (9.4, 3.2), (9.8, 4.45), (6.2, 4.45)], "mechdark")
    g.rect(6.9, 3.65, 9.1, 4.0, "visor", lw=0.8)
    glow(g, 8.0, 3.82, 0.35, colour="visor", rings=(1.6, 1.25))
    # the reactor and fins on the deck, the railgun reaching out
    g.rect(6.8, 6.45, 9.2, 7.2, "mechdark")
    g.rect(7.3, 6.6, 8.7, 7.0, "visor", lw=0.7)
    for fx in (4.4, 11.2):
        g.poly([(fx, 6.45), (fx + 0.4, 6.45), (fx + 0.3, 7.9), (fx + 0.1, 7.9)], "mechdark")
    g.rect(9.6, 6.45, 10.6, 6.75, "mechdark", lw=0.8)
    g.poly([(9.8, 6.75), (10.4, 6.75), (15.8, 5.6), (15.8, 5.2)], "mech")
    g.rect(15.4, 5.05, 16.2, 5.75, "mechdark", lw=0.8)
    # a soldier on the deck, watching the yard
    with g.frame(5.6, 6.45, 0.42):
        soldier_side(g, rng)
    # the operative hanging by both hands from the edge of a tier on the
    # left leg (seen from behind: her back to us, the portal gun on it)
    hx, hy = 2.72, 2.86
    for sx in (-0.13, 0.13):
        limb(g, [(hx + sx * 1.3, hy - 0.55), (hx + sx * 1.1, hy - 0.25), (hx + sx, hy + 0.02)], [0.12, 0.11, 0.1], "camoB")
        g.circle(hx + sx, hy + 0.03, 0.06, "skin", lw=0.5)
    g.blob([(hx - 0.22, hy - 0.5), (hx + 0.22, hy - 0.5), (hx + 0.19, hy - 1.12), (hx - 0.19, hy - 1.12)], "suit", tension=0.3, lw=0.7)
    camo(g, rng, [(hx, hy - 0.55), (hx, hy - 1.05)], 0.3, 2)
    with g.frame(hx, hy - 0.85, 0.22, 90):
        portal_gun(g)
    for sx in (-0.09, 0.1):
        limb(g, [(hx + sx, hy - 1.1), (hx + sx * 1.3, hy - 1.5), (hx + sx * 1.1, hy - 1.88)], [0.14, 0.12, 0.11], "suit")
        g.blob([(hx + sx * 1.1 - 0.08, hy - 1.86), (hx + sx * 1.1 + 0.08, hy - 1.86), (hx + sx * 1.1 + 0.07, hy - 2.0), (hx + sx * 1.1 - 0.07, hy - 2.0)], "leather", tension=0.3, lw=0.5)
    her_head(g, hx, hy - 0.36, 0.17, rng)
    return g, "card"


def mule_picture():
    g = Fig(ink=1.8)
    mule_head(g)
    return g, "hud"


PICTURES = {"title": title, "failed": failed, "complete": complete, "graceland": graceland,
            "mission1": mission1, "mission2": mission2, "mission3": mission3, "mission4": mission4, "mule": mule_picture}

FORBIDDEN = ["\\node", "\\text", "\\put", "\\char", "\\pgftext", "label=", "pin=", "\\Large", "\\huge", "\\LaTeX", "\\TeX", "\\symbol", "\\verb"]


def document(fig, kind):
    (W, H), _ = SIZE[kind]
    body = fig.render()
    for bad in FORBIDDEN:
        if bad in body:
            raise SystemExit(f"text-setting command {bad!r} in a picture: no words in the art")
    return (PREAMBLE + "\\begin{document}\n\\begin{tikzpicture}\n" + f"\\useasboundingbox (0,0) rectangle ({f(W)},{f(H)});\n\\clip (0,0) rectangle ({f(W)},{f(H)});\n" + body + "\\end{tikzpicture}\n\\end{document}\n")


# ---- checks ----------------------------------------------------------------

def png_read(path):
    """Width, height, colour type and the rows of a PNG (stdlib only)."""
    data = Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    pos, idat = 8, b""
    while pos < len(data):
        n, = struct.unpack(">I", data[pos:pos + 4])
        tag = data[pos + 4:pos + 8]
        chunk = data[pos + 8:pos + 8 + n]
        if tag == b"IHDR":
            w, h, depth, ctype = struct.unpack(">IIBB", chunk[:10])
        elif tag == b"IDAT":
            idat += chunk
        pos += 12 + n
    bpp = {6: 4, 2: 3}.get(ctype)
    if depth != 8 or bpp is None:
        return w, h, ctype, None
    raw = zlib.decompress(idat)
    stride = w * bpp
    rows, prev = [], bytearray(stride)
    for y in range(h):
        ft = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if ft == 1:
                line[i] = (line[i] + a) & 255
            elif ft == 2:
                line[i] = (line[i] + b) & 255
            elif ft == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif ft == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line)
        prev = line
    return w, h, ctype, (rows, bpp)


def check(name, kind, png, preview):
    """Refuse a picture that breaks the facts in the docstring."""
    _, (PW, PH) = SIZE[kind]
    w, h, ctype, _ = png_read(png)
    problems = []
    if (w, h) != (PW, PH):
        problems.append(f"size {w}x{h}, wanted {PW}x{PH}")
    if Path(png).stat().st_size > 1.5e6:
        problems.append(f"{Path(png).stat().st_size} bytes, over 1.5 MB")
    pw, ph, pct, (rows, bpp) = png_read(preview)
    if kind in OPAQUE:
        if ctype != 2:
            problems.append(f"colour type {ctype}: an opaque picture should have no alpha channel")
        if kind == "scene":
            ink = tuple(int(COLOURS["ink"][i:i + 2], 16) for i in (0, 2, 4))
            third = pw // 3
            n = sum(1 for row in rows for x in range(third)
                    if sum(abs(row[x * bpp + i] - ink[i]) for i in range(3)) < 90)
            frac = n / (third * ph)
            if frac > CALM_INK:
                problems.append(f"left third is {frac:.1%} ink, over {CALM_INK:.0%}: not calm")
    else:
        if ctype != 6:
            problems.append("no alpha channel on a cutout")
        alpha = [[row[i * 4 + 3] for i in range(pw)] for row in rows]
        if max(alpha[0]) or max(alpha[-1]) or max(r[0] for r in alpha) or max(r[-1] for r in alpha):
            problems.append("something touches an edge")
        solid = [(x, y) for y in range(ph) for x in range(pw) if alpha[y][x] > 40]
        xs, ys = [x for x, _ in solid], [y for _, y in solid]
        cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
        if abs(cx - pw / 2) > pw * 0.08 or abs(cy - ph / 2) > ph * 0.08:
            problems.append(f"not centred (centre at {cx:.0f},{cy:.0f} of {pw}x{ph})")
    return [f"{name}: {p}" for p in problems]


def main():
    only = set(sys.argv[1:])
    unknown = only - set(PICTURES)
    if unknown:
        raise SystemExit(f"no such picture: {', '.join(sorted(unknown))} (have {', '.join(PICTURES)})")
    failures = []
    for name, make in PICTURES.items():
        if only and name not in only:
            continue
        fig, kind = make()
        tex = document(fig, kind)
        _, (PW, PH) = SIZE[kind]
        clear = [] if kind in OPAQUE else ["-transp"]
        with tempfile.TemporaryDirectory() as tmp:
            t = Path(tmp)
            (t / "art.tex").write_text(tex)
            r = subprocess.run([PDFLATEX, "-interaction=nonstopmode", "-halt-on-error", "art.tex"], cwd=t, capture_output=True, text=True)
            if r.returncode != 0:
                print(r.stdout[-3000:])
                raise SystemExit(f"{name}: pdflatex failed")
            subprocess.run([PDFTOCAIRO, "-png", *clear, "-scale-to-x", str(PW), "-scale-to-y", str(PH), "-singlefile", str(t / "art.pdf"), str(t / "art")], check=True)
            subprocess.run([PDFTOCAIRO, "-png", *clear, "-scale-to-x", str(PW // 8), "-scale-to-y", str(PH // 8), "-singlefile", str(t / "art.pdf"), str(t / "small")], check=True)
            out = OUT / f"{name}.png"
            out.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(t / "art.png", out)
            src = SRC / f"{name}.tex"
            src.parent.mkdir(parents=True, exist_ok=True)
            src.write_text(tex)
            # written first so a refused picture can be looked at; the run still fails
            problems = check(name, kind, out, t / "small.png")
        print("wrote", out.relative_to(ROOT), f"{out.stat().st_size // 1024} KB", "REFUSED" if problems else "")
        failures += problems
    if failures:
        raise SystemExit("\n".join(failures))


if __name__ == "__main__":
    main()
