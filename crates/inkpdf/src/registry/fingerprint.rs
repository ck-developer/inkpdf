//! Fingerprint of a template directory: (relative path, size, mtime) of each file.
//!
//! The walk reads no content: it detects changes and gives the total size before anything is
//! read.

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Maximum walk depth (protects against pathological trees).
const MAX_DEPTH: usize = 32;

/// Fingerprint of a directory.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    hash: u64,
    /// Sum of the sizes of the kept files.
    pub total_bytes: u64,
}

impl Fingerprint {
    /// Computes the fingerprint of a directory.
    pub fn of(dir: &Path) -> io::Result<Fingerprint> {
        scan(dir).map(|scan| scan.fingerprint)
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.hash)
    }
}

/// File kept by the walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    /// Path relative to the template directory, `/`-separated.
    pub relative: String,
    /// Real path (target of the link, if any), under the template directory.
    pub path: PathBuf,
    pub size: u64,
}

/// Result of a directory walk.
#[derive(Debug, Clone)]
pub struct Scan {
    pub fingerprint: Fingerprint,
    pub files: Vec<ScannedFile>,
    /// Symbolic links excluded because their target is outside the template directory.
    pub escaping_links: Vec<String>,
}

/// Walks `dir` recursively (hidden files ignored, links followed as long as their target stays
/// under `dir`) and computes the fingerprint.
pub fn scan(dir: &Path) -> io::Result<Scan> {
    let root = fs::canonicalize(dir)?;
    let mut walker = Walker {
        root: &root,
        files: Vec::new(),
        escaping_links: Vec::new(),
        visited: HashSet::new(),
    };
    walker.visited.insert(root.clone());
    walker.walk(&root, "", 0)?;

    let Walker {
        mut files,
        escaping_links,
        ..
    } = walker;
    files.sort_by(|a, b| a.relative.cmp(&b.relative));

    let mut hasher = Fnv64::new();
    let mut total_bytes = 0u64;
    for file in &files {
        let mtime = fs::metadata(&file.path)?
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        hasher.write(file.relative.as_bytes());
        hasher.write(&[0]);
        hasher.write(&file.size.to_le_bytes());
        hasher.write(&mtime.to_le_bytes());
        total_bytes += file.size;
    }

    Ok(Scan {
        fingerprint: Fingerprint {
            hash: hasher.finish(),
            total_bytes,
        },
        files,
        escaping_links,
    })
}

struct Walker<'a> {
    root: &'a Path,
    files: Vec<ScannedFile>,
    escaping_links: Vec<String>,
    visited: HashSet<PathBuf>,
}

impl Walker<'_> {
    fn walk(&mut self, dir: &Path, prefix: &str, depth: usize) -> io::Result<()> {
        if depth > MAX_DEPTH {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let relative = format!("{prefix}{name}");
            let path = entry.path();

            // `canonicalize` resolves links: the target must stay under the directory.
            let target = match fs::canonicalize(&path) {
                Ok(target) => target,
                // Broken link or file removed during the walk: ignored.
                Err(_) => continue,
            };
            if !target.starts_with(self.root) {
                self.escaping_links.push(relative);
                continue;
            }

            let metadata = fs::metadata(&target)?;
            if metadata.is_dir() {
                if self.visited.insert(target.clone()) {
                    self.walk(&target, &format!("{relative}/"), depth + 1)?;
                }
            } else if metadata.is_file() {
                self.files.push(ScannedFile {
                    relative,
                    path: target,
                    size: metadata.len(),
                });
            }
        }
        Ok(())
    }
}

/// 64-bit FNV-1a: stable across Rust versions, so usable in the PDF's deterministic `ident`.
struct Fnv64(u64);

impl Fnv64 {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::time::{Duration, SystemTime};

    fn write(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn set_mtime(path: &Path, offset_secs: u64) {
        let file = File::options().write(true).open(path).unwrap();
        file.set_modified(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000 + offset_secs),
        )
        .unwrap();
    }

    #[test]
    fn same_content_same_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "main.typ", "a");
        write(dir.path(), "parts/b.typ", "b");
        assert_eq!(
            Fingerprint::of(dir.path()).unwrap(),
            Fingerprint::of(dir.path()).unwrap()
        );
    }

    #[test]
    fn size_or_mtime_change_changes_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "main.typ", "a");
        set_mtime(&dir.path().join("main.typ"), 0);
        let before = Fingerprint::of(dir.path()).unwrap();

        set_mtime(&dir.path().join("main.typ"), 1);
        let after_mtime = Fingerprint::of(dir.path()).unwrap();
        assert_ne!(before, after_mtime);

        write(dir.path(), "main.typ", "ab");
        set_mtime(&dir.path().join("main.typ"), 1);
        let after_size = Fingerprint::of(dir.path()).unwrap();
        assert_ne!(after_mtime, after_size);
        assert_eq!(after_size.total_bytes, 2);
    }

    #[test]
    fn adding_or_removing_a_file_changes_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "main.typ", "a");
        let before = Fingerprint::of(dir.path()).unwrap();
        write(dir.path(), "assets/x.txt", "x");
        let added = Fingerprint::of(dir.path()).unwrap();
        assert_ne!(before, added);
        fs::remove_file(dir.path().join("assets/x.txt")).unwrap();
        assert_ne!(added, Fingerprint::of(dir.path()).unwrap());
    }

    #[test]
    fn creation_order_has_no_effect() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        for (dir, names) in [(&a, ["x", "y", "z"]), (&b, ["z", "y", "x"])] {
            for name in names {
                write(dir.path(), name, name);
                set_mtime(&dir.path().join(name), 0);
            }
        }
        assert_eq!(
            Fingerprint::of(a.path()).unwrap(),
            Fingerprint::of(b.path()).unwrap()
        );
    }

    #[test]
    fn hidden_files_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "main.typ", "a");
        let before = Fingerprint::of(dir.path()).unwrap();
        write(dir.path(), ".DS_Store", "junk");
        assert_eq!(before, Fingerprint::of(dir.path()).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn escaping_symlinks_are_excluded() {
        let outside = tempfile::tempdir().unwrap();
        write(outside.path(), "secret.txt", "secret");
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "main.typ", "a");
        write(dir.path(), "real.txt", "r");
        std::os::unix::fs::symlink(outside.path().join("secret.txt"), dir.path().join("x"))
            .unwrap();
        std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("inside")).unwrap();

        let scan = scan(dir.path()).unwrap();
        let names: Vec<&str> = scan.files.iter().map(|f| f.relative.as_str()).collect();
        assert_eq!(names, ["inside", "main.typ", "real.txt"]);
        assert_eq!(scan.escaping_links, ["x"]);
    }
}
