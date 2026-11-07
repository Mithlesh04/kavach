//! `llms.txt` (spec 13.8) stays small and correct: under 4,000 tokens, and
//! every Kavach code block in it parses.

use std::path::Path;

#[test]
fn llms_txt_is_small_and_its_code_parses() {
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
    for block in blocks {
        let parsed = kavach_parser::parse(block);
        assert!(
            parsed.errors.is_empty(),
            "syntax errors {:?} in:\n{block}",
            parsed.errors
        );
    }
}
