//! Regular expressions (spec 14.8), shared by the interpreter and native
//! code so both match the same way.
//!
//! `regex.new` uses the linear-time engine (the `regex` crate): matching
//! time grows with the text only, so no pattern or text can stall a
//! program. `regex.advanced` also allows lookaround and backreferences
//! (the `fancy-regex` crate), with a limit on backtracking.

/// The most backtracking steps an advanced pattern may take on one text.
const BACKTRACK_LIMIT: usize = 1_000_000;

/// Matching with an advanced pattern did too much work and gave up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GaveUp;

pub type Found<T> = Result<T, GaveUp>;

enum Engine {
    Linear(regex::Regex),
    Advanced(fancy_regex::Regex),
}

pub struct Pattern {
    source: String,
    engine: Engine,
}

impl std::fmt::Debug for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Regex({:?})", self.source)
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

fn gave_up<E>(_: E) -> GaveUp {
    GaveUp
}

impl Pattern {
    /// Compiles `source`, or says what is wrong with it.
    pub fn new(source: &str, advanced: bool) -> Result<Pattern, String> {
        let engine = if advanced {
            // `fancy_regex` reads `a{3,1}` as text; the linear engine
            // refuses it, and both engines take the same patterns (14.8).
            if reversed_count(source) {
                return Err(format!(
                    "invalid pattern `{source}`: invalid repetition count range, the start must be <= the end"
                ));
            }
            fancy_regex::RegexBuilder::new(source)
                .backtrack_limit(BACKTRACK_LIMIT)
                .build()
                .map(Engine::Advanced)
                .map_err(|e| format!("invalid pattern `{source}`: {e}"))?
        } else {
            regex::Regex::new(source).map(Engine::Linear).map_err(|e| {
                let text = e.to_string();
                let hint = if text.contains("look-around") || text.contains("backreferences") {
                    "\n  lookaround and backreferences need `regex.advanced`"
                } else {
                    ""
                };
                let why = text
                    .lines()
                    .last()
                    .unwrap_or(&text)
                    .trim()
                    .trim_start_matches("error: ");
                format!("invalid pattern `{source}`: {why}{hint}")
            })?
        };
        Ok(Pattern {
            source: source.to_string(),
            engine,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn is_match(&self, text: &str) -> Found<bool> {
        match &self.engine {
            Engine::Linear(r) => Ok(r.is_match(text)),
            Engine::Advanced(r) => r.is_match(text).map_err(gave_up),
        }
    }

    pub fn find(&self, text: &str) -> Found<Option<String>> {
        match &self.engine {
            Engine::Linear(r) => Ok(r.find(text).map(|m| m.as_str().to_string())),
            Engine::Advanced(r) => Ok(r
                .find(text)
                .map_err(gave_up)?
                .map(|m| m.as_str().to_string())),
        }
    }

    pub fn find_all(&self, text: &str) -> Found<Vec<String>> {
        match &self.engine {
            Engine::Linear(r) => Ok(r.find_iter(text).map(|m| m.as_str().to_string()).collect()),
            Engine::Advanced(r) => r
                .find_iter(text)
                .map(|m| m.map(|m| m.as_str().to_string()).map_err(gave_up))
                .collect(),
        }
    }

    /// The whole match, then each group (`None` if it didn't take part).
    pub fn captures(&self, text: &str) -> Found<Option<Vec<Option<String>>>> {
        let groups = |n: usize, get: &dyn Fn(usize) -> Option<String>| (0..n).map(get).collect();
        match &self.engine {
            Engine::Linear(r) => Ok(r
                .captures(text)
                .map(|c| groups(c.len(), &|i| c.get(i).map(|m| m.as_str().to_string())))),
            Engine::Advanced(r) => Ok(r
                .captures(text)
                .map_err(gave_up)?
                .map(|c| groups(c.len(), &|i| c.get(i).map(|m| m.as_str().to_string())))),
        }
    }

    /// Each named group that took part in the first match, by name, in
    /// the order the pattern names them.
    pub fn named_captures(&self, text: &str) -> Found<Option<Vec<(String, String)>>> {
        let names: Vec<String> = match &self.engine {
            Engine::Linear(r) => r.capture_names().flatten().map(String::from).collect(),
            Engine::Advanced(r) => r.capture_names().flatten().map(String::from).collect(),
        };
        let found = match &self.engine {
            Engine::Linear(r) => r.captures(text).map(|c| {
                names
                    .iter()
                    .filter_map(|n| c.name(n).map(|m| (n.clone(), m.as_str().to_string())))
                    .collect()
            }),
            Engine::Advanced(r) => r.captures(text).map_err(gave_up)?.map(|c| {
                names
                    .iter()
                    .filter_map(|n| c.name(n).map(|m| (n.clone(), m.as_str().to_string())))
                    .collect()
            }),
        };
        Ok(found)
    }

    /// `text` with the first (or every) match replaced; `$1` and `${name}`
    /// in `with` insert groups.
    pub fn replace(&self, text: &str, with: &str, all: bool) -> Found<String> {
        match (&self.engine, all) {
            (Engine::Linear(r), false) => Ok(r.replace(text, with).into_owned()),
            (Engine::Linear(r), true) => Ok(r.replace_all(text, with).into_owned()),
            (Engine::Advanced(r), false) => r
                .try_replacen(text, 1, with)
                .map(|s| s.into_owned())
                .map_err(gave_up),
            (Engine::Advanced(r), true) => r
                .try_replacen(text, 0, with)
                .map(|s| s.into_owned())
                .map_err(gave_up),
        }
    }

    pub fn split(&self, text: &str) -> Found<Vec<String>> {
        match &self.engine {
            Engine::Linear(r) => Ok(r.split(text).map(String::from).collect()),
            Engine::Advanced(r) => r
                .split(text)
                .map(|p| p.map(String::from).map_err(gave_up))
                .collect(),
        }
    }
}

/// Whether `source` repeats something a number of times whose start is
/// more than its end, `a{3,1}`, outside a class and not escaped.
fn reversed_count(source: &str) -> bool {
    let b = source.as_bytes();
    let (mut i, mut class) = (0, false);
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'[' => class = true,
            b']' => class = false,
            b'{' if !class => {
                let rest = &source[i + 1..];
                if let Some(end) = rest.find('}')
                    && let Some((lo, hi)) = rest[..end].split_once(',')
                    && let (Ok(lo), Ok(hi)) = (lo.parse::<u64>(), hi.parse::<u64>())
                    && lo > hi
                {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_count_whose_start_passes_its_end_is_refused_by_both_engines() {
        for advanced in [false, true] {
            assert!(super::Pattern::new("a{3,1}", advanced).is_err());
            assert!(super::Pattern::new("a{1,3}", advanced).is_ok());
            assert!(super::Pattern::new("[{3,1}]", advanced).is_ok());
        }
        assert!(super::Pattern::new(r"a\{3,1}", true).is_ok());
    }

    use super::Pattern;

    #[test]
    fn both_engines_match_the_same_way() {
        for advanced in [false, true] {
            let p = Pattern::new(r"(?<y>\d{4})-(\d{2})", advanced).unwrap();
            assert_eq!(p.is_match("due 2026-10"), Ok(true));
            assert_eq!(
                p.find("a 2026-10 b 2027-01").unwrap().as_deref(),
                Some("2026-10")
            );
            assert_eq!(
                p.find_all("2026-10 2027-01").unwrap(),
                vec!["2026-10", "2027-01"]
            );
            assert_eq!(p.replace("2026-10", "$2/${y}", true).unwrap(), "10/2026");
            assert_eq!(
                p.named_captures("2026-10").unwrap(),
                Some(vec![("y".into(), "2026".into())])
            );
            let comma = Pattern::new(r"\s*,\s*", advanced).unwrap();
            assert_eq!(comma.split("a , b,c").unwrap(), vec!["a", "b", "c"]);
        }
    }

    #[test]
    fn lookaround_needs_the_advanced_engine() {
        let err = Pattern::new(r"\d+(?= kg)", false).unwrap_err();
        assert!(err.contains("regex.advanced"), "{err}");
        let p = Pattern::new(r"\d+(?= kg)", true).unwrap();
        assert_eq!(p.find("5 kg").unwrap().as_deref(), Some("5"));
        let twice = Pattern::new(r"(\w+) \1", true).unwrap();
        assert_eq!(twice.is_match("hello hello"), Ok(true));
    }

    #[test]
    fn advanced_patterns_give_up_instead_of_hanging() {
        // The lookahead needs the backtracking engine, and `(?:a|a)*`
        // makes it try every way to split the `a`s.
        let evil = Pattern::new(r"^(?:a|a)*(?!a)c", true).unwrap();
        let text = format!("{}!", "a".repeat(30));
        assert!(evil.is_match(&text).is_err());
        // The linear engine answers at once, whatever the text.
        let safe = Pattern::new(r"^(?:a|a)*c", false).unwrap();
        assert_eq!(safe.is_match(&text), Ok(false));
    }
}
