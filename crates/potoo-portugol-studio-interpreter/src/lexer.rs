use crate::{Diagnostic, DiagnosticKind};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Integer(i64),
    Real(f64),
    String(String),
    Character(char),
    Newline,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,
    Ampersand,
    Pipe,
    Caret,
    BitNot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusPlus,
    MinusMinus,
    Arrow,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    ShiftLeft,
    Greater,
    GreaterEqual,
    ShiftRight,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

struct Scanner {
    chars: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
    token_line: usize,
    token_column: usize,
    tokens: Vec<Token>,
}

impl Scanner {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
            token_line: 1,
            token_column: 1,
            tokens: Vec::new(),
        }
    }

    fn scan(mut self) -> Result<Vec<Token>, Vec<Diagnostic>> {
        while !self.is_at_end() {
            self.token_line = self.line;
            self.token_column = self.column;
            let character = self.advance();
            match character {
                ' ' | '\t' | '\r' => {}
                '\n' => {}
                '(' => self.push(TokenKind::LeftParen),
                ')' => self.push(TokenKind::RightParen),
                '{' => self.push(TokenKind::LeftBrace),
                '}' => self.push(TokenKind::RightBrace),
                '[' => self.push(TokenKind::LeftBracket),
                ']' => self.push(TokenKind::RightBracket),
                ',' => self.push(TokenKind::Comma),
                '.' if self.peek().is_some_and(|next| next.is_ascii_digit()) => self.number('.')?,
                '.' => self.push(TokenKind::Dot),
                ':' => self.push(TokenKind::Colon),
                ';' => self.push(TokenKind::Semicolon),
                '&' => self.push(TokenKind::Ampersand),
                '|' => self.push(TokenKind::Pipe),
                '^' => self.push(TokenKind::Caret),
                '~' => self.push(TokenKind::BitNot),
                '+' => {
                    let kind = if self.matches('+') {
                        TokenKind::PlusPlus
                    } else if self.matches('=') {
                        TokenKind::PlusEqual
                    } else {
                        TokenKind::Plus
                    };
                    self.push(kind);
                }
                '-' => {
                    let kind = if self.matches('-') {
                        if self.matches('>') {
                            TokenKind::Arrow
                        } else {
                            TokenKind::MinusMinus
                        }
                    } else if self.matches('=') {
                        TokenKind::MinusEqual
                    } else {
                        TokenKind::Minus
                    };
                    self.push(kind);
                }
                '*' => {
                    let kind = if self.matches('=') {
                        TokenKind::StarEqual
                    } else {
                        TokenKind::Star
                    };
                    self.push(kind);
                }
                '/' => {
                    if self.matches('/') {
                        while self.peek() != Some('\n') && !self.is_at_end() {
                            self.advance();
                        }
                    } else if self.matches('*') {
                        self.block_comment()?;
                    } else {
                        let kind = if self.matches('=') {
                            TokenKind::SlashEqual
                        } else {
                            TokenKind::Slash
                        };
                        self.push(kind);
                    }
                }
                '%' => self.push(TokenKind::Percent),
                '\\' => return Err(self.error("caractere inesperado '\\\\'")),
                '=' => {
                    let kind = if self.matches('=') {
                        TokenKind::EqualEqual
                    } else {
                        TokenKind::Equal
                    };
                    self.push(kind);
                }
                '!' if self.matches('=') => self.push(TokenKind::BangEqual),
                '!' => return Err(self.error("caractere inesperado '!'")),
                '<' => {
                    let kind = if self.matches('=') {
                        TokenKind::LessEqual
                    } else if self.matches('<') {
                        TokenKind::ShiftLeft
                    } else {
                        TokenKind::Less
                    };
                    self.push(kind);
                }
                '>' => {
                    let kind = if self.matches('=') {
                        TokenKind::GreaterEqual
                    } else if self.matches('>') {
                        TokenKind::ShiftRight
                    } else {
                        TokenKind::Greater
                    };
                    self.push(kind);
                }
                '"' => self.string_literal()?,
                '\'' => self.character_literal()?,
                character if character.is_ascii_digit() => self.number(character)?,
                character if is_identifier_start(character) => self.identifier(character),
                character => {
                    return Err(vec![Diagnostic::new(
                        DiagnosticKind::Lexical,
                        format!("caractere inesperado '{character}'"),
                        self.token_line,
                        self.token_column,
                    )]);
                }
            }
        }
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            line: self.line,
            column: self.column,
        });
        Ok(self.tokens)
    }

    fn identifier(&mut self, first: char) {
        let mut value = String::from(first);
        while self.peek().is_some_and(is_identifier_continue) {
            value.push(self.advance());
        }
        self.push(TokenKind::Identifier(value));
    }

    fn number(&mut self, first: char) -> Result<(), Vec<Diagnostic>> {
        if first == '0'
            && self
                .peek()
                .is_some_and(|character| matches!(character, 'x' | 'X'))
        {
            self.advance();
            let mut digits = String::new();
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_hexdigit())
            {
                digits.push(self.advance());
            }
            if digits.is_empty() {
                return Err(self.error("literal hexadecimal inválido"));
            }
            if digits.len() > 6 {
                return Err(self.error("literal hexadecimal inválido"));
            }
            let value = i64::from_str_radix(&digits, 16).map_err(|_| {
                vec![Diagnostic::new(
                    DiagnosticKind::Lexical,
                    "literal inteiro fora do limite",
                    self.token_line,
                    self.token_column,
                )]
            })?;
            self.push(TokenKind::Integer(value));
            return Ok(());
        }
        let mut value = String::from(first);
        let starts_with_decimal = first == '.';
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            value.push(self.advance());
        }

        if !starts_with_decimal && self.peek() == Some('.') {
            value.push(self.advance());
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                value.push(self.advance());
            }
        }
        if starts_with_decimal || value.contains('.') {
            let number = value.parse::<f64>().map_err(|_| {
                vec![Diagnostic::new(
                    DiagnosticKind::Lexical,
                    "número real inválido",
                    self.token_line,
                    self.token_column,
                )]
            })?;
            self.push(TokenKind::Real(number));
        } else {
            let number = value.parse::<i32>().map_err(|_| {
                vec![Diagnostic::new(
                    DiagnosticKind::Lexical,
                    "número inteiro fora do limite",
                    self.token_line,
                    self.token_column,
                )]
            })?;
            self.push(TokenKind::Integer(i64::from(number)));
        }
        Ok(())
    }

    fn string_literal(&mut self) -> Result<(), Vec<Diagnostic>> {
        let mut value = String::new();
        while let Some(character) = self.peek() {
            if character == '"' {
                self.advance();
                self.push(TokenKind::String(value));
                return Ok(());
            }
            if character == '\\' {
                self.advance();
                value.push(self.escape_character()?);
            } else {
                value.push(self.advance());
            }
        }
        Err(self.error("texto não terminado"))
    }

    fn character_literal(&mut self) -> Result<(), Vec<Diagnostic>> {
        let character = match self.peek() {
            Some('\\') if self.peek_next() == Some('\'') && self.peek_third() != Some('\'') => {
                self.advance()
            }
            Some('\\') => {
                self.advance();
                self.escape_character()?
            }
            None => return Err(self.error("caractere não terminado")),
            Some(_) => self.advance(),
        };
        if !self.matches('\'') {
            return Err(self.error("literal de caracter deve conter um caractere"));
        }
        self.push(TokenKind::Character(character));
        Ok(())
    }

    fn escape_character(&mut self) -> Result<char, Vec<Diagnostic>> {
        if self.is_at_end() {
            return Err(self.error("sequência de escape incompleta"));
        }
        match self.advance() {
            'b' => Ok('\u{0008}'),
            'f' => Ok('\u{000c}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            '\\' => Ok('\\'),
            '"' => Ok('"'),
            '\'' => Ok('\''),
            'u' => {
                let code_unit = self.unicode_code_unit()?;
                let codepoint = if (0xD800..=0xDBFF).contains(&code_unit) {
                    if self.peek() != Some('\\') || self.peek_next() != Some('u') {
                        return Err(self.error("sequência Unicode alta sem par de substituição"));
                    }
                    self.advance();
                    self.advance();
                    let low = self.unicode_code_unit()?;
                    if !(0xDC00..=0xDFFF).contains(&low) {
                        return Err(self.error("par de substituição Unicode inválido"));
                    }
                    0x10000 + ((code_unit - 0xD800) << 10) + (low - 0xDC00)
                } else if (0xDC00..=0xDFFF).contains(&code_unit) {
                    return Err(self.error("sequência Unicode baixa sem par de substituição"));
                } else {
                    code_unit
                };
                char::from_u32(codepoint)
                    .ok_or_else(|| self.error("sequência Unicode não representa um caractere"))
            }
            first @ '0'..='7' => {
                let mut value = first as u32 - '0' as u32;
                let maximum_digits = if first <= '3' { 3 } else { 2 };
                let mut digits = 1;
                while digits < maximum_digits
                    && self
                        .peek()
                        .is_some_and(|character| matches!(character, '0'..='7'))
                {
                    value = value * 8 + (self.advance() as u32 - '0' as u32);
                    digits += 1;
                }
                char::from_u32(value)
                    .ok_or_else(|| self.error("escape octal fora do limite de caractere"))
            }
            character => Err(self.error(format!("sequência de escape inválida '\\{character}'"))),
        }
    }

    fn unicode_code_unit(&mut self) -> Result<u32, Vec<Diagnostic>> {
        let mut digits = String::with_capacity(4);
        for _ in 0..4 {
            let Some(character) = self.peek() else {
                return Err(self.error("sequência Unicode incompleta"));
            };
            if !character.is_ascii_hexdigit() {
                return Err(self.error("sequência Unicode inválida"));
            }
            digits.push(self.advance());
        }
        u32::from_str_radix(&digits, 16).map_err(|_| self.error("sequência Unicode inválida"))
    }

    fn block_comment(&mut self) -> Result<(), Vec<Diagnostic>> {
        while !self.is_at_end() {
            if self.peek() == Some('*') && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(self.error("comentário de bloco não terminado"))
    }

    fn error(&self, message: impl Into<String>) -> Vec<Diagnostic> {
        vec![Diagnostic::new(
            DiagnosticKind::Lexical,
            message,
            self.token_line,
            self.token_column,
        )]
    }

    fn push(&mut self, kind: TokenKind) {
        self.tokens.push(Token {
            kind,
            line: self.token_line,
            column: self.token_column,
        });
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.chars.len()
    }

    fn advance(&mut self) -> char {
        let character = self.chars[self.current];
        self.current += 1;
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        character
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.current).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.current + 1).copied()
    }

    fn peek_third(&self) -> Option<char> {
        self.chars.get(self.current + 2).copied()
    }

    fn matches(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }
}

pub fn scan(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    Scanner::new(source).scan()
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}
