//! Tokens: the words and symbols of a Kavach program.

/// A byte range in the source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span {
            start: start as u32,
            end: end as u32,
        }
    }

    /// The source text this span covers.
    pub fn text(self, src: &str) -> &str {
        &src[self.start as usize..self.end as usize]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// What a token is. Names and number text are read from the source with
/// [`Span::text`]; only values that need decoding are stored here.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident,
    Keyword(Keyword),
    /// An integer literal, with `_` separators removed.
    Int(u64),
    /// A whole-number literal too big for 64 bits: a `BigInt` (spec 2.1).
    /// Its digits are read from the source.
    BigInt,
    /// A literal with a `.` or an exponent. Its text is kept as written,
    /// because context decides whether it is a `Float` or a `Decimal` (spec 1.7).
    Float,
    Char(char),
    /// A `///` doc comment, without the slashes and one leading space.
    DocComment(String),

    // A string is: StrStart, then any mix of Text and
    // InterpStart … InterpEnd (the tokens of a `{…}` expression), then StrEnd.
    StrStart,
    /// String content with escapes decoded and, for multi-line strings,
    /// the common indentation removed.
    Text(String),
    InterpStart,
    InterpEnd,
    StrEnd,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Dot,
    DotDot,
    DotDotEq,
    /// `...rest` in a list pattern (spec 1.12).
    Ellipsis,
    Question,
    QuestionDot,
    QuestionQuestion,
    Arrow,
    FatArrow,
    Bang,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    /// `**` and `**=` (spec 1.8).
    StarStar,
    StarStarEq,
    AmpEq,
    PipeEq,
    CaretEq,
    ShlEq,
    ShrEq,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,

    /// The end of a statement (spec 1.1).
    Newline,
    /// Text that is not valid Kavach. A matching [`crate::LexError`] explains it.
    Error,
    Eof,
}

/// Punctuation, longest first so that `..=` wins over `..` and `.`.
pub(crate) const PUNCTUATION: &[(&str, TokenKind)] = &[
    ("...", TokenKind::Ellipsis),
    ("**=", TokenKind::StarStarEq),
    ("<<=", TokenKind::ShlEq),
    (">>=", TokenKind::ShrEq),
    ("..=", TokenKind::DotDotEq),
    ("->", TokenKind::Arrow),
    ("=>", TokenKind::FatArrow),
    ("==", TokenKind::EqEq),
    ("!=", TokenKind::NotEq),
    ("<=", TokenKind::Le),
    (">=", TokenKind::Ge),
    ("**", TokenKind::StarStar),
    ("&=", TokenKind::AmpEq),
    ("|=", TokenKind::PipeEq),
    ("^=", TokenKind::CaretEq),
    ("<<", TokenKind::Shl),
    (">>", TokenKind::Shr),
    ("??", TokenKind::QuestionQuestion),
    ("?.", TokenKind::QuestionDot),
    ("..", TokenKind::DotDot),
    ("+=", TokenKind::PlusEq),
    ("-=", TokenKind::MinusEq),
    ("*=", TokenKind::StarEq),
    ("/=", TokenKind::SlashEq),
    ("%=", TokenKind::PercentEq),
    ("(", TokenKind::LParen),
    (")", TokenKind::RParen),
    ("[", TokenKind::LBracket),
    ("]", TokenKind::RBracket),
    ("{", TokenKind::LBrace),
    ("}", TokenKind::RBrace),
    (",", TokenKind::Comma),
    (":", TokenKind::Colon),
    (".", TokenKind::Dot),
    ("?", TokenKind::Question),
    ("!", TokenKind::Bang),
    ("=", TokenKind::Eq),
    ("<", TokenKind::Lt),
    (">", TokenKind::Gt),
    ("+", TokenKind::Plus),
    ("-", TokenKind::Minus),
    ("*", TokenKind::Star),
    ("/", TokenKind::Slash),
    ("%", TokenKind::Percent),
    ("&", TokenKind::Amp),
    ("|", TokenKind::Pipe),
    ("^", TokenKind::Caret),
    ("~", TokenKind::Tilde),
];

/// Whether a line that ends with this token continues on the next line
/// (spec 1.1). `<`, `>`, `>>` and `?` don't, because they also close types.
pub(crate) fn continues_line(kind: &TokenKind) -> bool {
    use TokenKind::*;
    matches!(
        kind,
        Comma
            | Colon
            | Dot
            | DotDot
            | DotDotEq
            | QuestionDot
            | QuestionQuestion
            | Arrow
            | FatArrow
            | Bang
            | Eq
            | EqEq
            | NotEq
            | Le
            | Ge
            | Plus
            | Minus
            | Star
            | Slash
            | Percent
            | PlusEq
            | MinusEq
            | StarEq
            | SlashEq
            | PercentEq
            | StarStar
            | StarStarEq
            | AmpEq
            | PipeEq
            | CaretEq
            | ShlEq
            | ShrEq
            | Amp
            | Pipe
            | Caret
            | Tilde
            | Shl
            | LParen
            | LBracket
            | LBrace
            | TokenKind::Keyword(
                crate::token::Keyword::And | crate::token::Keyword::Or | crate::token::Keyword::Not
            )
    )
}

macro_rules! keywords {
    ($($name:ident = $text:literal,)*) => {
        /// The reserved words of spec 1.6. Contextual keywords (`result`,
        /// `fails`, `context`, the agent words…) are ordinary names to the lexer.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Keyword { $($name,)* }

        impl Keyword {
            pub const ALL: &[Keyword] = &[$(Keyword::$name,)*];

            pub fn lookup(text: &str) -> Option<Keyword> {
                match text {
                    $($text => Some(Keyword::$name),)*
                    _ => None,
                }
            }

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Keyword::$name => $text,)*
                }
            }
        }
    };
}

keywords! {
    Agent = "agent", And = "and", Arena = "arena", As = "as",
    Assert = "assert", Break = "break", Capability = "capability",
    Catch = "catch", Const = "const", Continue = "continue", Deprecated = "deprecated", Else = "else",
    Ensure = "ensure", Example = "example", Extend = "extend", Extern = "extern",
    Fail = "fail", False = "false", Fn = "fn", For = "for", If = "if", In = "in",
    Interface = "interface", Invariant = "invariant", Let = "let",
    Match = "match", None = "none", Not = "not", Or = "or",
    Pub = "pub", Require = "require", Return = "return", Secret = "secret",
    SelfValue = "self", Spawn = "spawn", Test = "test", True = "true", Try = "try",
    Type = "type", Unsafe = "unsafe", Use = "use", Uses = "uses", Var = "var",
    While = "while", With = "with",
}
