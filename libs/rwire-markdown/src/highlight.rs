//! Minimal, zero-dependency syntax highlighting for fenced code blocks.
//!
//! A small table-driven lexer over the languages that actually show up in rwire apps (Rust,
//! JSON, shell, SQL). [`highlight`] returns `None` for an unknown language, so the caller falls
//! back to flat text. The aim is "readable at a glance", not a complete grammar — the lexer is
//! deliberately forgiving, and its one hard invariant is that the concatenation of the returned
//! tokens equals the input exactly (no character is dropped or duplicated).

/// A highlighted token's category — mapped to a colour at render time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenKind {
    Keyword,
    Str,
    Comment,
    Number,
    Plain,
}

/// One run of source text tagged with its category.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

/// Tokenize `code` for `lang` (case-insensitive name; e.g. `rs`, `sh`). Returns `None` for an
/// unsupported language so the caller can render the raw text instead.
pub fn highlight(code: &str, lang: &str) -> Option<Vec<Token>> {
    LangSpec::for_lang(lang).map(|spec| lex(code, &spec))
}

/// Per-language lexer rules. All fields are static, so a spec is cheap to copy.
#[derive(Clone, Copy)]
struct LangSpec {
    /// Reserved words. Stored lowercase when `case_insensitive` is set (e.g. SQL).
    keywords: &'static [&'static str],
    line_comment: Option<&'static str>,
    block_comment: Option<(&'static str, &'static str)>,
    string_delims: &'static [char],
    case_insensitive: bool,
}

impl LangSpec {
    fn for_lang(lang: &str) -> Option<LangSpec> {
        match lang.trim().to_ascii_lowercase().as_str() {
            "rust" | "rs" => Some(RUST),
            "json" => Some(JSON),
            "bash" | "sh" | "shell" | "zsh" => Some(BASH),
            "sql" => Some(SQL),
            _ => None,
        }
    }

    fn is_keyword(&self, word: &str) -> bool {
        if self.case_insensitive {
            let lower = word.to_ascii_lowercase();
            self.keywords.contains(&lower.as_str())
        } else {
            self.keywords.contains(&word)
        }
    }
}

const RUST: LangSpec = LangSpec {
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
        "true", "type", "unsafe", "use", "where", "while",
    ],
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    string_delims: &['"'],
    case_insensitive: false,
};

const JSON: LangSpec = LangSpec {
    keywords: &["true", "false", "null"],
    line_comment: None,
    block_comment: None,
    string_delims: &['"'],
    case_insensitive: false,
};

const BASH: LangSpec = LangSpec {
    keywords: &[
        "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac",
        "function", "in", "select", "return", "export", "local", "readonly", "declare", "set",
        "unset", "trap", "source", "eval", "exec", "echo",
    ],
    line_comment: Some("#"),
    block_comment: None,
    string_delims: &['"', '\''],
    case_insensitive: false,
};

const SQL: LangSpec = LangSpec {
    keywords: &[
        "select",
        "from",
        "where",
        "insert",
        "into",
        "update",
        "delete",
        "create",
        "table",
        "drop",
        "alter",
        "add",
        "column",
        "index",
        "view",
        "join",
        "left",
        "right",
        "inner",
        "outer",
        "on",
        "group",
        "by",
        "order",
        "having",
        "limit",
        "offset",
        "values",
        "set",
        "as",
        "distinct",
        "and",
        "or",
        "not",
        "null",
        "primary",
        "key",
        "foreign",
        "references",
        "default",
        "unique",
        "check",
        "union",
        "all",
        "exists",
        "between",
        "like",
        "is",
    ],
    line_comment: Some("--"),
    block_comment: Some(("/*", "*/")),
    string_delims: &['\''],
    case_insensitive: true,
};

fn lex(code: &str, spec: &LangSpec) -> Vec<Token> {
    let chars: Vec<char> = code.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut plain = String::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some(marker) = spec.line_comment {
            if matches_at(&chars, i, marker) {
                flush_plain(&mut plain, &mut tokens);
                let start = i;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                tokens.push(slice_token(&chars, start, i, TokenKind::Comment));
                continue;
            }
        }
        if let Some((open, close)) = spec.block_comment {
            if matches_at(&chars, i, open) {
                flush_plain(&mut plain, &mut tokens);
                let start = i;
                i += open.chars().count();
                while i < chars.len() && !matches_at(&chars, i, close) {
                    i += 1;
                }
                if i < chars.len() {
                    i += close.chars().count();
                }
                tokens.push(slice_token(&chars, start, i, TokenKind::Comment));
                continue;
            }
        }
        let c = chars[i];
        if spec.string_delims.contains(&c) {
            flush_plain(&mut plain, &mut tokens);
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != c {
                // Skip an escaped character so a `\"` doesn't close the string early.
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            if i < chars.len() {
                i += 1; // the closing delimiter
            }
            tokens.push(slice_token(&chars, start, i, TokenKind::Str));
            continue;
        }
        if c.is_ascii_digit() {
            flush_plain(&mut plain, &mut tokens);
            let start = i;
            while i < chars.len() && is_number_char(chars[i]) {
                i += 1;
            }
            tokens.push(slice_token(&chars, start, i, TokenKind::Number));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if spec.is_keyword(&word) {
                flush_plain(&mut plain, &mut tokens);
                tokens.push(Token {
                    kind: TokenKind::Keyword,
                    text: word,
                });
            } else {
                // A plain identifier merges into the surrounding plain run.
                plain.push_str(&word);
            }
            continue;
        }
        plain.push(c);
        i += 1;
    }
    flush_plain(&mut plain, &mut tokens);
    tokens
}

