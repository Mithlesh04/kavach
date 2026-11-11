//! What a dependency may name (spec 10.12): sources that are safe to
//! fetch, commits that can't be read as options, and folder names that
//! every system can make. Used by `deps.rs`.

/// Why `url` can't be fetched, if it can't: only `https://`, `ssh://`,
/// `user@host:path` and local folders, so nothing travels unencrypted,
/// no transport runs a command, and no password lands in `kavach.toml`
/// or `kavach.lock`, which are committed.
pub(crate) fn unsafe_url(url: &str) -> Option<&'static str> {
    let lower = url.to_ascii_lowercase();
    if url.starts_with('-') || url.chars().any(char::is_control) {
        return Some("it would be read as an option of git");
    }
    if lower.contains("::")
        || ["http://", "git://", "ftp://", "ftps://"]
            .iter()
            .any(|s| lower.starts_with(s))
    {
        return Some(
            "use `https://` or `ssh://`: other transports are unencrypted or run commands",
        );
    }
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("ssh://"));
    let authority = rest.map(|r| r.split('/').next().unwrap_or(""));
    if authority
        .and_then(|a| a.rsplit_once('@'))
        .is_some_and(|(user, _)| user.contains(':'))
    {
        return Some(
            "it holds a password or token, which would be committed: let git find it (a credential helper or an SSH key)",
        );
    }
    None
}

/// Whether `text` can be a commit id: hex digits only, so it is never
/// read as an option.
pub(crate) fn is_commit(text: &str) -> bool {
    (4..=64).contains(&text.len()) && text.chars().all(|c| c.is_ascii_hexdigit())
}

/// Whether Windows keeps `name` for a device (`con`, `nul`, `com1`…), so
/// `kavach_modules/<name>` couldn't be made there.
pub(crate) fn is_device_name(name: &str) -> bool {
    matches!(name, "con" | "prn" | "aux" | "nul")
        || ((name.starts_with("com") || name.starts_with("lpt"))
            && name.len() == 4
            && name.as_bytes()[3].is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_safe_sources_are_fetched() {
        let fetched = [
            "https://github.com/example/money",
            "https://github.com:443/example/money.git",
            "https://git@github.com/example/money",
            "ssh://git@github.com:22/example/money",
            "git@github.com:example/money.git",
            "../money",
            "C:/work/money",
        ];
        for url in fetched {
            assert_eq!(unsafe_url(url), None, "{url}");
        }
        let refused = [
            "--upload-pack=touch x",
            "-u x",
            "http://github.com/example/money",
            "git://github.com/example/money",
            "ext::sh -c touch% x",
            "fd::3",
            "https://user:token@github.com/example/money",
            "ssh://me:secret@host/money",
            "https://github.com/a\nb",
        ];
        for url in refused {
            assert!(unsafe_url(url).is_some(), "{url}");
        }
    }

    #[test]
    fn commits_and_device_names() {
        assert!(is_commit("445b5b6140a2ed723cb32e174c86e5295b3bd53b"));
        assert!(is_commit("445b5b6"));
        assert!(!is_commit("--upload-pack=x"));
        assert!(!is_commit("main"));
        assert!(!is_commit(""));
        for name in ["con", "nul", "aux", "prn", "com1", "lpt9"] {
            assert!(is_device_name(name), "{name}");
        }
        for name in ["console", "com", "com10", "money", "null"] {
            assert!(!is_device_name(name), "{name}");
        }
    }
}
