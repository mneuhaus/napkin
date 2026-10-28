//! Notes as plain UTF-8 `.numi` files in one folder.
//!
//! New notes get a timestamp file name (`note-20261028-130000.numi`) and take their title from
//! their first non-empty line. Renaming a note renames its file, and a note whose file name is
//! not a timestamp name is titled by its file name. Notes are ordered by creation time (newest
//! first), which is why saving writes in place instead of replacing the file.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Context as _;

/// File extension of notes.
pub(crate) const EXTENSION: &str = "numi";
/// Prefix of generated note file names.
const AUTO_PREFIX: &str = "note-";
/// Titles longer than this are cut.
const MAX_TITLE_CHARS: usize = 60;
/// Title of a note without any text.
pub(crate) const UNTITLED: &str = "Untitled";

/// One note file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Note {
    /// File name, the note's identity within the store (`Budget.numi`).
    pub(crate) file_name: String,
    pub(crate) title: String,
    /// The note's text, for search and previews.
    pub(crate) text: String,
    pub(crate) created: SystemTime,
    pub(crate) modified: SystemTime,
}

impl Note {
    /// Whether the note was renamed by the user (its title is its file name).
    pub(crate) fn is_renamed(&self) -> bool {
        !is_auto_name(stem(&self.file_name))
    }
}

/// The notes folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoteStore {
    dir: PathBuf,
}

impl NoteStore {
    pub(crate) fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    pub(crate) fn path(&self, file_name: &str) -> PathBuf {
        self.dir.join(file_name)
    }

    /// Every note, newest first.
    pub(crate) fn list(&self) -> Vec<Note> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut notes: Vec<Note> = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == EXTENSION)
            })
            .filter_map(|entry| {
                let file_name = entry.file_name().into_string().ok()?;
                let metadata = entry.metadata().ok()?;
                if !metadata.is_file() {
                    return None;
                }
                let text = fs::read_to_string(entry.path()).unwrap_or_default();
                let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let created = metadata.created().unwrap_or(modified);
                Some(Note {
                    title: title_for(&file_name, &text),
                    file_name,
                    text,
                    created,
                    modified,
                })
            })
            .collect();
        notes.sort_by(|a, b| {
            b.created
                .cmp(&a.created)
                .then_with(|| a.file_name.cmp(&b.file_name))
        });
        notes
    }

    pub(crate) fn read(&self, file_name: &str) -> io::Result<String> {
        fs::read_to_string(self.path(file_name))
    }

    /// Writes the note in place (keeping its creation time, which orders the notes).
    pub(crate) fn save(&self, file_name: &str, text: &str) -> anyhow::Result<()> {
        fs::create_dir_all(&self.dir).with_context(|| format!("creating {}", self.dir.display()))?;
        let path = self.path(file_name);
        fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
    }

    /// Creates a note with a fresh timestamp name and returns its file name.
    pub(crate) fn create(&self, text: &str) -> anyhow::Result<String> {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let file_name = self.unused_name(&format!("{AUTO_PREFIX}{stamp}"));
        self.save(&file_name, text)?;
        Ok(file_name)
    }

    /// Renames a note to `title` and returns its new file name. An empty title gives the note
    /// its automatic title back.
    pub(crate) fn rename(&self, file_name: &str, title: &str) -> anyhow::Result<String> {
        let wanted = sanitize(title);
        let base = if wanted.is_empty() {
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
            format!("{AUTO_PREFIX}{stamp}")
        } else {
            wanted
        };
        if stem(file_name) == base {
            return Ok(file_name.to_owned());
        }
        let new_name = self.unused_name(&base);
        let (from, to) = (self.path(file_name), self.path(&new_name));
        fs::rename(&from, &to).with_context(|| format!("renaming {} to {}", from.display(), to.display()))?;
        Ok(new_name)
    }

    /// Deletes a note: into the Trash on macOS, so a mistake can be undone in Finder.
    pub(crate) fn delete(&self, file_name: &str) -> anyhow::Result<()> {
        let path = self.path(file_name);
        if crate::platform::move_to_trash(&path) {
            return Ok(());
        }
        fs::remove_file(&path).with_context(|| format!("deleting {}", path.display()))
    }

    /// `base.numi`, or `base 2.numi`, `base 3.numi`, ... when taken.
    fn unused_name(&self, base: &str) -> String {
        let mut candidate = format!("{base}.{EXTENSION}");
        let mut counter = 2;
        while self.path(&candidate).exists() {
            candidate = format!("{base} {counter}.{EXTENSION}");
            counter += 1;
        }
        candidate
    }
}