/// Whether `chars[i..]` begins with `marker`.
fn matches_at(chars: &[char], i: usize, marker: &str) -> bool {
    marker
        .chars()
        .enumerate()
        .all(|(offset, m)| chars.get(i + offset) == Some(&m))
}

fn is_number_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '.' || c == '_'
}

fn slice_token(chars: &[char], start: usize, end: usize, kind: TokenKind) -> Token {
    Token {
        kind,
        text: chars[start..end].iter().collect(),
    }
}

/// Emit the accumulated plain run as a token (if non-empty) and clear it.
fn flush_plain(plain: &mut String, tokens: &mut Vec<Token>) {
    if !plain.is_empty() {
        tokens.push(Token {
            kind: TokenKind::Plain,
            text: std::mem::take(plain),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The load-bearing invariant: rendering loses nothing — the tokens reassemble the source.
    fn assert_roundtrip(code: &str, lang: &str) -> Vec<Token> {
        let tokens = highlight(code, lang).expect("known language");
        let rejoined: String = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(rejoined, code, "tokens must reassemble the input exactly");
        tokens
    }

    fn kinds(tokens: &[Token], kind: TokenKind) -> Vec<&str> {
        tokens
            .iter()
            .filter(|t| t.kind == kind)
            .map(|t| t.text.as_str())
            .collect()
    }

    #[test]
    fn unknown_language_is_none() {
        assert!(highlight("whatever", "brainfuck").is_none());
        assert!(highlight("x", "").is_none());
    }

    #[test]
    fn language_aliases_resolve() {
        assert!(highlight("let x = 1;", "rs").is_some());
        assert!(highlight("echo hi", "shell").is_some());
    }

    #[test]
    fn rust_keywords_numbers_strings_comments() {
        let code = "fn main() { let x = 42; } // done";
        let tokens = assert_roundtrip(code, "rust");
        assert!(kinds(&tokens, TokenKind::Keyword).contains(&"fn"));
        assert!(kinds(&tokens, TokenKind::Keyword).contains(&"let"));
        assert!(kinds(&tokens, TokenKind::Number).contains(&"42"));
        assert_eq!(kinds(&tokens, TokenKind::Comment), vec!["// done"]);
        // `main` is an identifier, not a keyword.
        assert!(!kinds(&tokens, TokenKind::Keyword).contains(&"main"));
    }

    #[test]
    fn rust_string_with_escaped_quote_is_one_token() {
        let code = r#"let s = "a\"b";"#;
        let tokens = assert_roundtrip(code, "rust");
        assert_eq!(kinds(&tokens, TokenKind::Str), vec![r#""a\"b""#]);
    }

    #[test]
    fn rust_block_comment() {
        let code = "a /* multi\nline */ b";
        let tokens = assert_roundtrip(code, "rust");
        assert_eq!(
            kinds(&tokens, TokenKind::Comment),
            vec!["/* multi\nline */"]
        );
    }

    #[test]
    fn json_strings_keywords_numbers() {
        let code = r#"{"on": true, "n": 12.5}"#;
        let tokens = assert_roundtrip(code, "json");
        assert_eq!(kinds(&tokens, TokenKind::Keyword), vec!["true"]);
        assert!(kinds(&tokens, TokenKind::Number).contains(&"12.5"));
        assert!(kinds(&tokens, TokenKind::Str).contains(&"\"on\""));
    }

    #[test]
    fn sql_keywords_are_case_insensitive() {
        let upper = assert_roundtrip("SELECT 1 FROM t", "sql");
        assert_eq!(kinds(&upper, TokenKind::Keyword), vec!["SELECT", "FROM"]);
        let lower = highlight("select 1 from t", "sql").unwrap();
        assert_eq!(kinds(&lower, TokenKind::Keyword), vec!["select", "from"]);
        // The original casing is preserved in the token text.
    }

    #[test]
    fn bash_hash_comment_and_quotes() {
        let code = "echo 'hi' # a note";
        let tokens = assert_roundtrip(code, "bash");
        assert!(kinds(&tokens, TokenKind::Keyword).contains(&"echo"));
        assert_eq!(kinds(&tokens, TokenKind::Str), vec!["'hi'"]);
        assert_eq!(kinds(&tokens, TokenKind::Comment), vec!["# a note"]);
    }

    #[test]
    fn unterminated_string_consumes_to_end_without_panic() {
        let code = r#"let s = "oops"#;
        let tokens = assert_roundtrip(code, "rust");
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Str));
    }

    #[test]
    fn unicode_is_preserved() {
        // Multi-byte chars in strings/identifiers must not corrupt the round-trip.
        assert_roundtrip(r#"let s = "héllo→ω";"#, "rust");
    }
}
