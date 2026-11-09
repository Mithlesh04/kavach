//! The short form of an answer's type, written into the prompt of an
//! `ask`'s first try (decision 0057): made from the same JSON Schema the
//! answer is checked against, so it can't say something else. A record
//! is `{"name": type (note), "maybe"?: type}`, a choice `"A"|"B"`, a
//! list `[type, …]`.

use kavach_runtime_core::json::Tree;

/// What the prompt ends with.
pub fn instruction(schema: &Tree) -> String {
    format!("Answer with only this JSON: {}", short(schema))
}

fn field<'t>(t: &'t Tree, name: &str) -> Option<&'t Tree> {
    match t {
        Tree::Object(fs) => fs.iter().find(|(k, _)| k == name).map(|(_, v)| v),
        _ => None,
    }
}

fn text(t: Option<&Tree>) -> Option<&str> {
    match t {
        Some(Tree::Text(s)) => Some(s),
        _ => None,
    }
}

fn short(t: &Tree) -> String {
    let base = shape(t);
    match text(field(t, "description")) {
        Some(note) => format!("{base} ({note})"),
        None => base,
    }
}

fn shape(t: &Tree) -> String {
    if let Some(Tree::Array(options)) = field(t, "anyOf") {
        let null = |o: &Tree| text(field(o, "type")) == Some("null");
        let parts: Vec<String> = options.iter().filter(|o| !null(o)).map(short).collect();
        let joined = parts.join(" | ");
        return match options.iter().any(null) {
            true => format!("{joined} or null"),
            false => joined,
        };
    }
    if let Some(Tree::Array(choices)) = field(t, "enum") {
        let quoted: Vec<String> = choices
            .iter()
            .filter_map(|c| text(Some(c)).map(|s| format!("{s:?}")))
            .collect();
        return quoted.join("|");
    }
    match text(field(t, "type")) {
        Some("object") => object(t),
        Some("array") => {
            let item = field(t, "items").map_or("any".to_string(), short);
            format!("[{item}, …]")
        }
        Some("integer") => "int".into(),
        Some("number") => "number".into(),
        Some("boolean") => "bool".into(),
        Some("string") => "string".into(),
        Some("null") => "null".into(),
        _ => "any".into(),
    }
}

fn object(t: &Tree) -> String {
    let Some(Tree::Object(properties)) = field(t, "properties") else {
        // A map with text keys.
        let value = field(t, "additionalProperties").map_or("any".to_string(), short);
        return format!("{{string: {value}}}");
    };
    let required: Vec<&str> = match field(t, "required") {
        Some(Tree::Array(names)) => names.iter().filter_map(|n| text(Some(n))).collect(),
        _ => Vec::new(),
    };
    let parts: Vec<String> = properties
        .iter()
        .map(|(name, v)| {
            let maybe = if required.contains(&name.as_str()) {
                ""
            } else {
                "?"
            };
            format!("{name:?}{maybe}: {}", short(v))
        })
        .collect();
    format!("{{{}}}", parts.join(", "))
}

/// The JSON in a reply written as text: the outermost object, without the
/// fences or words a model may put around it.
pub fn json_in(reply: &str) -> Option<&str> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    (start < end).then(|| &reply[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use kavach_runtime_core::json::{Limits, parse};

    #[test]
    fn a_record_with_a_choice_an_optional_field_and_a_list() {
        let schema = parse(
            r#"{"type":"object","properties":{
                "kind":{"type":"string","enum":["Billing","Bug"],"description":"what it is about"},
                "urgency":{"type":"integer"},
                "summary":{"anyOf":[{"type":"string"},{"type":"null"}]},
                "tags":{"type":"array","items":{"type":"string"}}},
              "required":["kind","urgency","tags"],"additionalProperties":false}"#,
            &Limits::default(),
        )
        .unwrap();
        assert_eq!(
            instruction(&schema),
            r#"Answer with only this JSON: {"kind": "Billing"|"Bug" (what it is about), "urgency": int, "summary"?: string or null, "tags": [string, …]}"#
        );
    }

    #[test]
    fn the_json_is_found_inside_fences() {
        assert_eq!(json_in("```json\n{\"a\": 1}\n```"), Some("{\"a\": 1}"));
        assert_eq!(json_in("no json"), None);
    }
}
