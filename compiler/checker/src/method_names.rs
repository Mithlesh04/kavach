//! The name of every built-in method (spec 14.2), so tools such as code
//! completion can list what a value has: [`crate::builtin_methods`] keeps
//! the names the checker accepts for a type. A test below checks this
//! list against the methods `builtins.rs` and `stdlib.rs` type.

pub(crate) const NAMES: &[&str] = &[
    "abs",
    "accept",
    "accept_tls",
    "add",
    "address",
    "all",
    "any",
    "as_list",
    "bits",
    "bytes",
    "cancel",
    "captures",
    "chars",
    "checked_add",
    "checked_div",
    "checked_mul",
    "checked_sub",
    "clamp",
    "client_cert",
    "close",
    "contains",
    "contains_key",
    "count",
    "div_floor",
    "ends_with",
    "enumerate",
    "filter",
    "find",
    "find_all",
    "first",
    "flat_map",
    "get",
    "index_of",
    "is_empty",
    "is_match",
    "is_none",
    "is_some",
    "join",
    "keys",
    "last",
    "len",
    "lines",
    "lower",
    "map",
    "max",
    "min",
    "mod",
    "name",
    "named_captures",
    "near",
    "open_stream",
    "parallel_map",
    "peer",
    "peer_certificate",
    "protocol",
    "push",
    "put",
    "read",
    "read_stream",
    "receive",
    "remove",
    "repeat",
    "replace",
    "replace_all",
    "reverse",
    "round",
    "round_even",
    "send",
    "shared",
    "skip",
    "slice",
    "sort",
    "sort_by",
    "split",
    "start_tls",
    "starts_with",
    "sum",
    "take",
    "text",
    "to_list",
    "trim",
    "try_put",
    "unwrap",
    "update",
    "update_with",
    "upper",
    "value",
    "values",
    "wait",
    "weak",
    "wrapping_add",
    "wrapping_mul",
    "wrapping_sub",
    "write",
    "write_stream",
    "xxhash64",
];

#[cfg(test)]
mod tests {
    use super::NAMES;

    /// The names in the match arms of function `name` in `src`.
    fn arms(src: &str, name: &str) -> Vec<String> {
        let start = src.find(&format!("fn {name}(")).expect("the function");
        let body = &src[start..];
        let end = body[1..]
            .find("\n    pub(crate) fn ")
            .map_or(body.len(), |e| e + 1);
        let mut out = Vec::new();
        for line in body[..end].lines() {
            let line = line.trim_start();
            let Some((pattern, _)) = line.split_once("=>") else {
                continue;
            };
            if !line.starts_with('"') {
                continue;
            }
            for word in pattern.split('|') {
                let word = word.trim();
                if let Some(w) = word.strip_prefix('"').and_then(|w| w.split('"').next()) {
                    out.push(w.to_string());
                }
            }
        }
        out
    }

    #[test]
    fn every_built_in_method_is_listed() {
        let found = arms(include_str!("builtin_methods.rs"), "builtin_method")
            .into_iter()
            .chain(arms(include_str!("more_methods.rs"), "more_methods"))
            .chain(arms(include_str!("enums.rs"), "enum_method"));
        let mut missing: Vec<String> = found.filter(|n| !NAMES.contains(&n.as_str())).collect();
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "add to NAMES: {missing:?}");
        assert!(NAMES.windows(2).all(|w| w[0] < w[1]), "keep NAMES sorted");
    }
}
