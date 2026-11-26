#!/usr/bin/env python3
"""Build Keyfluent's theme set: original palettes -> UI/terminal roles, with WCAG AA fixes.

Writes themes.json (all themes: source, license, original palette, final roles, every
adjustment with before/after contrast) and REPORT.md (human-readable; feeds the website's
Themes page). Pure stdlib. Run: python3 docs/design/themes/build_themes.py

Rules (DESIGN.md §5):
  text roles (foreground, muted)            >= 4.5:1 on background AND surface
  accent text (blue..purple, status roles)  >= 4.5:1 on background
  on-accent text (chips, status line, keys) >= 4.5:1 on its accent
  border, focus                             >= 3:1 on background
Failing colors keep hue and chroma; only OKLCH lightness moves (toward contrast), with chroma
reduced only if needed to stay in sRGB. Target 4.55 / 3.05 so rounding never fails.
"""
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent

# slot names: bg, surface, raised, fg, muted, red, orange, yellow, green, aqua, blue, purple
THEMES = [
    dict(id="tomorrow-night-bright", name="Tomorrow Night Bright", family="Tomorrow", kind="dark",
         author="Chris Kempson", url="https://github.com/chriskempson/tomorrow-theme", license="MIT",
         p=dict(bg="#000000", surface="#2a2a2a", raised="#424242", fg="#eaeaea", muted="#969896",
                red="#d54e53", orange="#e78c45", yellow="#e7c547", green="#b9ca4a", aqua="#70c0b1",
                blue="#7aa6da", purple="#c397d8")),
    dict(id="tomorrow-night", name="Tomorrow Night", family="Tomorrow", kind="dark",
         author="Chris Kempson", url="https://github.com/chriskempson/tomorrow-theme", license="MIT",
         p=dict(bg="#1d1f21", surface="#282a2e", raised="#373b41", fg="#c5c8c6", muted="#969896",
                red="#cc6666", orange="#de935f", yellow="#f0c674", green="#b5bd68", aqua="#8abeb7",
                blue="#81a2be", purple="#b294bb")),
    dict(id="tomorrow-night-eighties", name="Tomorrow Night Eighties", family="Tomorrow", kind="dark",
         author="Chris Kempson", url="https://github.com/chriskempson/tomorrow-theme", license="MIT",
         p=dict(bg="#2d2d2d", surface="#393939", raised="#515151", fg="#cccccc", muted="#999999",
                red="#f2777a", orange="#f99157", yellow="#ffcc66", green="#99cc99", aqua="#66cccc",
                blue="#6699cc", purple="#cc99cc")),
    dict(id="tomorrow-night-blue", name="Tomorrow Night Blue", family="Tomorrow", kind="dark",
         author="Chris Kempson", url="https://github.com/chriskempson/tomorrow-theme", license="MIT",
         p=dict(bg="#002451", surface="#00346e", raised="#003f8e", fg="#ffffff", muted="#7285b7",
                red="#ff9da4", orange="#ffc58f", yellow="#ffeead", green="#d1f1a9", aqua="#99ffff",
                blue="#bbdaff", purple="#ebbbff")),
    dict(id="tomorrow", name="Tomorrow", family="Tomorrow", kind="light",
         author="Chris Kempson", url="https://github.com/chriskempson/tomorrow-theme", license="MIT",
         p=dict(bg="#ffffff", surface="#efefef", raised="#d6d6d6", fg="#4d4d4c", muted="#8e908c",
                red="#c82829", orange="#f5871f", yellow="#eab700", green="#718c00", aqua="#3e999f",
                blue="#4271ae", purple="#8959a8")),
    dict(id="solarized-dark", name="Solarized Dark", family="Solarized", kind="dark",
         author="Ethan Schoonover", url="https://ethanschoonover.com/solarized/", license="MIT",
         p=dict(bg="#002b36", surface="#073642", raised="#0e4553", fg="#839496", muted="#586e75",
                red="#dc322f", orange="#cb4b16", yellow="#b58900", green="#859900", aqua="#2aa198",
                blue="#268bd2", purple="#6c71c4")),
    dict(id="solarized-light", name="Solarized Light", family="Solarized", kind="light",
         author="Ethan Schoonover", url="https://ethanschoonover.com/solarized/", license="MIT",
         p=dict(bg="#fdf6e3", surface="#eee8d5", raised="#e2dcc8", fg="#657b83", muted="#93a1a1",
                red="#dc322f", orange="#cb4b16", yellow="#b58900", green="#859900", aqua="#2aa198",
                blue="#268bd2", purple="#6c71c4")),
    dict(id="gruvbox-dark", name="Gruvbox Dark", family="Gruvbox", kind="dark",
         author="Pavel Pertsev (morhetz)", url="https://github.com/morhetz/gruvbox", license="MIT",
         p=dict(bg="#282828", surface="#3c3836", raised="#504945", fg="#ebdbb2", muted="#928374",
                red="#fb4934", orange="#fe8019", yellow="#fabd2f", green="#b8bb26", aqua="#8ec07c",
                blue="#83a598", purple="#d3869b")),
    dict(id="gruvbox-light", name="Gruvbox Light", family="Gruvbox", kind="light",
         author="Pavel Pertsev (morhetz)", url="https://github.com/morhetz/gruvbox", license="MIT",
         p=dict(bg="#fbf1c7", surface="#ebdbb2", raised="#d5c4a1", fg="#3c3836", muted="#928374",
                red="#9d0006", orange="#af3a03", yellow="#b57614", green="#79740e", aqua="#427b58",
                blue="#076678", purple="#8f3f71")),
    dict(id="dracula", name="Dracula", family="Dracula", kind="dark",
         author="Zeno Rocha and contributors", url="https://draculatheme.com", license="MIT",
         p=dict(bg="#282a36", surface="#44475a", raised="#565970", fg="#f8f8f2", muted="#6272a4",
                red="#ff5555", orange="#ffb86c", yellow="#f1fa8c", green="#50fa7b", aqua="#8be9fd",
                blue="#bd93f9", purple="#ff79c6")),
    dict(id="nord", name="Nord", family="Nord", kind="dark",
         author="Arctic Ice Studio and Sven Greb", url="https://www.nordtheme.com", license="MIT",
         p=dict(bg="#2e3440", surface="#3b4252", raised="#434c5e", fg="#d8dee9", muted="#616e88",
                red="#bf616a", orange="#d08770", yellow="#ebcb8b", green="#a3be8c", aqua="#8fbcbb",
                blue="#88c0d0", purple="#b48ead")),
    dict(id="catppuccin-mocha", name="Catppuccin Mocha", family="Catppuccin", kind="dark",
         author="Catppuccin", url="https://github.com/catppuccin/catppuccin", license="MIT",
         p=dict(bg="#1e1e2e", surface="#313244", raised="#45475a", fg="#cdd6f4", muted="#a6adc8",
                red="#f38ba8", orange="#fab387", yellow="#f9e2af", green="#a6e3a1", aqua="#94e2d5",
                blue="#89b4fa", purple="#cba6f7")),
    dict(id="catppuccin-latte", name="Catppuccin Latte", family="Catppuccin", kind="light",
         author="Catppuccin", url="https://github.com/catppuccin/catppuccin", license="MIT",
         p=dict(bg="#eff1f5", surface="#e6e9ef", raised="#ccd0da", fg="#4c4f69", muted="#6c6f85",
                red="#d20f39", orange="#fe640b", yellow="#df8e1d", green="#40a02b", aqua="#179299",
                blue="#1e66f5", purple="#8839ef")),
    dict(id="green-phosphor", name="Green phosphor", family="Keyfluent", kind="dark", mono=True,
         author="Keyfluent", url="https://github.com/samuelkordik/keyfluent", license="MIT",
         p=dict(bg="#0a0f0a", surface="#122012", raised="#1a301a", fg="#33ff66", muted="#1fbf4a",
                red="#33ff66", orange="#33ff66", yellow="#33ff66", green="#33ff66", aqua="#33ff66",
                blue="#33ff66", purple="#33ff66")),
    dict(id="amber-phosphor", name="Amber phosphor", family="Keyfluent", kind="dark", mono=True,
         author="Keyfluent", url="https://github.com/samuelkordik/keyfluent", license="MIT",
         p=dict(bg="#0f0a00", surface="#1f1500", raised="#2e2000", fg="#ffb000", muted="#c78600",
                red="#ffb000", orange="#ffb000", yellow="#ffb000", green="#ffb000", aqua="#ffb000",
                blue="#ffb000", purple="#ffb000")),
]

