//! A project's index in `.kavach/index` (spec 13.9): used while no file
//! of the project changed, rebuilt and saved when one did.

use crate::index::Index;
use kavach_driver::{Compiled, Config, Source, compile, compile_sources, project_sources};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

/// Changes whenever the index's layout does, so an old index is rebuilt.
const FORMAT: u32 = 1;

/// Why there is no index.
#[derive(Debug)]
pub enum NoIndex {
    /// The program has errors: every message, rendered for a terminal.
    Errors(String),
    /// A file or `kavach.toml` could not be read.
    Failed(String),
}

/// The index of the program `entry` belongs to. In a project it comes
/// from `.kavach/index` when no file changed since it was written;
/// otherwise the program is checked, and the index built and saved. A
/// single file outside a project is checked every time.
pub fn index_for(entry: &str) -> Result<Index, NoIndex> {
    let path = Path::new(entry);
    let config = Config::for_file(path).map_err(NoIndex::Failed)?;
    let options = config.options_for(path);
    let (Some(sources), Some(root)) = (project_sources(path), config.root) else {
        let src = std::fs::read_to_string(path)
            .map_err(|e| NoIndex::Failed(format!("can't read {entry}: {e}")))?;
        return checked(&compile(&src, entry, options), entry);
    };
    let stamp = stamp(&root, &sources);
    let file = root.join(".kavach").join("index");
    let saved = std::fs::read_to_string(&file)
        .ok()
        .and_then(|text| serde_json::from_str::<Index>(&text).ok());
    if let Some(index) = saved.filter(|i| i.entry == entry && i.stamp == stamp) {
        return Ok(index);
    }
    let mut index = checked(&compile_sources(sources, options), entry)?;
    index.stamp = stamp;
    // Only a cache: when it can't be saved, the next call checks again.
    let _ = save(&file, &index);
    Ok(index)
}

fn checked(c: &Compiled, entry: &str) -> Result<Index, NoIndex> {
    if c.has_errors() {
        return Err(NoIndex::Errors(c.render()));
    }
    Ok(Index::build(c, entry))
}

/// The compiler version, `kavach.toml` and each file of the program,
/// with a hash of its text: when any of them differs, the index is old.
fn stamp(root: &Path, sources: &[Source]) -> Vec<(String, String)> {
    let hash = |text: &str| {
        let mut h = DefaultHasher::new();
        text.hash(&mut h);
        format!("{:016x}", h.finish())
    };
    let toml = std::fs::read_to_string(root.join("kavach.toml")).unwrap_or_default();
    let mut out = vec![
        (
            "kavach".to_string(),
            format!("{} {FORMAT}", env!("CARGO_PKG_VERSION")),
        ),
        ("kavach.toml".to_string(), hash(&toml)),
    ];
    out.extend(sources.iter().map(|s| (s.path.clone(), hash(&s.text))));
    out
}

/// Writes to a temporary file first, so that another call never reads
/// half an index.
fn save(file: &Path, index: &Index) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string(index).map_err(std::io::Error::other)?;
    let temp = file.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, file)
}
