// ─────────────────────────────────────────────────────────────
// ANWE v0.1 — LEXER
//
// Reads Anwe source text and produces tokens.
// Hand-written for maximum performance and clarity.
// No dependencies. No regex. No generator.
//
// The lexer recognizes Anwe's unique operators:
//   <->  <<>>  >>  =>  <=  ~  ~>  <-  *  ?
// These symbols carry the meaning of the language visually.
// ─────────────────────────────────────────────────────────────

use crate::token::{lookup_keyword, Span, Token, TokenKind};

/// Lexer error.
#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lex error at {}: {}", self.span, self.message)
    }
}

impl std::error::Error for LexError {}

/// The Anwe lexer. Converts source text into tokens.
pub struct Lexer<'src> {
    source: &'src [u8],
    pos: usize,
    line: u32,
    column: u32,
    line_start: usize,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source text.
    pub fn new(source: &'src str) -> Self {
        Lexer {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
            line_start: 0,
        }
    }

    /// Tokenize the entire source into a Vec of tokens.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token.is_eof();
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// Get the next token.
    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();

        if self.pos >= self.source.len() {
            return Ok(Token::new(
                TokenKind::Eof,
                self.span_here(0),
            ));
        }

        let start = self.pos;
        let ch = self.source[self.pos];

        match ch {
            // ─── Multi-character operators ───
            b'<' => self.lex_angle_left(start),
            b'>' => self.lex_angle_right(start),
            b'~' => self.lex_tilde(start),
            b'=' => self.lex_equals(start),

            // ─── Single-character operators ───
            b'*' => { self.advance(); Ok(self.token(TokenKind::Commit, start)) }
            b'?' => { self.advance(); Ok(self.token(TokenKind::Question, start)) }
            b'+' => { self.advance(); Ok(self.token(TokenKind::Plus, start)) }
            b'-' => {
                // '-' standalone is Minus (subtraction/negation)
                // '--' is a comment (handled in skip_whitespace_and_comments)
                self.advance();
                Ok(self.token(TokenKind::Minus, start))
            }
            b'/' => { self.advance(); Ok(self.token(TokenKind::Slash, start)) }
            b'%' => { self.advance(); Ok(self.token(TokenKind::Percent, start)) }
            b'!' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    Ok(self.token(TokenKind::BangEqual, start)) // !=
                } else {
                    Err(LexError {
                        message: "Unexpected '!'. Did you mean '!='?".into(),
                        span: self.span_here(1),
                    })
                }
            }
            b'|' => {
                self.advance(); // consume '|'
                if self.peek() == Some(b'>') {
                    self.advance(); // consume '>'
                    Ok(self.token(TokenKind::Pipe, start)) // |>
                } else {
                    Ok(self.token(TokenKind::Bar, start)) // | (lambda delimiter)
                }
            }
            b'{' => { self.advance(); Ok(self.token(TokenKind::LBrace, start)) }
            b'}' => { self.advance(); Ok(self.token(TokenKind::RBrace, start)) }
            b'(' => { self.advance(); Ok(self.token(TokenKind::LParen, start)) }
            b')' => { self.advance(); Ok(self.token(TokenKind::RParen, start)) }
            b'[' => { self.advance(); Ok(self.token(TokenKind::LBracket, start)) }
            b']' => { self.advance(); Ok(self.token(TokenKind::RBracket, start)) }
            b':' => { self.advance(); Ok(self.token(TokenKind::Colon, start)) }
            b',' => { self.advance(); Ok(self.token(TokenKind::Comma, start)) }
            b';' => { self.advance(); Ok(self.token(TokenKind::Semicolon, start)) }
            b'.' => {
                // Check if this is the start of a number like .5
                if self.pos + 1 < self.source.len() && self.source[self.pos + 1].is_ascii_digit() {
                    self.lex_number(start)
                } else {
                    self.advance();
                    Ok(self.token(TokenKind::Dot, start))
                }
            }

            // ─── String literals ───
            b'"' => self.lex_string(start),

            // ─── Number literals ───
            b'0'..=b'9' => self.lex_number(start),

            // ─── Identifiers and keywords ───
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_identifier(start),

            _ => Err(LexError {
                message: format!("Unexpected character: '{}'", ch as char),
                span: self.span_here(1),
            }),
        }
    }

    /// Lex operators starting with '<'
    fn lex_angle_left(&mut self, start: usize) -> Result<Token, LexError> {
        self.advance(); // consume '<'

        if self.peek() == Some(b'-') {
            self.advance(); // consume '-'
            if self.peek() == Some(b'>') {
                self.advance(); // consume '>'
                Ok(self.token(TokenKind::BiDir, start))    // <->
            } else {
                Ok(self.token(TokenKind::StructChange, start)) // <-
            }
        } else if self.peek() == Some(b'<') {
            self.advance(); // consume second '<'
            if self.peek() == Some(b'>') {
                self.advance(); // consume first '>'
                if self.peek() == Some(b'>') {
                    self.advance(); // consume second '>'
                    Ok(self.token(TokenKind::Converge, start)) // <<>>
                } else {
                    Err(LexError {
                        message: "Expected '>' to complete '<<>>' operator".to_string(),
                        span: self.span_here(1),
                    })
                }
            } else {
                Err(LexError {
                    message: "Expected '>>' to complete '<<>>' operator".to_string(),
                    span: self.span_here(1),
                })
            }
        } else if self.peek() == Some(b'=') {
            self.advance(); // consume '='
            Ok(self.token(TokenKind::LessEq, start)) // <=
        } else {
            Ok(self.token(TokenKind::Less, start)) // <
        }
    }

    /// Lex operators starting with '>'
    fn lex_angle_right(&mut self, start: usize) -> Result<Token, LexError> {
        self.advance(); // consume '>'
        if self.peek() == Some(b'>') {
            self.advance(); // consume second '>'
            Ok(self.token(TokenKind::Alert, start)) // >>
        } else if self.peek() == Some(b'=') {
            self.advance(); // consume '='
            Ok(self.token(TokenKind::GreaterEq, start)) // >=
        } else {
            Ok(self.token(TokenKind::Greater, start)) // >
        }
    }

    /// Lex operators starting with '~'
    fn lex_tilde(&mut self, start: usize) -> Result<Token, LexError> {
        self.advance(); // consume '~'
        if self.peek() == Some(b'>') {
            self.advance(); // consume '>'
            Ok(self.token(TokenKind::PatternFlow, start)) // ~>
        } else {
            Ok(self.token(TokenKind::Sync, start)) // ~
        }
    }

    /// Lex operators starting with '='
    fn lex_equals(&mut self, start: usize) -> Result<Token, LexError> {
        self.advance(); // consume '='
        if self.peek() == Some(b'>') {
            self.advance(); // consume '>'
            Ok(self.token(TokenKind::Apply, start)) // =>
        } else if self.peek() == Some(b'=') {
            self.advance(); // consume second '='
            Ok(self.token(TokenKind::EqualEqual, start)) // ==
        } else {
            Ok(self.token(TokenKind::Assign, start)) // = (assignment)
        }
    }

    /// Lex a string literal.
    fn lex_string(&mut self, start: usize) -> Result<Token, LexError> {
        self.advance(); // consume opening '"'
        let mut value = String::new();

        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            match ch {
                b'"' => {
                    self.advance(); // consume closing '"'
                    return Ok(self.token(TokenKind::StringLit(value), start));
                }
                b'\\' => {
                    self.advance();
                    if self.pos >= self.source.len() {
                        return Err(LexError {
                            message: "Unterminated escape sequence in string".to_string(),
                            span: self.span_here(1),
                        });
                    }
                    match self.source[self.pos] {
                        b'n' => value.push('\n'),
                        b't' => value.push('\t'),
                        b'\\' => value.push('\\'),
                        b'"' => value.push('"'),
                        other => {
                            return Err(LexError {
                                message: format!("Unknown escape sequence: \\{}", other as char),
                                span: self.span_here(1),
                            });
                        }
                    }
                    self.advance();
                }
                b'\n' => {
                    value.push('\n');
                    self.advance();
                    self.line += 1;
                    self.column = 1;
                    self.line_start = self.pos;
                }
                _ => {
                    value.push(ch as char);
                    self.advance();
                }
            }
        }

        Err(LexError {
            message: "Unterminated string literal".to_string(),
            span: Span::new(start, self.pos, self.line, (start - self.line_start + 1) as u32),
        })
    }

    /// Lex a number literal.
    fn lex_number(&mut self, start: usize) -> Result<Token, LexError> {
        let mut has_dot = self.source[self.pos] == b'.';
        self.advance();

        while self.pos < self.source.len() {
            match self.source[self.pos] {
                b'0'..=b'9' => self.advance(),
                b'.' if !has_dot => {
                    has_dot = true;
                    self.advance();
                }
                _ => break,
            }
        }

        let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap();
        let value: f64 = text.parse().map_err(|_| LexError {
            message: format!("Invalid number: {}", text),
            span: Span::new(start, self.pos, self.line, (start - self.line_start + 1) as u32),
        })?;

        Ok(self.token(TokenKind::Number(value), start))
    }

    /// Lex an identifier or keyword.
    fn lex_identifier(&mut self, start: usize) -> Result<Token, LexError> {
        self.advance();

        while self.pos < self.source.len() {
            match self.source[self.pos] {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => self.advance(),
                _ => break,
            }
        }

        let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap();

        // f"..." is an interpolated string literal
        if text == "f" && self.pos < self.source.len() && self.source[self.pos] == b'"' {
            return self.lex_fstring(start);
        }

        let kind = lookup_keyword(text)
            .unwrap_or_else(|| TokenKind::Ident(text.to_string()));

        Ok(self.token(kind, start))
    }

    /// Lex an f-string literal: f"Hello {name}, you have {len(items)} items"
    /// The raw content (including {expr} markers) is captured as a single string.
    fn lex_fstring(&mut self, start: usize) -> Result<Token, LexError> {
        self.advance(); // consume opening '"'
        let mut value = String::new();
        let mut brace_depth = 0;

        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            match ch {
                b'"' if brace_depth == 0 => {
                    self.advance(); // consume closing '"'
                    return Ok(self.token(TokenKind::FStringLit(value), start));
                }
                b'{' => {
                    brace_depth += 1;
                    value.push('{');
                    self.advance();
                }
                b'}' => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    }
                    value.push('}');
                    self.advance();
                }
                b'\\' => {
                    self.advance();
                    if self.pos >= self.source.len() {
                        return Err(LexError {
                            message: "Unterminated escape in f-string".into(),
                            span: self.span_here(1),
                        });
                    }
                    match self.source[self.pos] {
                        b'n' => value.push('\n'),
                        b't' => value.push('\t'),
                        b'\\' => value.push('\\'),
                        b'"' => value.push('"'),
                        b'{' => value.push('{'),
                        b'}' => value.push('}'),
                        other => {
                            return Err(LexError {
                                message: format!("Unknown escape: \\{}", other as char),
                                span: self.span_here(1),
                            });
                        }
                    }
                    self.advance();
                }
                b'\n' => {
                    value.push('\n');
                    self.advance();
                    self.line += 1;
                    self.column = 1;
                    self.line_start = self.pos;
                }
                _ => {
                    value.push(ch as char);
                    self.advance();
                }
            }
        }

        Err(LexError {
            message: "Unterminated f-string literal".into(),
            span: Span::new(start, self.pos, self.line, (start - self.line_start + 1) as u32),
        })
    }

    /// Skip whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.source.len() {
            match self.source[self.pos] {
                b' ' | b'\t' | b'\r' => self.advance(),
                b'\n' => {
                    self.advance();
                    self.line += 1;
                    self.column = 1;
                    self.line_start = self.pos;
                }
                b'-' if self.pos + 1 < self.source.len() && self.source[self.pos + 1] == b'-' => {
                    // Line comment: -- ... \n
                    self.advance(); // first -
                    self.advance(); // second -
                    while self.pos < self.source.len() && self.source[self.pos] != b'\n' {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    /// Peek at the next byte without consuming.
    #[inline(always)]
    fn peek(&self) -> Option<u8> {
        if self.pos < self.source.len() {
            Some(self.source[self.pos])
        } else {
            None
        }
    }

    /// Advance one byte.
    #[inline(always)]
    fn advance(&mut self) {
        self.pos += 1;
        self.column += 1;
    }

    /// Create a span at the current position.
    fn span_here(&self, len: usize) -> Span {
        Span::new(
            self.pos,
            self.pos + len,
            self.line,
            self.column,
        )
    }

    /// Create a token from start position to current position.
    fn token(&self, kind: TokenKind, start: usize) -> Token {
        let col = (start.saturating_sub(self.line_start) + 1) as u32;
        Token::new(kind, Span::new(start, self.pos, self.line, col))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(src);
        lexer.tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| !matches!(k, TokenKind::Eof))
            .collect()
    }

    #[test]
    fn operators() {
        assert_eq!(lex("<->"), vec![TokenKind::BiDir]);
        assert_eq!(lex(">>"), vec![TokenKind::Alert]);
        assert_eq!(lex("=>"), vec![TokenKind::Apply]);
        assert_eq!(lex("<<>>"), vec![TokenKind::Converge]);
        assert_eq!(lex("~"), vec![TokenKind::Sync]);
        assert_eq!(lex("*"), vec![TokenKind::Commit]);
        assert_eq!(lex("<-"), vec![TokenKind::StructChange]);
        assert_eq!(lex("~>"), vec![TokenKind::PatternFlow]);
    }

    #[test]
    fn keywords() {
        assert_eq!(lex("agent"), vec![TokenKind::KwAgent]);
        assert_eq!(lex("link"), vec![TokenKind::KwLink]);
        assert_eq!(lex("connect"), vec![TokenKind::KwConnect]);
        assert_eq!(lex("when"), vec![TokenKind::KwWhen]);
        assert_eq!(lex("pending"), vec![TokenKind::KwPending]);
    }

    #[test]
    fn qualities() {
        assert_eq!(lex("attending"), vec![TokenKind::QualAttending]);
        assert_eq!(lex("questioning"), vec![TokenKind::QualQuestioning]);
        assert_eq!(lex("disturbed"), vec![TokenKind::QualDisturbed]);
    }

    #[test]
    fn literals() {
        assert_eq!(lex("42"), vec![TokenKind::Number(42.0)]);
        assert_eq!(lex("0.75"), vec![TokenKind::Number(0.75)]);
        assert_eq!(
            lex("\"hello\""),
            vec![TokenKind::StringLit("hello".to_string())]
        );
    }

    #[test]
    fn identifiers() {
        assert_eq!(lex("Mikel"), vec![TokenKind::Ident("Mikel".to_string())]);
        assert_eq!(
            lex("Primordia"),
            vec![TokenKind::Ident("Primordia".to_string())]
        );
    }

    #[test]
    fn comments_skipped() {
        assert_eq!(lex("-- this is a comment\nagent"), vec![TokenKind::KwAgent]);
    }

    #[test]
    fn hello_world_fragment() {
        let tokens = lex("agent Mikel\nagent Primordia");
        assert_eq!(tokens, vec![
            TokenKind::KwAgent,
            TokenKind::Ident("Mikel".to_string()),
            TokenKind::KwAgent,
            TokenKind::Ident("Primordia".to_string()),
        ]);
    }

    #[test]
    fn link_declaration() {
        let tokens = lex("link Mikel <-> Primordia {");
        assert_eq!(tokens, vec![
            TokenKind::KwLink,
            TokenKind::Ident("Mikel".to_string()),
            TokenKind::BiDir,
            TokenKind::Ident("Primordia".to_string()),
            TokenKind::LBrace,
        ]);
    }

    #[test]
    fn alert_expression() {
        let tokens = lex(">> { quality: deep, priority: 0.92 } \"something moved\"");
        assert_eq!(tokens, vec![
            TokenKind::Alert,
            TokenKind::LBrace,
            TokenKind::KwQuality,
            TokenKind::Colon,
            TokenKind::DepthDeep,
            TokenKind::Comma,
            TokenKind::KwPriority,
            TokenKind::Colon,
            TokenKind::Number(0.92),
            TokenKind::RBrace,
            TokenKind::StringLit("something moved".to_string()),
        ]);
    }

    #[test]
    fn sync_expression() {
        let tokens = lex("Alpha ~ Beta until synchronized");
        assert_eq!(tokens, vec![
            TokenKind::Ident("Alpha".to_string()),
            TokenKind::Sync,
            TokenKind::Ident("Beta".to_string()),
            TokenKind::KwUntil,
            TokenKind::SyncSynchronized,
        ]);
    }

    #[test]
    fn apply_expression() {
        let tokens = lex("=> when sync_level > 0.7");
        assert_eq!(tokens, vec![
            TokenKind::Apply,
            TokenKind::KwWhen,
            TokenKind::KwSyncLevel,
            TokenKind::Greater,
            TokenKind::Number(0.7),
        ]);
    }

    #[test]
    fn commit_expression() {
        let tokens = lex("* from apply");
        assert_eq!(tokens, vec![
            TokenKind::Commit,
            TokenKind::KwFrom,
            TokenKind::KwApply,
        ]);
    }

    #[test]
    fn converge_expression() {
        let tokens = lex("converge Alpha <<>> Beta {");
        assert_eq!(tokens, vec![
            TokenKind::KwConverge,
            TokenKind::Ident("Alpha".to_string()),
            TokenKind::Converge,
            TokenKind::Ident("Beta".to_string()),
            TokenKind::LBrace,
        ]);
    }

    #[test]
    fn pending_expression() {
        let tokens = lex("pending? link_not_established {");
        assert_eq!(tokens, vec![
            TokenKind::KwPending,
            TokenKind::Question,
            TokenKind::NyLinkNotEstablished,
            TokenKind::LBrace,
        ]);
    }

    #[test]
    fn arithmetic_tokens() {
        assert_eq!(lex("+"), vec![TokenKind::Plus]);
        assert_eq!(lex("/"), vec![TokenKind::Slash]);
        assert_eq!(lex("%"), vec![TokenKind::Percent]);
        assert_eq!(lex("3 + 4"), vec![
            TokenKind::Number(3.0),
            TokenKind::Plus,
            TokenKind::Number(4.0),
        ]);
        assert_eq!(lex("10 - 3"), vec![
            TokenKind::Number(10.0),
            TokenKind::Minus,
            TokenKind::Number(3.0),
        ]);
    }

    #[test]
    fn bracket_tokens() {
        assert_eq!(lex("[1, 2, 3]"), vec![
            TokenKind::LBracket,
            TokenKind::Number(1.0),
            TokenKind::Comma,
            TokenKind::Number(2.0),
            TokenKind::Comma,
            TokenKind::Number(3.0),
            TokenKind::RBracket,
        ]);
    }

    #[test]
    fn iteration_keywords() {
        assert_eq!(lex("each"), vec![TokenKind::KwEach]);
        assert_eq!(lex("in"), vec![TokenKind::KwIn]);
        assert_eq!(lex("if"), vec![TokenKind::KwIf]);
        assert_eq!(lex("else"), vec![TokenKind::KwElse]);
    }

    // ─── FIRST-PERSON COGNITION TOKENS ──────────────────────

    #[test]
    fn first_person_keywords() {
        assert_eq!(lex("mind"), vec![TokenKind::KwMind]);
        assert_eq!(lex("attend"), vec![TokenKind::KwAttend]);
        assert_eq!(lex("think"), vec![TokenKind::KwThink]);
        assert_eq!(lex("express"), vec![TokenKind::KwExpress]);
    }

    #[test]
    fn mind_declaration() {
        let tokens = lex("mind Cognition {");
        assert_eq!(tokens, vec![
            TokenKind::KwMind,
            TokenKind::Ident("Cognition".to_string()),
            TokenKind::LBrace,
        ]);
    }

    #[test]
    fn mind_with_attention() {
        let tokens = lex("mind Focus attention 0.5 {");
        assert_eq!(tokens, vec![
            TokenKind::KwMind,
            TokenKind::Ident("Focus".to_string()),
            TokenKind::KwAttention,
            TokenKind::Number(0.5),
            TokenKind::LBrace,
        ]);
    }

    #[test]
    fn attend_block() {
        let tokens = lex("attend \"incoming signal\" priority 0.9 {");
        assert_eq!(tokens, vec![
            TokenKind::KwAttend,
            TokenKind::StringLit("incoming signal".to_string()),
            TokenKind::KwPriority,
            TokenKind::Number(0.9),
            TokenKind::LBrace,
        ]);
    }

    #[test]
    fn think_block() {
        let tokens = lex("think { insight <- \"pattern\" }");
        assert_eq!(tokens, vec![
            TokenKind::KwThink,
            TokenKind::LBrace,
            TokenKind::Ident("insight".to_string()),
            TokenKind::StructChange,
            TokenKind::StringLit("pattern".to_string()),
            TokenKind::RBrace,
        ]);
    }

    #[test]
    fn express_statement() {
        let tokens = lex("express \"I see it\"");
        assert_eq!(tokens, vec![
            TokenKind::KwExpress,
            TokenKind::StringLit("I see it".to_string()),
        ]);
    }

    #[test]
    fn express_with_attrs() {
        let tokens = lex("express { quality: recognizing, priority: 0.8 } \"recognized\"");
        assert_eq!(tokens, vec![
            TokenKind::KwExpress,
            TokenKind::LBrace,
            TokenKind::KwQuality,
            TokenKind::Colon,
            TokenKind::QualRecognizing,
            TokenKind::Comma,
            TokenKind::KwPriority,
            TokenKind::Colon,
            TokenKind::Number(0.8),
            TokenKind::RBrace,
            TokenKind::StringLit("recognized".to_string()),
        ]);
    }

    #[test]
    fn full_mind_fragment() {
        // A complete mind declaration fragment
        let tokens = lex(r#"mind Self attention 0.8 { attend "reflect" priority 0.9 { think { depth <- 3 } express "done" } }"#);
        assert_eq!(tokens[0], TokenKind::KwMind);
        assert_eq!(tokens[1], TokenKind::Ident("Self".to_string()));
        assert_eq!(tokens[2], TokenKind::KwAttention);
        assert_eq!(tokens[3], TokenKind::Number(0.8));
        assert_eq!(tokens[4], TokenKind::LBrace);
        assert_eq!(tokens[5], TokenKind::KwAttend);
        assert!(tokens.contains(&TokenKind::KwThink));
        assert!(tokens.contains(&TokenKind::KwExpress));
    }
}
