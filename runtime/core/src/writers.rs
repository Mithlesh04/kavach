//! Files written piece by piece (`fs.create_writer`, spec 14.3): open
//! writers by number, for the interpreter and native code. Writes are
//! gathered in memory and go to the file 64 KB at a time; `finish` writes
//! the rest and closes it, and a writer let go before `finish` closes its
//! file and deletes it.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// How much is gathered before it goes to the file.
const BUFFER: usize = 64 * 1024;

struct Writer {
    path: PathBuf,
    /// `None` once finished.
    file: Option<BufWriter<File>>,
    written: i64,
}

fn writers() -> MutexGuard<'static, HashMap<i64, Writer>> {
    static WRITERS: OnceLock<Mutex<HashMap<i64, Writer>>> = OnceLock::new();
    WRITERS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn finished() -> std::io::Error {
    std::io::Error::other("finished")
}

/// A new, empty file at `path` to write (one there is emptied): its
/// writer's number.
pub fn open(path: &str) -> std::io::Result<i64> {
    let file = File::create(path)?;
    static NEXT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let writer = Writer {
        path: PathBuf::from(path),
        file: Some(BufWriter::with_capacity(BUFFER, file)),
        written: 0,
    };
    writers().insert(id, writer);
    Ok(id)
}

/// The path writer `id` writes, for its errors.
pub fn path(id: i64) -> String {
    writers()
        .get(&id)
        .map(|w| w.path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Adds `data` at the end of writer `id`'s file.
pub fn write(id: i64, data: &[u8]) -> std::io::Result<()> {
    let mut all = writers();
    let w = all.get_mut(&id).ok_or_else(finished)?;
    let file = w.file.as_mut().ok_or_else(finished)?;
    file.write_all(data)?;
    w.written += data.len() as i64;
    Ok(())
}

/// Writes what is gathered, closes the file (with `sync`, forcing it to
/// the disk first), and gives how many bytes it holds; the same again
/// once finished.
pub fn finish(id: i64, sync: bool) -> std::io::Result<i64> {
    let mut all = writers();
    let w = all.get_mut(&id).ok_or_else(finished)?;
    if let Some(file) = w.file.take() {
        let file = file.into_inner().map_err(|e| e.into_error())?;
        if sync {
            file.sync_all()?;
        }
    }
    Ok(w.written)
}

/// Lets writer `id` go: a file not finished is closed and deleted.
pub fn close(id: i64) {
    let Some(w) = writers().remove(&id) else {
        return;
    };
    if let Some(file) = w.file {
        drop(file);
        let _ = std::fs::remove_file(&w.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_make_the_file_and_an_unfinished_one_is_deleted() {
        let dir = std::env::temp_dir().join(format!("kavach-writers-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let done = dir.join("done.bin");
        let id = open(done.to_str().unwrap()).unwrap();
        write(id, b"abc").unwrap();
        write(id, &vec![7u8; 100_000]).unwrap();
        assert_eq!(finish(id, true).unwrap(), 100_003);
        assert_eq!(finish(id, false).unwrap(), 100_003);
        assert_eq!(write(id, b"more").unwrap_err().to_string(), "finished");
        close(id);
        assert_eq!(std::fs::read(&done).unwrap().len(), 100_003);
        let left = dir.join("left.bin");
        let id = open(left.to_str().unwrap()).unwrap();
        write(id, b"half").unwrap();
        close(id);
        assert!(!left.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