/// The file name without its extension.
pub(crate) fn stem(file_name: &str) -> &str {
    file_name
        .strip_suffix(&format!(".{EXTENSION}"))
        .unwrap_or(file_name)
}

/// Whether `stem` is a generated name (`note-20261028-130000`, maybe with ` 2`).
fn is_auto_name(stem: &str) -> bool {
    let Some(rest) = stem.strip_prefix(AUTO_PREFIX) else {
        return false;
    };
    let stamp = rest.split(' ').next().unwrap_or(rest);
    let bytes = stamp.as_bytes();
    bytes.len() == 15
        && bytes[8] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 8 || byte.is_ascii_digit())
}

/// The title of a note: its file name when renamed, else its first non-empty line.
pub(crate) fn title_for(file_name: &str, text: &str) -> String {
    let stem = stem(file_name);
    if !is_auto_name(stem) {
        return stem.to_owned();
    }
    first_line_title(text)
}

/// The first non-empty line, without a header's `#`, cut to a readable length.
pub(crate) fn first_line_title(text: &str) -> String {
    let Some(line) = text.lines().map(str::trim).find(|line| !line.is_empty()) else {
        return UNTITLED.to_owned();
    };
    let line = line.trim_start_matches('#').trim();
    if line.is_empty() {
        return UNTITLED.to_owned();
    }
    if line.chars().count() <= MAX_TITLE_CHARS {
        return line.to_owned();
    }
    let cut: String = line.chars().take(MAX_TITLE_CHARS - 1).collect();
    format!("{}…", cut.trim_end())
}

/// A title made safe as a file name.
fn sanitize(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|character| {
            if matches!(character, '/' | ':' | '\\') || character.is_control() {
                '-'
            } else {
                character
            }
        })
        .collect();
    cleaned.trim().trim_start_matches('.').trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(name: &str) -> NoteStore {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/test-notes")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        NoteStore::new(dir)
    }

    #[test]
    fn titles_come_from_the_first_non_empty_line() {
        assert_eq!(
            title_for("note-20261028-130000.numi", "\n\n# Trip  \n1 + 2"),
            "Trip"
        );
        assert_eq!(
            title_for("note-20261028-130000 2.numi", "  20 inches in cm"),
            "20 inches in cm"
        );
        assert_eq!(title_for("note-20261028-130000.numi", "  \n "), UNTITLED);
        assert_eq!(title_for("Budget.numi", "# Something else"), "Budget");
    }

    #[test]
    fn long_titles_are_cut() {
        let title = first_line_title(&"x".repeat(100));
        assert_eq!(title.chars().count(), MAX_TITLE_CHARS);
        assert!(title.ends_with('…'));
    }

    #[test]
    fn create_save_rename_and_list() {
        let store = store("lifecycle");
        let first = store.create("# First\n1").expect("created");
        let second = store.create("2").expect("created");
        assert_ne!(first, second, "names stay unique within one second");
        store.save(&second, "# Second\n2").expect("saved");
        let renamed = store.rename(&second, "Budget: 2026").expect("renamed");
        assert_eq!(renamed, "Budget- 2026.numi");

        let notes = store.list();
        let titles: Vec<&str> = notes.iter().map(|note| note.title.as_str()).collect();
        assert_eq!(titles, ["Budget- 2026", "First"]);
        assert!(notes[0].is_renamed());
        assert!(!notes[1].is_renamed());
        assert_eq!(store.read(&renamed).expect("read"), "# Second\n2");
    }

    #[test]
    fn renaming_to_an_empty_title_restores_the_automatic_title() {
        let store = store("unrename");
        let file = store.create("# Auto\n1").expect("created");
        let renamed = store.rename(&file, "Named").expect("renamed");
        let restored = store.rename(&renamed, "  ").expect("restored");
        assert_eq!(title_for(&restored, "# Auto\n1"), "Auto");
    }

    #[test]
    fn taken_names_get_a_counter() {
        let store = store("counter");
        let one = store.create("a").expect("created");
        let two = store.create("b").expect("created");
        let a = store.rename(&one, "Same").expect("renamed");
        let b = store.rename(&two, "Same").expect("renamed");
        assert_eq!((a.as_str(), b.as_str()), ("Same.numi", "Same 2.numi"));
    }

    #[test]
    fn missing_folder_lists_nothing() {
        assert!(store("missing").list().is_empty());
    }
}