ACCENTS = ["red", "orange", "yellow", "green", "aqua", "blue", "purple"]


# ---------- color math ----------
def hex2rgb(h):
    h = h.lstrip("#")
    return [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)]


def rgb2hex(c):
    return "#" + "".join(f"{round(max(0, min(1, v)) * 255):02x}" for v in c)


def lin(v):
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def delin(v):
    return 12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055


def lum(h):
    r, g, b = (lin(v) for v in hex2rgb(h))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def to_oklch(h):
    r, g, b = (lin(v) for v in hex2rgb(h))
    l_ = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b) ** (1 / 3)
    m_ = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b) ** (1 / 3)
    s_ = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b) ** (1 / 3)
    L = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_
    A = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_
    B = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_
    return L, math.hypot(A, B), math.atan2(B, A)


def from_oklch(L, C, H):
    A, B = C * math.cos(H), C * math.sin(H)
    l_ = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3
    m_ = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3
    s_ = (L - 0.0894841775 * A - 1.2914855480 * B) ** 3
    r = 4.0767416621 * l_ - 3.3077118200 * m_ + 0.2309699292 * s_
    g = -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_
    b = -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_
    return [r, g, b]


def in_gamut(c):
    return all(-1e-4 <= v <= 1 + 1e-4 for v in c)


