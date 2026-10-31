//! The list of mods Mods Core has heard from, put together from their info chunks.

use crate::mods_core::{self, Chunk};

/// Frames a mod stays listed after its last chunk (about 4 seconds).
pub const FORGET_AFTER: u32 = 240;
/// Most mods kept.
pub const MAX_MODS: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

/// A text still arriving.
struct Partial {
    slug: String,
    checksum: u32,
    parts: Vec<Option<[u8; mods_core::CHUNK]>>,
}

#[derive(Default)]
pub struct List {
    /// Listed mods, sorted by name.
    pub entries: Vec<Entry>,
    /// Frame each slug was last heard.
    seen: Vec<(String, u32, u32)>,
    partial: Vec<Partial>,
}

impl List {
    /// Takes one chunk heard at `frame`; true when the list changed.
    pub fn take(&mut self, chunk: &Chunk, frame: u32) -> bool {
        // Remember when, and with which checksum, the mod was heard.
        let known = match self.seen.iter_mut().find(|(slug, _, _)| slug == chunk.slug) {
            Some(seen) => {
                seen.1 = frame;
                seen.2 == chunk.checksum && self.entries.iter().any(|e| e.slug == chunk.slug)
            }
            None => {
                if self.seen.len() >= MAX_MODS {
                    return false;
                }
                self.seen.push((chunk.slug.to_owned(), frame, u32::MAX));
                false
            }
        };
        if known {
            return false;
        }
        let at = match self.partial.iter().position(|p| p.slug == chunk.slug) {
            Some(at) => at,
            None => {
                self.partial.push(Partial { slug: chunk.slug.to_owned(), checksum: chunk.checksum, parts: Vec::new() });
                self.partial.len() - 1
            }
        };
        let partial = &mut self.partial[at];
        if partial.checksum != chunk.checksum || partial.parts.len() != chunk.count {
            partial.checksum = chunk.checksum;
            partial.parts = vec![None; chunk.count];
        }
        partial.parts[chunk.index] = Some(chunk.bytes);
        if partial.parts.iter().any(Option::is_none) {
            return false;
        }
        let bytes: Vec<u8> = partial.parts.iter().flatten().flatten().copied().collect();
        let partial = self.partial.swap_remove(at);
        let Some((name, version, description)) = mods_core::parse(bytes, partial.checksum) else {
            return false;
        };
        if let Some(seen) = self.seen.iter_mut().find(|(slug, _, _)| *slug == partial.slug) {
            seen.2 = partial.checksum;
        }
        let entry = Entry { slug: partial.slug, name, version, description };
        match self.entries.iter_mut().find(|e| e.slug == entry.slug) {
            Some(old) => *old = entry,
            None => self.entries.push(entry),
        }
        // By name, ignoring case; compared byte by byte (no lowercase copies, which cost fuel).
        self.entries.sort_by(|a, b| {
            let key = |e: &Entry| e.name.bytes().map(|c| c.to_ascii_lowercase()).collect::<Vec<u8>>();
            key(a).cmp(&key(b)).then_with(|| a.slug.cmp(&b.slug))
        });
        true
    }

    /// Drops mods not heard for a while; true when the list changed.
    pub fn forget(&mut self, frame: u32) -> bool {
        let stale: Vec<String> = self
            .seen
            .iter()
            .filter(|(_, last, _)| frame.wrapping_sub(*last) > FORGET_AFTER)
            .map(|(slug, _, _)| slug.clone())
            .collect();
        if stale.is_empty() {
            return false;
        }
        self.seen.retain(|(slug, _, _)| !stale.contains(slug));
        self.partial.retain(|p| !stale.contains(&p.slug));
        let before = self.entries.len();
        self.entries.retain(|e| !stale.contains(&e.slug));
        self.entries.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods_core::{chunks, read_chunk, text};

    fn send(list: &mut List, slug: &str, name: &str, frame: u32) -> bool {
        let mut changed = false;
        for values in chunks(&text(name, "1.0", "A mod.")) {
            let cue = format!("mcore.t.{slug}");
            changed |= list.take(&read_chunk(&cue, &values).unwrap(), frame);
        }
        changed
    }

    #[test]
    fn mods_come_sorted_and_go_when_silent() {
        let mut list = List::default();
        assert!(send(&mut list, "zoomify", "Zoomify", 1));
        assert!(send(&mut list, "day-counter", "Day Counter", 1));
        assert!(!send(&mut list, "zoomify", "Zoomify", 2), "the same text changes nothing");
        assert_eq!(list.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["Day Counter", "Zoomify"]);
        assert!(send(&mut list, "zoomify", "Zoomify Plus", 3), "a new text replaces the old");
        assert!(!list.forget(100));
        send(&mut list, "zoomify", "Zoomify Plus", 200);
        assert!(list.forget(300));
        assert_eq!(list.entries.len(), 1);
        assert_eq!(list.entries[0].slug, "zoomify");
    }
}
