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
    for dir in project.ancestors().skip(1) {
        let Some(patterns) = listed(dir) else {
            continue;
        };
        // Matched by its path, so a listed member that is broken or
        // missing doesn't hide this one.
        let Ok(rel) = project.strip_prefix(dir) else {
            continue;
        };
        let parts: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        let fits = |pattern: &String| {
            let want: Vec<&str> = pattern
                .split('/')
                .filter(|p| !p.is_empty() && *p != ".")
                .collect();
            want.len() == parts.len() && want.iter().zip(&parts).all(|(w, p)| matches(w, p))
        };
        if patterns.iter().any(fits) {
            return dir.to_path_buf();
        }
    }
    project.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn a_member_finds_its_root_though_another_member_is_missing() {
        let root = std::env::temp_dir().join(format!("kavach-ws-home-{}", std::process::id()));
        let member = root.join("packages").join("router");
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(
            root.join("kavach.toml"),
            "[workspace]
members = [\"packages/*\", \"templates/gone\"]
",
        )
        .unwrap();
        std::fs::write(
            member.join("kavach.toml"),
            "[package]
name = \"router\"
",
        )
        .unwrap();
        assert_eq!(super::home(&member), root);
        assert_eq!(super::home(&root.join("elsewhere")), root.join("elsewhere"));
        let _ = std::fs::remove_dir_all(&root);
    }

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
