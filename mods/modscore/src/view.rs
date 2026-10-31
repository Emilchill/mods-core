//! The Mods Core panel: the whole panel once (embedded in the component), then value patches.

use crate::list::Entry;
use crate::surface_controls::{CONTROLS, DOCUMENT_ESCAPED, HEIGHT, LINE, ROWS, THUMB_TOP, THUMB_TRAVEL, THUMB_X, WIDTH};

/// The key that opens Mods Core. Cinnabar also always opens a menu with it.
pub const TOGGLE_KEY: &str = "F10";
pub const SCREEN: &str = "modscore.menu";
/// Blur of the world behind the open list, in GUI pixels (Cinnabar allows up to 32).
const WORLD_BLUR: usize = 8;
/// Brightness of the world behind the open list (1 is unchanged).
const WORLD_BRIGHTNESS: &str = "0.7";

/// The whole panel, for the component's panel bundle.
pub fn full() -> String {
    let mut o = String::with_capacity(DOCUMENT_ESCAPED.len() + 1024);
    o.push_str("{\"toggle_key\":\"");
    o.push_str(TOGGLE_KEY);
    o.push_str("\",\"motion\":true,\"capture_key\":false,\"reference_size\":[");
    push_u(&mut o, (WIDTH + 20) as usize);
    o.push(',');
    push_u(&mut o, (HEIGHT + 20) as usize);
    o.push_str("],\"controls\":[");
    for (n, id) in CONTROLS.iter().enumerate() {
        if n > 0 {
            o.push(',');
        }
        o.push_str("{\"kind\":\"button\",\"id\":\"");
        o.push_str(id);
        o.push_str("\",\"label\":\"");
        o.push_str(id);
        o.push_str("\"}");
    }
    o.push_str("],\"surface\":{\"screen\":\"");
    o.push_str(SCREEN);
    o.push_str("\",\"document\":\"");
    o.push_str(DOCUMENT_ESCAPED);
    o.push_str("\",");
    // The world behind the list blurs and dims in, and back out when it closes.
    o.push_str("\"transitions\":{\"#world_blur\":{\"duration\":0.25,\"from\":0,\"exit\":0},");
    o.push_str("\"#world_brightness\":{\"duration\":0.25,\"from\":1,\"exit\":1}},");
    bindings(&mut o, &[], 0);
    o.push_str("}}");
    o
}

/// The values for `entries` scrolled to `top`.
pub fn patch(entries: &[Entry], top: usize) -> String {
    let mut o = String::with_capacity(2048);
    o.push_str("{\"controls\":[],");
    bindings(&mut o, entries, top);
    o.push('}');
    o
}

/// The first row index that keeps a full page in view.
pub fn clamp_top(top: usize, count: usize) -> usize {
    top.min(count.saturating_sub(ROWS))
}

fn bindings(o: &mut String, entries: &[Entry], top: usize) {
    // Plain pushes only: `format!` costs a lot of fuel in WebAssembly.
    let top = clamp_top(top, entries.len());
    let count = entries.len();
    // Cinnabar blurs and dims the world behind an open panel with these (GUI pixels, factor).
    o.push_str("\"bindings\":{\"#world_blur\":");
    push_u(o, WORLD_BLUR);
    o.push_str(",\"#world_brightness\":");
    o.push_str(WORLD_BRIGHTNESS);
    o.push_str(",\"#count\":\"");
    match count {
        0 => o.push_str("No mods yet"),
        1 => o.push_str("1 mod"),
        n => {
            push_u(o, n);
            o.push_str(" mods");
        }
    }
    o.push('"');
    o.push_str(if count == 0 { ",\"#empty\":true" } else { ",\"#empty\":false" });
    let mut key = String::with_capacity(12);
    for row in 0..ROWS {
        let entry = entries.get(top + row);
        let mut name = |what: &str| {
            key.clear();
            key.push_str("#r");
            key.push((b'0' + row as u8) as char);
            key.push('_');
            key.push_str(what);
            key.clone()
        };
        o.push_str(",\"");
        o.push_str(&name("on"));
        o.push_str(if entry.is_some() { "\":true" } else { "\":false" });
        let (title, version, description) = entry.map_or(("", "", ""), |e| (e.name.as_str(), e.version.as_str(), e.description.as_str()));
        let [d1, d2] = wrap(description);
        pair(o, &name("name"), &clip(title, 40), false);
        let mut ver = String::new();
        if !version.is_empty() {
            ver.push('v');
            ver.push_str(&clip(version, 12));
        }
        pair(o, &name("ver"), &ver, false);
        pair(o, &name("d1"), &d1, false);
        pair(o, &name("d2"), &d2, false);
    }
    o.push_str(",\"#page\":\"");
    if count > 0 {
        push_u(o, top + 1);
        o.push('-');
        push_u(o, (top + ROWS).min(count));
        o.push_str(" of ");
        push_u(o, count);
    }
    o.push('"');
    let scrollable = count.saturating_sub(ROWS);
    let travel = if scrollable == 0 { 0 } else { THUMB_TRAVEL as usize * top / scrollable };
    o.push_str(",\"#thumb\":[");
    push_u(o, THUMB_X as usize);
    o.push(',');
    push_u(o, THUMB_TOP as usize + travel);
    o.push_str("]}");
}