def oklch_hex(L, C, H):
    while C > 0 and not in_gamut(from_oklch(L, C, H)):
        C -= 0.002
    return rgb2hex([delin(max(0, min(1, v))) for v in from_oklch(L, max(C, 0), H)])


def fix(color, against, need):
    """Nearest color of the same hue/chroma passing `need` on every bg in `against`."""
    if all(ratio(color, b) >= need for b in against):
        return color
    L, C, H = to_oklch(color)
    dark_bg = max(lum(b) for b in against) < 0.18
    step = 0.004 if dark_bg else -0.004
    cand = color
    while 0 <= L <= 1:
        L += step
        cand = oklch_hex(L, C, H)
        if all(ratio(cand, b) >= need for b in against):
            return cand
    return "#ffffff" if dark_bg else "#000000"


def best_on(accent, options):
    """Text color for use on an accent fill: first option that passes, else best of black/white."""
    for o in options:
        if ratio(o, accent) >= 4.55:
            return o
    return max(["#000000", "#ffffff"], key=lambda o: ratio(o, accent))


# ---------- build ----------
def build(t):
    p, adj = t["p"], []
    roles = {}

    def put(role, color, against, need, why):
        final = fix(color, against, need)
        roles[role] = final
        if final != color:
            adj.append(dict(role=role, original=color, adjusted=final, why=why,
                            before=round(min(ratio(color, b) for b in against), 2),
                            after=round(min(ratio(final, b) for b in against), 2), need=4.5 if need > 4 else 3.0))

    bg, surface = p["bg"], p["surface"]
    roles["background"], roles["surface"], roles["surface-raised"] = bg, surface, p["raised"]
    put("foreground", p["fg"], [bg, surface, p["raised"]], 4.55, "body text on background, surface and raised panels")
    put("muted", p["muted"], [bg, surface], 4.55, "secondary text on background and surface")
    for a in ACCENTS:
        put(a, p[a], [bg], 4.55, f"{a} used as text (grade words, category labels) on background")
    # semantic roles
    roles["primary"] = roles["blue"]
    roles["link"] = roles["blue"]
    roles["danger"], roles["warning"], roles["success"], roles["info"] = (
        roles["red"], roles["yellow"], roles["green"], roles["aqua"])
    put("border", p["muted"], [bg, surface], 3.05, "panel frames on background and surface")
    put("focus", p["yellow"], [bg, surface], 3.05, "focus ring on background and surface")
    # text on accent fills: prefer the theme's own background, then foreground
    for a in ACCENTS:
        roles[f"on-{a}"] = best_on(roles[a], [bg, roles["foreground"]])
    roles["on-primary"] = roles["on-blue"]
    # category, grade and terminal mapping (aliases, resolved here for the app)
    mapping = {
        "category.basics": "blue", "category.copy-mode": "green", "category.sessions": "purple",
        "category.layout": "orange", "grade.easy": "aqua", "grade.good": "green",
        "grade.hard": "yellow", "grade.again": "red", "stars.on": "yellow", "stars.off": "surface-raised",
        "terminal.status-bg": "green", "terminal.status-fg": "on-green",
        "terminal.active-border": "green", "terminal.border": "border",
        "terminal.selection": "surface-raised", "terminal.prompt-user": "green",
        "terminal.prompt-path": "blue", "fkey.bg": "surface", "fkey.key-bg": "blue", "fkey.key-fg": "on-blue",
    }
    stripe = ["red", "orange", "yellow", "green", "aqua", "blue"]
    # original-colors mode: same mapping, no fixes
    orig = dict(background=bg, surface=surface, **{"surface-raised": p["raised"]},
                foreground=p["fg"], muted=p["muted"], **{a: p[a] for a in ACCENTS})
    return dict(id=t["id"], name=t["name"], family=t["family"], kind=t["kind"],
                monochrome=bool(t.get("mono")), author=t["author"], url=t["url"], license=t["license"],
                original=p, roles=roles, aliases=mapping, stripe=stripe, original_roles=orig,
                adjustments=adj)


