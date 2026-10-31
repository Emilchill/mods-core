#!/usr/bin/env python3
"""Generates the Mods Core screen (src/surface.json, src/surface.escaped, src/surface_controls.rs).

The look follows Bedrock's Ore UI: square slate panels with near-black outlines, a header bar
with an upper-case title, list rows with a light top edge and a dark bottom edge, light grey
bevelled buttons, and the game's own font. Everything that changes is a `#binding` the mod
sends; the document itself never changes.

TITLE_FONT=mod_panel draws the title in the font embedded in the component (see
scripts/build-mod.py --font); the default is the game's font."""
import json, os, pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1] / "mods/modscore/src"
ROWS = 5
W = 340
HEAD = 28           # header bar
SUB = 16            # "5 mods" strip under it
ROW_H, GAP = 36, 4
TOP = HEAD + SUB + 4
LIST_H = ROWS * (ROW_H + GAP) - GAP
FOOT = 22
H = TOP + LIST_H + FOOT
LINE = 58           # characters per description line

GAME = "default"
TITLE_FONT = os.environ.get("TITLE_FONT", GAME)

# Ore UI colours (dark theme).
OUTLINE = [0.118, 0.118, 0.122, 1]      # #1E1E1F
HEADER = [0.192, 0.196, 0.2, 0.98]      # #313233
BODY = [0.282, 0.286, 0.29, 0.97]       # #48494A
ROW = [0.192, 0.196, 0.2, 1]            # #313233
ROW_TOP = [0.353, 0.357, 0.361, 1]      # #5A5B5C
ROW_BOTTOM = [0.141, 0.141, 0.149, 1]   # #242426
BUTTON = [0.816, 0.82, 0.831, 1]        # #D0D1D4
BUTTON_TOP = [1, 1, 1, 1]
BUTTON_BOTTOM = [0.345, 0.345, 0.353, 1]  # #58585A
TAG = [0.141, 0.141, 0.149, 1]
WHITE = [1, 1, 1, 1]
GREY = [0.694, 0.698, 0.71, 1]          # #B1B2B5
DARK_TEXT = [0.118, 0.118, 0.122, 1]
GREEN = [0.42, 0.78, 0.33, 1]           # #6BC854

controls = []


def control(cid):
    if cid not in controls:
        controls.append(cid)
    return controls.index(cid)


def place(node, size, offset, anchor="top_left"):
    node.update({"size": size, "offset": offset, "anchor_from": anchor, "anchor_to": anchor})
    return node


def rect(size, offset, color, bindings=None):
    n = place({"type": "custom", "renderer": "cinnabar_rounded_rectangle", "radius": 0, "color": color}, size, offset)
    if bindings:
        n["bindings"] = bindings
    return n


def text(value, size, offset, color=WHITE, scale=1.0, align="left", bound=False, font=GAME, shadow=False):
    n = place({"type": "label", "text": value, "color": color, "localize": False, "font_type": font,
               "font_scale_factor": scale}, size, offset)
    if align != "left":
        n["text_alignment"] = align
    if shadow:
        n["shadow"] = True
    if bound:
        n["bindings"] = [{"binding_name": value}]
    return n


def visible(node, binding):
    node.setdefault("bindings", []).append({"binding_name": binding, "binding_name_override": "#visible"})
    return node


def named(name, node):
    return {name: node}


def hover(idx, size, color=[1, 1, 1, 0.07]):
    return named("hover", rect(size, [0, 0], color, [{"binding_name": f"#row_{idx}_hover", "binding_name_override": "#alpha"}]))


def button(cid, size, offset, children, glow=True, action=None):
    idx = control(cid) if action is None else None
    kids = children + ([hover(idx, size)] if glow and idx is not None else [])
    return place({"type": "button", "controls": kids,
                  "button_mappings": [{"from_button_id": "button.menu_select",
                                       "to_button_id": action or f"mod.control:{idx}", "mapping_type": "pressed"}]},
                 size, offset)


def bevel(size, face, top, bottom):
    """An Ore UI block: outline, a light top edge and a darker bottom edge."""
    w, h = size
    return [
        named("outline", rect([w, h], [0, 0], OUTLINE)),
        named("bottom", rect([w - 2, h - 2], [1, 1], bottom)),
        named("face", rect([w - 2, h - 4], [1, 1], face)),
        named("top", rect([w - 2, 1], [1, 1], top)),
    ]


def panel(size, offset, children):
    return place({"type": "panel", "controls": children}, size, offset)


def light_button(cid, size, offset, label, action=None, scale=1.0):
    w, h = size
    kids = bevel(size, BUTTON, BUTTON_TOP, BUTTON_BOTTOM)
    kids.append(named("t", text(label, [w, 10], [0, (h - 2 - 8 * scale) / 2], DARK_TEXT, scale, "center")))
    return button(cid, size, offset, kids, action=action)


