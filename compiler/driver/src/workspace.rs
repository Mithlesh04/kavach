//! Workspaces (spec 10.14, decision 0091): a root `kavach.toml` with
//! `[workspace] members = ["packages/*"]` makes the listed folders one
//! workspace, with one `kavach.lock` and one `kavach_modules/` at the
//! root for all of them.

use std::path::{Path, PathBuf};

/// The `members` the `kavach.toml` in `root` lists, if it has a
/// `[workspace]`.
fn listed(root: &Path) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(root.join("kavach.toml")).ok()?;
    let table = text.parse::<toml::Table>().ok()?;
    let members = table.get("workspace")?.get("members")?.as_array()?;
    Some(
        members
            .iter()
            .filter_map(|m| m.as_str().map(str::to_string))
            .collect(),
    )
}

/// Whether folder `name` matches `pattern`, where `*` stands for any text.
fn matches(pattern: &str, name: &str) -> bool {
    let Some((before, after)) = pattern.split_once('*') else {
        return pattern == name;
    };
    let Some(rest) = name.strip_prefix(before) else {
        return false;
    };
    (0..=rest.len())
        .filter(|i| rest.is_char_boundary(*i))
        .any(|i| matches(after, &rest[i..]))
}

/// The member folders of the workspace whose root is `root`, in the order
/// `members` lists them; an error for a member that isn't a package.
pub fn members(root: &Path) -> Result<Vec<PathBuf>, String> {
    let Some(patterns) = listed(root) else {
        return Ok(Vec::new());
    };
    let mut out: Vec<PathBuf> = Vec::new();
    for pattern in patterns {
        if pattern.starts_with('/') || pattern.split('/').any(|p| p == "..") {
            return Err(format!(
                "workspace member `{pattern}` in {}: write a folder inside the workspace (spec 10.14)",
                root.join("kavach.toml").display()
            ));
        }
        let mut found = vec![root.to_path_buf()];
        for part in pattern.split('/').filter(|p| !p.is_empty() && *p != ".") {
            let mut next = Vec::new();
            for dir in &found {
                if !part.contains('*') {
                    next.push(dir.join(part));
                    continue;
                }
                let mut names: Vec<String> = std::fs::read_dir(dir)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| matches(part, n))
                    .collect();
                names.sort();
                next.extend(names.into_iter().map(|n| dir.join(n)));
            }
            found = next;
        }
        let wild = pattern.contains('*');
        for dir in found {
            if !dir.join("kavach.toml").is_file() {
                if wild {
                    continue;
                }
                return Err(format!(
                    "workspace member `{pattern}` has no kavach.toml (spec 10.14)"
                ));
            }
            if !out.contains(&dir) {
                out.push(dir);
            }
        }
    }
    Ok(out)
}

/// The folder that holds the `kavach.lock` and `kavach_modules/` of the
/// project at `project`: the root of the workspace it is a member of, or
/// the project itself.
pub fn home(project: &Path) -> PathBuf {
    let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let me = canonical(project);
    for dir in project.ancestors().skip(1) {
        if listed(dir).is_none() {
            continue;
        }
        if members(dir)
            .unwrap_or_default()
            .iter()
            .any(|m| canonical(m) == me)
        {
            return dir.to_path_buf();
        }
    }
    project.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn stars_stand_for_any_text() {
        assert!(matches("*", "router"));
        assert!(matches("rath-*", "rath-router"));
        assert!(matches("*-plugin", "auth-plugin"));
        assert!(matches("a*b*c", "aXbYc"));
        assert!(!matches("rath-*", "router"));
        assert!(!matches("router", "routers"));
    }
}