fn push_u(o: &mut String, mut n: usize) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for b in &buf[i..] {
        o.push(*b as char);
    }
}

/// `"key":"value"` with the value escaped.
fn pair(o: &mut String, key: &str, value: &str, first: bool) {
    if !first {
        o.push(',');
    }
    o.push('"');
    o.push_str(key);
    o.push_str("\":\"");
    // Most text needs no escaping: copy it in one go.
    if !value.bytes().any(|b| b == b'"' || b == b'\\' || b < 0x20 || b == 0x7f) {
        o.push_str(value);
        o.push('"');
        return;
    }
    for ch in value.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if c.is_control() => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
}

/// At most `max` characters, with "..." when cut.
fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

/// A description in two lines, cut at a word; "..." when it does not fit.
pub fn wrap(text: &str) -> [String; 2] {
    // Short text fits on the first line as it is.
    if text.len() <= LINE && !text.contains('\n') {
        return [text.to_owned(), String::new()];
    }
    let mut lines = [String::new(), String::new()];
    let mut line = 0;
    let mut cut = false;
    for word in text.split_whitespace() {
        let len = lines[line].chars().count();
        let need = word.chars().count() + usize::from(len > 0);
        if len + need > LINE {
            if line == 1 {
                cut = true;
                break;
            }
            line = 1;
        }
        if !lines[line].is_empty() {
            lines[line].push(' ');
        }
        lines[line].push_str(word);
    }
    for l in &mut lines {
        if l.chars().count() > LINE {
            *l = clip(l, LINE);
        }
    }
    if cut {
        let l = &mut lines[1];
        while l.chars().count() > LINE - 3 {
            l.pop();
        }
        l.push_str("...");
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: usize) -> Entry {
        Entry { slug: format!("m{n}"), name: format!("Mod {n}"), version: "1.0".into(), description: "Does \"things\".".into() }
    }

    #[test]
    fn panel_and_patch_are_json() {
        let full: serde_json::Value = serde_json::from_str(&full()).unwrap();
        assert_eq!(full["toggle_key"], "F10");
        assert_eq!(full["surface"]["bindings"]["#empty"], true);
        assert_eq!(full["surface"]["bindings"]["#world_blur"], 8);
        assert_eq!(full["surface"]["transitions"]["#world_blur"]["exit"], 0);
        let entries: Vec<Entry> = (0..7).map(entry).collect();
        let patch: serde_json::Value = serde_json::from_str(&patch(&entries, 9)).unwrap();
        let b = &patch["bindings"];
        assert_eq!(b["#page"], "3-7 of 7");
        assert_eq!(b["#r0_name"], "Mod 2");
        assert_eq!(b["#r4_ver"], "v1.0");
        assert_eq!(b["#r0_d1"], "Does \"things\".");
        assert_eq!(b["#count"], "7 mods");
        assert_eq!(b["#thumb"][1], THUMB_TOP + THUMB_TRAVEL);
    }

    #[test]
    fn descriptions_wrap_in_two_lines() {
        let [a, b] = wrap("short");
        assert_eq!((a.as_str(), b.as_str()), ("short", ""));
        let long = "word ".repeat(40);
        let [a, b] = wrap(&long);
        assert!(a.chars().count() <= LINE && b.ends_with("...") && b.chars().count() <= LINE);
    }
}
