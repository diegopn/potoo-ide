use crate::{Diagnostic, DiagnosticKind};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TokenKind {
    Word(String),
    Integer(String),
    Real(String),
    Text(String),
    Character(u16),
    Symbol(String),
    End,
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl Token {
    pub(crate) fn is_word(&self, expected: &str) -> bool {
        matches!(&self.kind, TokenKind::Word(word) if word == expected)
    }

    pub(crate) fn is_symbol(&self, expected: &str) -> bool {
        matches!(&self.kind, TokenKind::Symbol(symbol) if symbol == expected)
    }
}

pub(crate) fn lex(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    Lexer::new(source).run()
}

struct Lexer {
    source: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    errors: Vec<Diagnostic>,
    tokens: Vec<Token>,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            errors: Vec::new(),
            tokens: Vec::new(),
        }
    }

    fn run(mut self) -> Result<Vec<Token>, Vec<Diagnostic>> {
        while let Some(character) = self.peek(0) {
            if character.is_whitespace() {
                self.advance();
            } else if character == '/' && self.peek(1) == Some('/') {
                self.line_comment();
            } else if character == '/' && self.peek(1) == Some('*') {
                self.block_comment();
            } else if character == '"' {
                self.quoted_text();
            } else if character == '\'' {
                self.quoted_character();
            } else if character.is_ascii_digit() {
                self.number();
            } else if character.is_alphabetic() || character == '_' {
                self.word();
            } else {
                self.symbol();
            }
        }
        self.tokens.push(Token {
            kind: TokenKind::End,
            line: self.line,
            column: self.column,
        });
        if self.errors.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.errors)
        }
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.source.get(self.index + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek(0)?;
        self.index += 1;
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(character)
    }

    fn push(&mut self, kind: TokenKind, line: usize, column: usize) {
        self.tokens.push(Token { kind, line, column });
    }

    fn error(&mut self, message: impl Into<String>, line: usize, column: usize) {
        self.errors.push(Diagnostic::new(
            DiagnosticKind::Lexical,
            message,
            line,
            column,
        ));
    }

    fn line_comment(&mut self) {
        while self.peek(0).is_some_and(|character| character != '\n') {
            self.advance();
        }
    }

    fn block_comment(&mut self) {
        let line = self.line;
        let column = self.column;
        self.advance();
        self.advance();
        while self.peek(0).is_some() {
            if self.peek(0) == Some('*') && self.peek(1) == Some('/') {
                self.advance();
                self.advance();
                return;
            }
            self.advance();
        }
        self.error("comentário de bloco sem fechamento", line, column);
    }

    fn quoted_text(&mut self) {
        let line = self.line;
        let column = self.column;
        self.advance();
        let mut value = String::new();
        let mut closed = false;
        while let Some(character) = self.advance() {
            match character {
                '"' => {
                    closed = true;
                    break;
                }
                '\\' => match self.advance() {
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('t') => value.push('\t'),
                    Some('0') => value.push('\0'),
                    Some(escaped) => value.push(escaped),
                    None => break,
                },
                '\n' => {
                    self.error("texto literal não pode atravessar uma linha", line, column);
                    break;
                }
                character => value.push(character),
            }
        }
        if closed {
            self.push(TokenKind::Text(value), line, column);
        } else if !self
            .errors
            .iter()
            .any(|error| error.line == line && error.column == column)
        {
            self.error("texto literal sem fechamento", line, column);
        }
    }

    fn quoted_character(&mut self) {
        let line = self.line;
        let column = self.column;
        self.advance();
        let value = match self.advance() {
            Some('\\') => match self.advance() {
                Some('n') => Some('\n'),
                Some('r') => Some('\r'),
                Some('t') => Some('\t'),
                Some(escaped) => Some(escaped),
                None => None,
            },
            Some('\'') | Some('\n') | None => None,
            Some(character) => Some(character),
        };
        if self.peek(0) != Some('\'') {
            self.error(
                "literal de caractere deve conter exatamente um caractere",
                line,
                column,
            );
            while self
                .peek(0)
                .is_some_and(|character| character != '\'' && character != '\n')
            {
                self.advance();
            }
        }
        if self.peek(0) == Some('\'') {
            self.advance();
        }
        if let Some(character) = value {
            let mut units = [0; 2];
            let encoded = character.encode_utf16(&mut units);
            if encoded.len() != 1 {
                self.error(
                    "literal de caractere deve caber em uma unidade UTF-16",
                    line,
                    column,
                );
            } else {
                self.push(TokenKind::Character(encoded[0]), line, column);
            }
        }
    }

    fn number(&mut self) {
        let line = self.line;
        let column = self.column;
        let mut value = String::new();
        while self
            .peek(0)
            .is_some_and(|character| character.is_ascii_digit())
        {
            value.push(self.advance().unwrap_or_default());
        }
        let mut is_real = false;
        if self.peek(0) == Some('.') && self.peek(1).is_some_and(|c| c.is_ascii_digit()) {
            is_real = true;
            value.push(self.advance().unwrap_or_default());
            while self
                .peek(0)
                .is_some_and(|character| character.is_ascii_digit())
            {
                value.push(self.advance().unwrap_or_default());
            }
        }
        if matches!(self.peek(0), Some('e' | 'E')) {
            let exponent_offset =
                usize::from(self.peek(1) == Some('+') || self.peek(1) == Some('-'));
            if self
                .peek(1 + exponent_offset)
                .is_some_and(|character| character.is_ascii_digit())
            {
                is_real = true;
                value.push(self.advance().unwrap_or_default());
                if matches!(self.peek(0), Some('+' | '-')) {
                    value.push(self.advance().unwrap_or_default());
                }
                while self
                    .peek(0)
                    .is_some_and(|character| character.is_ascii_digit())
                {
                    value.push(self.advance().unwrap_or_default());
                }
            }
        }
        self.push(
            if is_real {
                TokenKind::Real(value)
            } else {
                TokenKind::Integer(value)
            },
            line,
            column,
        );
    }

    fn word(&mut self) {
        let line = self.line;
        let column = self.column;
        let mut word = String::new();
        while self
            .peek(0)
            .is_some_and(|character| character.is_alphanumeric() || character == '_')
        {
            word.push(self.advance().unwrap_or_default());
        }
        self.push(TokenKind::Word(word), line, column);
    }

    fn symbol(&mut self) {
        let line = self.line;
        let column = self.column;
        let character = self.advance().unwrap_or_default();
        if matches!(character, '=' | '!' | '<' | '>') && self.peek(0) == Some('=') {
            let mut symbol = String::from(character);
            symbol.push(self.advance().unwrap_or_default());
            self.push(TokenKind::Symbol(symbol), line, column);
        } else if matches!(
            character,
            '(' | ')'
                | '['
                | ']'
                | ','
                | ';'
                | ':'
                | '='
                | '<'
                | '>'
                | '+'
                | '-'
                | '*'
                | '/'
                | '\\'
                | '%'
                | '!'
        ) {
            self.push(TokenKind::Symbol(character.to_string()), line, column);
        } else {
            self.error(format!("caractere inesperado '{character}'"), line, column);
        }
    }
}