def main():
    out = [build(t) for t in THEMES]
    (HERE / "themes.json").write_text(json.dumps(dict(
        schema=1, generated_by="docs/design/themes/build_themes.py", defaults=dict(
            dark="tomorrow-night-bright", light="tomorrow"), themes=out), indent=2) + "\n")
    lines = ["# Keyfluent themes: WCAG AA report", "",
             "_Generated by `build_themes.py`; do not edit. Source for the website's Themes page "
             "and the About screen credits._", "",
             "Every theme is checked against WCAG 2.2 AA: body and secondary text at least 4.5:1 on "
             "the background and panels, colored text at least 4.5:1 on the background, borders and "
             "the focus ring at least 3:1. Where an original color falls short, Keyfluent uses the "
             "nearest color with the same hue and saturation that passes (only lightness changes). "
             "**Preferences → Appearance → Use original theme colors** turns these changes off.", "",
             "| Theme | By | License | Kind | Colors changed |", "|---|---|---|---|---|"]
    for t in out:
        lines.append(f"| [{t['name']}]({t['url']}) | {t['author']} | {t['license']} | {t['kind']} | "
                     f"{len(t['adjustments']) or 'none'} |")
    for t in out:
        lines += ["", f"## {t['name']}", ""]
        if not t["adjustments"]:
            lines.append("Passes as published. No colors changed.")
            continue
        lines += ["| Role | Original | Keyfluent | Before | After | Needed | Why |", "|---|---|---|---|---|---|---|"]
        for a in t["adjustments"]:
            lines.append(f"| {a['role']} | `{a['original']}` | `{a['adjusted']}` | {a['before']}:1 | "
                         f"{a['after']}:1 | {a['need']:.1f}:1 | {a['why']} |")
    (HERE / "REPORT.md").write_text("\n".join(lines) + "\n")
    for t in out:
        print(f"{t['id']:26} {len(t['adjustments'])} adjusted")


if __name__ == "__main__":
    main()
