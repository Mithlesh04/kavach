//! A file's details and permissions, renaming, copying and removing a
//! folder with what is in it (spec 14.3), for the interpreter and native
//! code alike. Errors are `io::Error`s; each caller turns them into an
//! `FsError`.

use std::io;
use std::time::UNIX_EPOCH;

/// What is at `path`, as numbers: its kind (0 file, 1 folder, 2 link),
/// size in bytes (0 for a folder), last change in milliseconds since
/// 1970, read-only (0 or 1), and permission bits. A link is described
/// itself, not what it points to.
pub fn stat(path: &str) -> io::Result<[i64; 5]> {
    let m = std::fs::symlink_metadata(path)?;
    let t = m.file_type();
    let kind = if t.is_symlink() {
        2
    } else if t.is_dir() {
        1
    } else {
        0
    };
    let size = if t.is_dir() { 0 } else { m.len() as i64 };
    let modified = (m.modified().ok())
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64);
    let read_only = m.permissions().readonly();
    Ok([
        kind,
        size,
        modified,
        i64::from(read_only),
        bits(&m, t.is_dir()),
    ])
}

/// The Unix permission bits.
#[cfg(unix)]
fn bits(m: &std::fs::Metadata, _: bool) -> i64 {
    use std::os::unix::fs::PermissionsExt;
    i64::from(m.permissions().mode() & 0o7777)
}

/// Windows has no Unix bits: these are what Go and Rust report there.
#[cfg(not(unix))]
fn bits(m: &std::fs::Metadata, folder: bool) -> i64 {
    match (m.permissions().readonly(), folder) {
        (true, false) => 0o444,
        (false, false) => 0o666,
        (true, true) => 0o555,
        (false, true) => 0o777,
    }
}

/// Makes `path` read-only, or writable again. On Unix, writable again
/// means the owner may write: no one else gets the right by it.
pub fn set_read_only(path: &str, read_only: bool) -> io::Result<()> {
    let m = std::fs::metadata(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = m.permissions().mode() & 0o7777;
        let mode = if read_only {
            mode & !0o222
        } else {
            mode | 0o200
        };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
    }
    #[cfg(not(unix))]
    {
        let mut p = m.permissions();
        p.set_readonly(read_only);
        std::fs::set_permissions(path, p)
    }
}

/// Sets the Unix permission bits `mode` (`0o0` to `0o7777`). Windows keeps
/// only the owner's write bit: without it, the file is read-only.
pub fn set_permissions(path: &str, mode: i64) -> io::Result<()> {
    if !(0..=0o7777).contains(&mode) {
        return Err(io::Error::other("permissions are 0o0 to 0o7777"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode as u32))
    }
    #[cfg(not(unix))]
    {
        set_read_only(path, mode & 0o200 == 0)
    }
}

/// Moves or renames `from` to `to`, replacing a file there.
pub fn rename(from: &str, to: &str) -> io::Result<()> {
    std::fs::rename(from, to)
}

/// Copies the file `from` to `to`, with its permissions, replacing a file
/// there.
pub fn copy(from: &str, to: &str) -> io::Result<()> {
    if std::fs::metadata(from)?.is_dir() {
        return Err(io::Error::other("a folder: copy its files one by one"));
    }
    std::fs::copy(from, to).map(|_| ())
}

/// A symbolic link at `link` to `target`, kept as written. Windows makes
/// one only in Developer Mode or with administrator rights.
pub fn symlink(target: &str, link: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        // A relative target is relative to the link's folder.
        let folder = std::path::Path::new(link)
            .parent()
            .unwrap_or(std::path::Path::new(""));
        let made = if folder.join(target).is_dir() {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        };
        // Without the rights to (error 1314), it is a refusal: `Denied`.
        made.map_err(|e| match e.raw_os_error() {
            Some(1314) => io::Error::new(io::ErrorKind::PermissionDenied, e),
            _ => e,
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        Err(io::Error::other("links are not made on this system"))
    }
}

/// Deletes a folder and everything in it, or a file or link.
pub fn remove_all(path: &str) -> io::Result<()> {
    if std::fs::symlink_metadata(path)?.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_permissions_and_moves() {
        let dir = std::env::temp_dir().join(format!("kavach-file-info-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let file = dir.join("a.txt").to_string_lossy().to_string();
        std::fs::write(&file, "hello").unwrap();

        let [kind, size, modified, read_only, _] = stat(&file).unwrap();
        assert_eq!((kind, size, read_only), (0, 5, 0));
        assert!(modified > 1_700_000_000_000, "{modified}");
        let folder = stat(&dir.join("sub").to_string_lossy()).unwrap();
        assert_eq!((folder[0], folder[1]), (1, 0));

        set_read_only(&file, true).unwrap();
        assert_eq!(stat(&file).unwrap()[3], 1);
        assert!(std::fs::write(&file, "x").is_err());
        set_read_only(&file, false).unwrap();
        assert_eq!(stat(&file).unwrap()[3], 0);
        set_permissions(&file, 0o444).unwrap();
        assert_eq!(stat(&file).unwrap()[3], 1);
        set_permissions(&file, 0o644).unwrap();
        #[cfg(unix)]
        assert_eq!(stat(&file).unwrap()[4], 0o644);
        #[cfg(not(unix))]
        assert_eq!(stat(&file).unwrap()[4], 0o666);
        assert!(set_permissions(&file, 0o10000).is_err());

        let copied = dir.join("sub/b.txt").to_string_lossy().to_string();
        copy(&file, &copied).unwrap();
        assert_eq!(std::fs::read_to_string(&copied).unwrap(), "hello");
        let moved = dir.join("c.txt").to_string_lossy().to_string();
        rename(&file, &moved).unwrap();
        assert!(stat(&file).is_err());
        // A link to `c.txt`, read through and described itself. Windows
        // makes one only with the rights to: without them, `Denied`.
        let link = dir.join("link.txt").to_string_lossy().to_string();
        match symlink("c.txt", &link) {
            Ok(()) => {
                assert_eq!(std::fs::read_to_string(&link).unwrap(), "hello");
                assert_eq!(stat(&link).unwrap()[0], 2);
            }
            // Only Windows may refuse, for lack of the rights to.
            Err(e) if cfg!(windows) => {
                assert_eq!(e.kind(), io::ErrorKind::PermissionDenied, "{e}");
            }
            Err(e) => panic!("{e}"),
        }
        remove_all(&dir.to_string_lossy()).unwrap();
        assert!(!dir.exists());
    }
}
