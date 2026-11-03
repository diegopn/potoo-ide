use crate::{Diagnostic, DiagnosticKind, SourceLocation, SourceSpan};

/// A lexical unit from a Potigol source file.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

/// Token category. Keywords remain identifiers so parser rules can decide their context.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Integer(i64),
    Real(f64),
    Character(char),
    /// String contents retain escape sequences and interpolation braces for later parsing.
    String(String),
    Symbol(char),
    Operator(String),
    Newline,
    Eof,
}

struct Scanner {
    characters: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    byte_offset: usize,
    tokens: Vec<Token>,
}

impl Scanner {
    fn new(source: &str) -> Self {
        Self {
            characters: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            byte_offset: 0,
            tokens: Vec::new(),
        }
    }

    fn scan(mut self) -> Result<Vec<Token>, Vec<Diagnostic>> {
        while let Some(character) = self.current() {
            let start = self.location();
            match character {
                '\n' => {
                    self.advance();
                    self.push(TokenKind::Newline, start);
                }
                '\r' if self.peek(1) == Some('\n') => {
                    self.advance();
                    self.push(TokenKind::Newline, start);
                }
                '\r' => {
                    self.advance();
                }
                ' ' | '\t' | '\u{000c}' => {
                    self.advance();
                }
                '#' => self.skip_comment(start)?,
                '"' => self.scan_string(start)?,
                '\'' => self.scan_character(start)?,
                character if is_identifier_start(character) => self.scan_identifier(start),
                character if character.is_ascii_digit() => self.scan_number(start)?,
                '.' if self.peek(1).is_some_and(|next| next.is_ascii_digit()) => {
                    self.scan_number(start)?;
                }
                '(' | ')' | '[' | ']' | '{' | '}' | ',' | ':' | '.' => {
                    if let Some(operator) = self.scan_compound_operator() {
                        self.push(TokenKind::Operator(operator), start);
                    } else {
                        let symbol = self.advance().unwrap_or(character);
                        self.push(TokenKind::Symbol(symbol), start);
                    }
                }
                '+' | '-' | '*' | '/' | '\\' | '^' | '%' | '=' | '<' | '>' | '!' | '&' | '|'
                | '~' => {
                    let operator = self
                        .scan_compound_operator()
                        .unwrap_or_else(|| self.advance().unwrap_or(character).to_string());
                    self.push(TokenKind::Operator(operator), start);
                }
                other => {
                    return Err(vec![self.error(
                        DiagnosticKind::Lexical,
                        format!("caractere inválido: '{other}'"),
                        start,
                    )]);
                }
            }
        }

        let location = self.location();
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: SourceSpan {
                start: location,
                end: location,
            },
        });
        Ok(self.tokens)
    }

    fn scan_identifier(&mut self, start: SourceLocation) {
        let mut value = String::new();
        while self.current().is_some_and(is_identifier_continue) {
            value.push(self.advance().unwrap_or_default());
        }
        self.push(TokenKind::Identifier(value), start);
    }

    fn scan_number(&mut self, start: SourceLocation) -> Result<(), Vec<Diagnostic>> {
        let mut raw = String::new();
        let mut is_real = false;

        if self.current() == Some('.') {
            is_real = true;
            raw.push(self.advance().unwrap_or('.'));
        }
        while self
            .current()
            .is_some_and(|character| character.is_ascii_digit())
        {
            raw.push(self.advance().unwrap_or_default());
        }

        if !is_real
            && self.current() == Some('.')
            && self
                .peek(1)
                .is_some_and(|character| character.is_ascii_digit())
        {
            is_real = true;
            raw.push(self.advance().unwrap_or('.'));
            while self
                .current()
                .is_some_and(|character| character.is_ascii_digit())
            {
                raw.push(self.advance().unwrap_or_default());
            }
        }

        if is_real {
            match raw.parse::<f64>() {
                Ok(value) if value.is_finite() => self.push(TokenKind::Real(value), start),
                _ => {
                    return Err(vec![self.error(
                        DiagnosticKind::Lexical,
                        "número real inválido ou fora do limite",
                        start,
                    )]);
                }
            }
        } else {
            match raw.parse::<i64>() {
                Ok(value) => self.push(TokenKind::Integer(value), start),
                Err(_) => {
                    return Err(vec![self.error(
                        DiagnosticKind::Lexical,
                        "número inteiro fora do limite",
                        start,
                    )]);
                }
            }
        }
        Ok(())
    }

    fn scan_character(&mut self, start: SourceLocation) -> Result<(), Vec<Diagnostic>> {
        self.advance();
        let value = match self.current() {
            Some('\n' | '\r') | None => {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "literal de caractere não terminado",
                    start,
                )]);
            }
            Some(character) => {
                self.advance();
                character
            }
        };
        if u32::from(value) > u32::from(u16::MAX) {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "caractere fora do limite UTF-16",
                start,
            )]);
        }
        if self.current() != Some('\'') {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "literal de caractere deve conter exatamente um caractere",
                start,
            )]);
        }
        self.advance();
        self.push(TokenKind::Character(value), start);
        Ok(())
    }

    fn scan_string(&mut self, start: SourceLocation) -> Result<(), Vec<Diagnostic>> {
        if self.is_triple_quote() {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "aspas triplas não fazem parte da gramática Potigol",
                start,
            )]);
        }
        self.advance();

        let mut value = String::new();
        loop {
            let Some(character) = self.current() else {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "texto não terminado",
                    start,
                )]);
            };
            if character == '"' {
                self.advance();
                break;
            }
            if character == '{' {
                self.scan_interpolation(&mut value)?;
                continue;
            }
            value.push(self.advance_raw().unwrap_or(character));
        }

        self.push(TokenKind::String(value), start);
        Ok(())
    }

    fn scan_interpolation(&mut self, value: &mut String) -> Result<(), Vec<Diagnostic>> {
        let interpolation_start = self.location();
        let mut depth = 0usize;
        value.push(self.advance_raw().unwrap_or('{'));
        loop {
            let Some(character) = self.current() else {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "interpolação não terminada",
                    interpolation_start,
                )]);
            };
            if character == '"' {
                self.copy_nested_string(value)?;
                continue;
            }
            if character == '\'' {
                self.copy_nested_character(value)?;
                continue;
            }
            if character == '#' || (character == '/' && self.peek(1) == Some('/')) {
                while self
                    .current()
                    .is_some_and(|next| !matches!(next, '\n' | '\r'))
                {
                    value.push(self.advance_raw().unwrap_or_default());
                }
                continue;
            }
            match character {
                '{' => {
                    depth = depth.saturating_add(1);
                    value.push(self.advance_raw().unwrap_or('{'));
                }
                '}' if depth == 0 => {
                    value.push(self.advance_raw().unwrap_or('}'));
                    return Ok(());
                }
                '}' => {
                    depth -= 1;
                    value.push(self.advance_raw().unwrap_or('}'));
                }
                _ => value.push(self.advance_raw().unwrap_or(character)),
            }
        }
    }

    fn copy_nested_string(&mut self, value: &mut String) -> Result<(), Vec<Diagnostic>> {
        let start = self.location();
        if self.is_triple_quote() {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "aspas triplas não fazem parte da gramática Potigol",
                start,
            )]);
        }
        value.push(self.advance_raw().unwrap_or('"'));
        loop {
            let Some(character) = self.current() else {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "texto dentro de interpolação não terminado",
                    start,
                )]);
            };
            if character == '"' {
                value.push(self.advance_raw().unwrap_or('"'));
                return Ok(());
            }
            value.push(self.advance_raw().unwrap_or(character));
        }
    }

    fn copy_nested_character(&mut self, value: &mut String) -> Result<(), Vec<Diagnostic>> {
        let start = self.location();
        value.push(self.advance_raw().unwrap_or('\''));
        let Some(character) = self.current() else {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "literal de caractere não terminado dentro da interpolação",
                start,
            )]);
        };
        if matches!(character, '\n' | '\r' | '\'') || u32::from(character) > u32::from(u16::MAX) {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "literal de caractere inválido dentro da interpolação",
                start,
            )]);
        }
        value.push(self.advance_raw().unwrap_or(character));
        if self.current() != Some('\'') {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "literal de caractere não terminado dentro da interpolação",
                start,
            )]);
        }
        value.push(self.advance_raw().unwrap_or('\''));
        Ok(())
    }

    fn scan_compound_operator(&mut self) -> Option<String> {
        const OPERATORS: [&str; 7] = ["::", ":=", "=>", "<=", ">=", "<>", "=="];
        for operator in OPERATORS {
            if self.matches(operator) {
                for _ in operator.chars() {
                    self.advance();
                }
                return Some(operator.to_owned());
            }
        }
        None
    }

    fn skip_comment(&mut self, start: SourceLocation) -> Result<(), Vec<Diagnostic>> {
        self.advance();
        while self
            .current()
            .is_some_and(|character| !matches!(character, '\n' | '\r'))
        {
            self.advance();
        }
        if self.current().is_none() || (self.current() == Some('\r') && self.peek(1) != Some('\n'))
        {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "comentário precisa terminar com uma quebra de linha",
                start,
            )]);
        }
        Ok(())
    }

    fn push(&mut self, kind: TokenKind, start: SourceLocation) {
        self.tokens.push(Token {
            kind,
            span: SourceSpan {
                start,
                end: self.location(),
            },
        });
    }

    fn error(
        &self,
        kind: DiagnosticKind,
        message: impl Into<String>,
        start: SourceLocation,
    ) -> Diagnostic {
        Diagnostic::new(
            kind,
            message,
            SourceSpan {
                start,
                end: self.location(),
            },
        )
    }

    fn is_triple_quote(&self) -> bool {
        self.current() == Some('"') && self.peek(1) == Some('"') && self.peek(2) == Some('"')
    }

    fn matches(&self, text: &str) -> bool {
        text.chars()
            .enumerate()
            .all(|(offset, character)| self.peek(offset) == Some(character))
    }

    fn current(&self) -> Option<char> {
        self.characters.get(self.index).copied()
    }

    fn peek(&self, distance: usize) -> Option<char> {
        self.characters
            .get(self.index.saturating_add(distance))
            .copied()
    }

    fn location(&self) -> SourceLocation {
        SourceLocation {
            line: self.line,
            column: self.column,
            byte_offset: self.byte_offset,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.current()?;
        self.index += 1;
        self.byte_offset += character.len_utf8();
        match character {
            '\r' => {
                if self.current() == Some('\n') {
                    self.index += 1;
                    self.byte_offset += 1;
                }
                self.line += 1;
                self.column = 1;
                Some('\n')
            }
            '\n' => {
                self.line += 1;
                self.column = 1;
                Some('\n')
            }
            _ => {
                self.column += 1;
                Some(character)
            }
        }
    }

    fn advance_raw(&mut self) -> Option<char> {
        let character = self.current()?;
        self.index += 1;
        self.byte_offset += character.len_utf8();
        match character {
            '\r' if self.current() == Some('\n') => self.column += 1,
            '\r' | '\n' => {
                self.line += 1;
                self.column = 1;
            }
            _ => self.column += 1,
        }
        Some(character)
    }
}

/// Converts Potigol source into tokens with one-based source locations.
pub fn lex(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    Scanner::new(source).scan()
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic() || is_accent(character)
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric() || is_accent(character)
}

fn is_accent(character: char) -> bool {
    ('\u{00a1}'..='\u{fffc}').contains(&character)
}
