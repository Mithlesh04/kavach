//! `llms.txt` (spec 13.8) stays small and correct: under 4,000 tokens,
//! every Kavach code block in it parses, and none teaches a habit the
//! compiler refuses.

use std::path::Path;

#[test]
fn llms_txt_is_small_and_its_code_is_current() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../llms.txt");
    let text = std::fs::read_to_string(path).expect("llms.txt at the repository root");
    let tokens = kavach_ai::estimate_tokens(&text);
    assert!(
        tokens < 4_000,
        "llms.txt is about {tokens} tokens; the limit is 4,000"
    );

    let blocks: Vec<&str> = text
        .split("```kavach\n")
        .skip(1)
        .filter_map(|b| b.split("```").next())
        .collect();
    assert!(blocks.len() >= 6, "found only {} code blocks", blocks.len());
    for block in &blocks {
        let parsed = kavach_parser::parse(block);
        assert!(
            parsed.errors.is_empty(),
            "syntax errors {:?} in:\n{block}",
            parsed.errors
        );
    }

    // A block is a fragment, so names it doesn't declare are expected; but
    // a habit the compiler refuses (a variant without its type, a removed
    // function) would teach every model the wrong code.
    let refused = ["decision 0070", "is gone", "is not Kavach", "Kavach has no"];
    for block in &blocks {
        let compiled = kavach_driver::compile(block, "main.kav", Default::default());
        let wrong: Vec<&str> = compiled
            .messages
            .iter()
            .map(|m| m.text.as_str())
            .filter(|t| refused.iter().any(|r| t.contains(r)))
            .collect();
        assert!(wrong.is_empty(), "llms.txt teaches {wrong:?} in:\n{block}");
    }
    let prose = ["Int.parse"];
    for gone in prose {
        assert!(!text.contains(gone), "llms.txt still teaches `{gone}`");
    }
}
