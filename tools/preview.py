#!/usr/bin/env python3
"""Rough PNG preview of the Mods Core screen with sample mods. usage: preview.py out.png"""
import json, sys, pathlib
from PIL import Image, ImageDraw, ImageFont
src = pathlib.Path(__file__).resolve().parents[1] / "mods/modscore/src"
doc = json.loads((src / "surface.json").read_text())
S = 3
font = lambda sc, bold=False: ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans%s.ttf" % ("-Bold" if bold else ""), max(6, int(8.5 * sc * S)))
mods = [("Day Counter", "6.1.0", ["Shows the world day above your hotbar, and the time if you", "want."]),
        ("Keystrokes", "4.1.0", ["WASD, jump and mouse buttons on screen, with clicks per second.", ""]),
        ("Technic Tweaks", "3.1.0", ["Chunk borders, slime chunks, guardian spawns, light spots and", "hitboxes, plus an info HUD."]),
        ("Waypoints", "2.1.0", ["Waypoints with beacons and labels you can see through blocks.", ""]),
        ("Zoomify", "4.1.0", ["Smooth zoom with transitions, spyglass and cinematic camera.", ""])]
vals = {"#count": "5 mods", "#empty": False, "#page": "1-5 of 5", "#thumb": [308, 62]}
for i, (n, v, d) in enumerate(mods):
    vals.update({f"#r{i}_on": True, f"#r{i}_name": n, f"#r{i}_ver": "v" + v, f"#r{i}_d1": d[0], f"#r{i}_d2": d[1]})
box = doc["menu"]["controls"][0]["box"]
W, H = box["size"]
img = Image.new("RGB", ((W + 40) * S, (H + 40) * S), (95, 140, 70))
d = ImageDraw.Draw(img, "RGBA")
col = lambda c: tuple(int(255 * x) for x in c[:3]) + (int(255 * c[3]),)
def draw(node, ox, oy, pw, ph):
    w, h = node.get("size", [pw, ph]); off = node.get("offset", [0, 0]); x, y = ox + off[0], oy + off[1]
    for b in node.get("bindings", []):
        v = vals.get(b["binding_name"]); o = b.get("binding_name_override")
        if o == "#visible" and not v: return
        if o == "#offset" and v: x, y = ox + v[0], oy + v[1]
    t = node.get("type")
    if t == "custom":
        d.rounded_rectangle([x * S, y * S, (x + w) * S, (y + h) * S], radius=node.get("radius", 0) * S, fill=col(node["color"]))
    elif t == "label":
        s = node["text"]; s = str(vals.get(s, s)) if s.startswith("#") else s
        f = font(node.get("font_scale_factor", 1), node.get("font_type") == "default" and node.get("font_scale_factor", 1) > 1.5)
        tw = d.textlength(s, font=f); al = node.get("text_alignment", "left")
        tx = x * S if al == "left" else (x + w) * S - tw if al == "right" else x * S + (w * S - tw) / 2
        if node.get("shadow"): d.text((tx + S, y * S), s, font=f, fill=(40, 40, 40, 255))
        d.text((tx, y * S - S), s, font=f, fill=col(node.get("color", [1, 1, 1, 1])))
    for ch in node.get("controls", []):
        draw(list(ch.values())[0], x, y, w, h)
draw(box, 20, 20, W, H)
img.save(sys.argv[1])