# ---------------------------------------------------------------- rows
rw = W - 16 - 22
rows = []
for i in range(ROWS):
    y = TOP + i * (ROW_H + GAP)
    idx = control(f"row_{i}")
    row = button(f"row_{i}", [rw, ROW_H], [0, 0], [
        *bevel([rw, ROW_H], ROW, ROW_TOP, ROW_BOTTOM),
        named("name", text(f"#r{i}_name", [rw - 90, 10], [8, 5], WHITE, 1.0, bound=True, shadow=True)),
        named("tag_bg", rect([58, 11], [rw - 80, 4], TAG)),
        named("ver", text(f"#r{i}_ver", [56, 10], [rw - 79, 5.5], GREY, 0.8, "center", True)),
        named("chev", text(">", [10, 10], [rw - 16, 13], GREY, 1.0, "center")),
        named("d1", text(f"#r{i}_d1", [rw - 30, 9], [8, 17], GREY, 0.75, bound=True)),
        named("d2", text(f"#r{i}_d2", [rw - 30, 9], [8, 26], GREY, 0.75, bound=True)),
    ], glow=False)
    row["controls"].append(hover(idx, [rw - 2, ROW_H - 2], [1, 1, 1, 0.08]))
    rows.append(named(f"row{i}", visible(panel([rw, ROW_H], [8, y], [named("b", row)]), f"#r{i}_on")))

# ---------------------------------------------------------------- scroll bar
sx = W - 8 - 14
thumb_top = TOP + 16
thumb_h = 18
travel = LIST_H - 32 - thumb_h
scroll = [
    named("up", light_button("up", [14, 14], [sx, TOP], "^")),
    named("down", light_button("down", [14, 14], [sx, TOP + LIST_H - 14], "v")),
    named("track", rect([6, LIST_H - 32], [sx + 4, TOP + 16], OUTLINE)),
    named("thumb", place({"type": "panel", "controls": [
        named("o", rect([6, thumb_h], [0, 0], OUTLINE)),
        named("f", rect([4, thumb_h - 2], [1, 1], BUTTON)),
    ], "bindings": [{"binding_name": "#thumb", "binding_name_override": "#offset"}]}, [6, thumb_h], [sx + 4, thumb_top])),
]

# ---------------------------------------------------------------- frame
box = [
    named("outline", rect([W, H], [0, 0], OUTLINE)),
    named("body", rect([W - 4, H - 4], [2, 2], BODY)),
    named("head", rect([W - 4, HEAD - 2], [2, 2], HEADER)),
    named("head_line", rect([W - 4, 2], [2, HEAD], OUTLINE)),
    named("title", text("MODS", [W, 12], [0, 9], WHITE, 1.3 if TITLE_FONT == GAME else 1.25, "center",
                        font=TITLE_FONT, shadow=True)),
    named("close", light_button("close", [16, 16], [W - 22, 6], "x", action="mod.close")),
    named("count", text("#count", [W - 20, 10], [10, HEAD + 5], GREEN, 0.8, "left", True)),
    named("empty", visible(text("No mods to show yet.", [W - 20, 10], [10, TOP + 20], GREY, 1.0, "center"), "#empty")),
    named("empty2", visible(text("Mods appear here when they support Mods Core.", [W - 20, 10], [10, TOP + 34], GREY, 0.8,
                                 "center"), "#empty")),
    *rows,
    *scroll,
    named("foot_line", rect([W - 4, 2], [2, H - FOOT], OUTLINE)),
    named("page", text("#page", [120, 10], [10, H - FOOT + 7], GREY, 0.8, "left", True)),
    named("hint", text("Click a mod to open its settings", [W - 140, 10], [W - (W - 140) - 10, H - FOOT + 7], GREY,
                       0.8, "right")),
]
screen = {"type": "screen", "size": ["100%", "100%"], "controls": [
    named("box", place({"type": "panel", "controls": box}, [W, H], [0, 0], "center")),
]}
document = {"namespace": "modscore", "menu": screen}


def tidy(v):
    if isinstance(v, float):
        return round(v, 3) if v != int(v) else int(v)
    if isinstance(v, list):
        return [tidy(x) for x in v]
    if isinstance(v, dict):
        return {k: tidy(x) for k, x in v.items()}
    return v


doc = json.dumps(tidy(document), separators=(",", ":"), ensure_ascii=False)
(ROOT / "surface.json").write_text(doc)
(ROOT / "surface.escaped").write_text(json.dumps(doc, ensure_ascii=False)[1:-1])
lines = ["//! Generated by tools/gen_surface.py: the screen's buttons, in the order they press",
         "//! `mod.control:<index>`.", "",
         f"pub const CONTROLS: [&str; {len(controls)}] = [{', '.join(json.dumps(c) for c in controls)}];", "",
         "/// The document, already escaped to sit inside a JSON string.",
         'pub const DOCUMENT_ESCAPED: &str = include_str!("surface.escaped");', "",
         f"pub const ROWS: usize = {ROWS};",
         f"pub const WIDTH: u32 = {W};", f"pub const HEIGHT: u32 = {H};",
         "/// Characters per description line.",
         f"pub const LINE: usize = {LINE};",
         "/// Top of the scroll thumb and how far it travels.",
         f"pub const THUMB_X: u32 = {sx + 4};", f"pub const THUMB_TOP: u32 = {thumb_top};",
         f"pub const THUMB_TRAVEL: u32 = {travel};", ""]
(ROOT / "surface_controls.rs").write_text("\n".join(lines))
print(len(controls), "controls,", len(doc), "bytes,", f"{W}x{H}")
