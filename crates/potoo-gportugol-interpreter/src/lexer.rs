use crate::source::{Diagnostic, DiagnosticKind, SourceLocation, SourceSpan};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Identifier,
    IntegerLiteral,
    RealLiteral,
    CharacterLiteral,
    StringLiteral,
    Algorithm,
    Variables,
    EndVariables,
    Integer,
    Real,
    Character,
    Literal,
    Logical,
    Start,
    End,
    True,
    False,
    Or,
    And,
    Not,
    If,
    Else,
    Then,
    EndIf,
    While,
    Do,
    EndWhile,
    For,
    From,
    To,
    Step,
    EndFor,
    Matrix,
    Integers,
    Reals,
    Characters,
    Literals,
    Logicals,
    Function,
    Return,
    Repeat,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Pipe,
    Caret,
    Ampersand,
    Tilde,
    Assign,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LogicalOr,
    LogicalAnd,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    Semicolon,
    EndOfFile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: SourceSpan,
}

const KEYWORDS: &[(&str, TokenKind)] = &[
    ("fim-variáveis", TokenKind::EndVariables),
    ("fim-enquanto", TokenKind::EndWhile),
    ("fim-para", TokenKind::EndFor),
    ("fim-se", TokenKind::EndIf),
    ("algoritmo", TokenKind::Algorithm),
    ("variáveis", TokenKind::Variables),
    ("inteiro", TokenKind::Integer),
    ("caractere", TokenKind::Character),
    ("literal", TokenKind::Literal),
    ("lógico", TokenKind::Logical),
    ("verdadeiro", TokenKind::True),
    ("falso", TokenKind::False),
    ("início", TokenKind::Start),
    ("fim", TokenKind::End),
    ("ou", TokenKind::Or),
    ("e", TokenKind::And),
    ("não", TokenKind::Not),
    ("se", TokenKind::If),
    ("senão", TokenKind::Else),
    ("então", TokenKind::Then),
    ("enquanto", TokenKind::While),
    ("faça", TokenKind::Do),
    ("para", TokenKind::For),
    ("de", TokenKind::From),
    ("até", TokenKind::To),
    ("passo", TokenKind::Step),
    ("matriz", TokenKind::Matrix),
    ("inteiros", TokenKind::Integers),
    ("reais", TokenKind::Reals),
    ("caracteres", TokenKind::Characters),
    ("literais", TokenKind::Literals),
    ("lógicos", TokenKind::Logicals),
    ("real", TokenKind::Real),
    ("função", TokenKind::Function),
    ("retorne", TokenKind::Return),
    ("repita", TokenKind::Repeat),
];

pub fn lex(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    Lexer::new(source).run()
}

struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: usize::from(source.starts_with('\u{feff}')) * '\u{feff}'.len_utf8(),
            line: 1,
            column: 1,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn run(mut self) -> Result<Vec<Token>, Vec<Diagnostic>> {
        while let Some(character) = self.current() {
            if matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{00a0}') {
                self.advance();
                continue;
            }

            if self.remaining().starts_with("//") {
                self.advance();
                self.advance();
                while self
                    .current()
                    .is_some_and(|value| value != '\n' && value != '\r')
                {
                    self.advance();
                }
                continue;
            }

            if self.remaining().starts_with("/*") {
                self.block_comment();
                continue;
            }

            if self.keyword() {
                continue;
            }

            let start = self.location();
            if character == '_' || character.is_alphabetic() {
                self.identifier(start);
                continue;
            }
            if character.is_ascii_digit() {
                self.number(start);
                continue;
            }
            if character == '"' || character == '\'' {
                self.quoted_literal(start, character);
                continue;
            }

            self.operator_or_error(start, character);
        }

        let location = self.location();
        self.tokens.push(Token {
            kind: TokenKind::EndOfFile,
            lexeme: String::new(),
            span: SourceSpan {
                start: location,
                end: location,
            },
        });

        if self.diagnostics.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.diagnostics)
        }
    }

    fn current(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.remaining().chars().nth(offset)
    }

    fn remaining(&self) -> &'a str {
        &self.source[self.offset..]
    }

    fn location(&self) -> SourceLocation {
        SourceLocation {
            line: self.line,
            column: self.column,
            byte_offset: self.offset,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.current()?;
        if character == '\r' {
            self.offset += 1;
            if self.source.as_bytes().get(self.offset) == Some(&b'\n') {
                self.offset += 1;
            }
            self.line += 1;
            self.column = 1;
            return Some('\n');
        }
        self.offset += character.len_utf8();
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(character)
    }

    fn span_from(&self, start: SourceLocation) -> SourceSpan {
        SourceSpan {
            start,
            end: self.location(),
        }
    }

    fn emit(&mut self, start: SourceLocation, kind: TokenKind) {
        let span = self.span_from(start);
        self.tokens.push(Token {
            kind,
            lexeme: self.source[span.start.byte_offset..span.end.byte_offset].to_owned(),
            span,
        });
    }

    fn error(&mut self, start: SourceLocation, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::new(
            DiagnosticKind::Lexical,
            message,
            self.span_from(start),
        ));
    }

    fn keyword(&mut self) -> bool {
        for (keyword, kind) in KEYWORDS {
            if !self.remaining().starts_with(keyword) {
                continue;
            }
            let after = &self.remaining()[keyword.len()..];
            if after.chars().next().is_some_and(is_identifier_continue) {
                continue;
            }
            let start = self.location();
            for _ in keyword.chars() {
                self.advance();
            }
            self.emit(start, *kind);
            return true;
        }
        false
    }

    fn identifier(&mut self, start: SourceLocation) {
        while self.current().is_some_and(is_identifier_continue) {
            self.advance();
        }
        let span = self.span_from(start);
        let lexeme = &self.source[span.start.byte_offset..span.end.byte_offset];
        if !lexeme
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            self.diagnostics.push(Diagnostic::new(
                DiagnosticKind::Lexical,
                "identifiers must contain only ASCII letters, digits, and underscores",
                span,
            ));
            return;
        }
        self.tokens.push(Token {
            kind: TokenKind::Identifier,
            lexeme: lexeme.to_owned(),
            span,
        });
    }

    fn number(&mut self, start: SourceLocation) {
        if self.current() == Some('0') {
            if let Some(prefix) = self
                .peek(1)
                .filter(|prefix| matches!(*prefix, 'b' | 'B' | 'c' | 'C' | 'x' | 'X'))
            {
                self.advance();
                self.advance();
                let base = match prefix.to_ascii_lowercase() {
                    'b' => 2,
                    'c' => 8,
                    _ => 16,
                };
                let digits_start = self.offset;
                while self
                    .current()
                    .is_some_and(|value| value.is_ascii_alphanumeric())
                {
                    self.advance();
                }
                let digits = &self.source[digits_start..self.offset];
                let valid = !digits.is_empty()
                    && digits.chars().all(|digit| match base {
                        2 => matches!(digit, '0' | '1'),
                        8 => matches!(digit, '0'..='7'),
                        _ => digit.is_ascii_hexdigit(),
                    });
                if valid {
                    self.emit(start, TokenKind::IntegerLiteral);
                } else {
                    self.error(start, "invalid integer literal for its base");
                }
                return;
            }
        }

        while self.current().is_some_and(|value| value.is_ascii_digit()) {
            self.advance();
        }
        let mut kind = TokenKind::IntegerLiteral;
        if self.current() == Some('.') && self.peek(1).is_some_and(|value| value.is_ascii_digit()) {
            kind = TokenKind::RealLiteral;
            self.advance();
            while self.current().is_some_and(|value| value.is_ascii_digit()) {
                self.advance();
            }
        }
        self.emit(start, kind);
    }

    fn quoted_literal(&mut self, start: SourceLocation, quote: char) {
        self.advance();
        let content_start = self.offset;
        let mut closed = false;
        let mut non_bmp = false;
        while let Some(character) = self.current() {
            if character == quote {
                let content_end = self.offset;
                self.advance();
                closed = true;
                if non_bmp {
                    self.error(start, "non-BMP characters are not supported in literals");
                } else if quote == '\''
                    && character_count(&self.source[content_start..content_end]) > 1
                {
                    self.error(
                        start,
                        "a character literal must contain at most one character",
                    );
                } else {
                    self.emit(
                        start,
                        if quote == '\'' {
                            TokenKind::CharacterLiteral
                        } else {
                            TokenKind::StringLiteral
                        },
                    );
                }
                break;
            }
            if matches!(character, '\n' | '\r') {
                break;
            }
            non_bmp |= u32::from(character) > u32::from('\u{FFFF}');
            if character == '\\' {
                self.advance();
                non_bmp |= self
                    .current()
                    .is_some_and(|value| u32::from(value) > u32::from('\u{FFFF}'));
                if self
                    .current()
                    .is_none_or(|value| matches!(value, '\n' | '\r'))
                {
                    break;
                }
            }
            self.advance();
        }
        if !closed {
            self.error(
                start,
                if quote == '\'' {
                    "unterminated character literal"
                } else {
                    "unterminated string literal"
                },
            );
        }
    }

    fn block_comment(&mut self) {
        self.advance();
        self.advance();
        while !self.is_at_end() {
            if self.remaining().starts_with("*/") {
                self.advance();
                self.advance();
                return;
            }
            self.advance();
        }
    }

    fn operator_or_error(&mut self, start: SourceLocation, character: char) {
        self.advance();
        let kind = match character {
            '+' => Some(TokenKind::Plus),
            '-' => Some(TokenKind::Minus),
            '*' => Some(TokenKind::Star),
            '/' => Some(TokenKind::Slash),
            '%' => Some(TokenKind::Percent),
            '^' => Some(TokenKind::Caret),
            '~' => Some(TokenKind::Tilde),
            '(' => Some(TokenKind::LeftParen),
            ')' => Some(TokenKind::RightParen),
            '[' => Some(TokenKind::LeftBracket),
            ']' => Some(TokenKind::RightBracket),
            ',' => Some(TokenKind::Comma),
            ';' => Some(TokenKind::Semicolon),
            ':' if self.consume_if('=') => Some(TokenKind::Assign),
            ':' => Some(TokenKind::Colon),
            '=' => Some(TokenKind::Equal),
            '<' if self.consume_if('=') => Some(TokenKind::LessEqual),
            '<' if self.consume_if('>') => Some(TokenKind::NotEqual),
            '<' => Some(TokenKind::Less),
            '>' if self.consume_if('=') => Some(TokenKind::GreaterEqual),
            '>' => Some(TokenKind::Greater),
            '|' if self.consume_if('|') => {
                self.error(start, "symbolic logical operators are not supported");
                return;
            }
            '|' => Some(TokenKind::Pipe),
            '&' if self.consume_if('&') => {
                self.error(start, "symbolic logical operators are not supported");
                return;
            }
            '&' => Some(TokenKind::Ampersand),
            _ => None,
        };
        if let Some(kind) = kind {
            self.emit(start, kind);
        } else {
            self.error(start, format!("unexpected character '{character}'"));
        }
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.current() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn is_at_end(&self) -> bool {
        self.offset >= self.source.len()
    }
}

fn is_identifier_continue(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn character_count(literal: &str) -> usize {
    let mut count = 0;
    let mut escaped = false;
    for character in literal.chars() {
        if escaped {
            count += 1;
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            count += 1;
        }
    }
    count
}
