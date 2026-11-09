//! Folders that file access is kept inside (`uses Fs("./data")`, spec
//! 8.1): one check for the interpreter and native code.
//!
//! A path is turned into the absolute path it really names: the part that
//! exists is resolved by the operating system, so `..` and symbolic links
//! are followed, and the part that doesn't exist yet (a file about to be
//! written) is added after it. A path is inside a folder when the folder,
//! resolved the same way, starts it.

use std::path::{Component, Path, PathBuf};

/// The absolute path `path` names, `..` and symbolic links resolved as far
/// as the path exists.
pub fn resolve(path: &str) -> PathBuf {
    let p = Path::new(path);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    };
    let parts: Vec<Component> = abs.components().collect();
    for n in (1..=parts.len()).rev() {
        let existing: PathBuf = parts[..n].iter().collect();
        if let Ok(mut out) = existing.canonicalize() {
            for c in &parts[n..] {
                match c {
                    Component::ParentDir => {
                        out.pop();
                    }
                    Component::Normal(name) => out.push(name),
                    _ => {}
                }
            }
            return out;
        }
    }
    abs
}

/// Whether `path` is inside every folder of `scopes` (each `resolve`d).
pub fn inside(scopes: &[PathBuf], path: &str) -> bool {
    if scopes.is_empty() {
        return true;
    }
    let p = resolve(path);
    scopes.iter().all(|s| p.starts_with(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_stay_inside_their_folder() {
        let base = std::env::temp_dir().join(format!("kavach-scope-{}", std::process::id()));
        let data = base.join("data");
        std::fs::create_dir_all(data.join("inner")).unwrap();
        let scopes = [resolve(data.to_str().unwrap())];
        let at = |rest: &str| format!("{}/{rest}", data.display());
        assert!(inside(&scopes, &at("a.txt")));
        assert!(inside(&scopes, &at("inner/../b.txt")));
        assert!(inside(&scopes, &at("new/folder/c.txt")));
        assert!(!inside(&scopes, &at("../secrets.txt")));
        assert!(!inside(&scopes, &at("inner/../../x")));
        assert!(!inside(&scopes, &at("new/../../y")));
        // Folders nest: a path must be in all of them.
        let both = [scopes[0].clone(), resolve(&at("inner"))];
        assert!(inside(&both, &at("inner/d.txt")));
        assert!(!inside(&both, &at("e.txt")));
        std::fs::remove_dir_all(&base).unwrap();
    }
}
