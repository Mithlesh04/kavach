//! Property test without external crates: lex thousands of random inputs
//! built from tricky pieces and check what must always hold.

use kavach_lexer::{TokenKind, lex};

const PIECES: &[&str] = &[
    "a",
    "Z",
    "_",
    "नाम",
    "्",
    "1",
    "0x",
    "0b",
    "9",
    "e",
    ".",
    "..",
    "\"",
    "\"\"\"",
    "r\"",
    "'",
    "\\",
    "\\u{",
    "41",
    "}",
    "{",
    "(",
    ")",
    "[",
    "]",
    "\n",
    " ",
    "\t",
    "\r",
    "//",
    "///",
    "/*",
    "*/",
    ";",
    "+",
    "-",
    "=",
    ">",
    "<",
    "?",
    "!",
    "€",
    "😀",
    ",",
    ":",
    "let",
    "fn",
];

/// A small deterministic xorshift generator, so failures can be reproduced.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as usize
    }
}

#[test]
fn random_input_never_breaks_the_lexer() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for case in 0..3000 {
        let len = rng.next() % 40;
        let src: String = (0..len)
            .map(|_| PIECES[rng.next() % PIECES.len()])
            .collect();
        let lexed = lex(&src);

        let last = lexed.tokens.last().expect("at least Eof");
        assert_eq!(last.kind, TokenKind::Eof, "case {case}: {src:?}");
        let mut previous_start = 0;
        for t in &lexed.tokens {
            let (start, end) = (t.span.start as usize, t.span.end as usize);
            assert!(
                start <= end && end <= src.len(),
                "case {case}: bad span in {src:?}"
            );
            assert!(
                src.get(start..end).is_some(),
                "case {case}: span splits a character in {src:?}"
            );
            assert!(
                start >= previous_start,
                "case {case}: tokens out of order in {src:?}"
            );
            previous_start = start;
        }
        for e in &lexed.errors {
            assert!(
                e.span.end as usize <= src.len(),
                "case {case}: bad error span in {src:?}"
            );
        }
        let starts = lexed
            .tokens
            .iter()
            .filter(|t| t.kind == TokenKind::StrStart)
            .count();
        let ends = lexed
            .tokens
            .iter()
            .filter(|t| t.kind == TokenKind::StrEnd)
            .count();
        let open_interpolations = lexed
            .errors
            .iter()
            .filter(|e| e.message.contains("inside a string"))
            .count();
        assert_eq!(
            starts,
            ends + open_interpolations,
            "case {case}: unbalanced strings in {src:?}"
        );
    }
}
