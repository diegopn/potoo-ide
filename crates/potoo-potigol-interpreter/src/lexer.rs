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
                '\n' | '\r' => {
                    self.advance();
                    self.push(TokenKind::Newline, start);
                }
                ' ' | '\t' | '\u{000c}' => {
                    self.advance();
                }
                '#' => self.skip_comment(),
                '/' if self.peek(1) == Some('/') => self.skip_comment(),
                '"' => self.scan_string(start)?,
                '\'' => self.scan_character(start)?,
                character if is_identifier_start(character) => self.scan_identifier(start),
                character if character.is_ascii_digit() => self.scan_number(start)?,
                '.' if self.peek(1).is_some_and(|next| next.is_ascii_digit()) => {
                    self.scan_number(start)?;
                }
                '(' | ')' | '[' | ']' | '{' | '}' | ',' | ':' | ';' | '.' => {
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

        if !is_real && self.current() == Some('.') && self.peek(1) != Some('.') {
            is_real = true;
            raw.push(self.advance().unwrap_or('.'));
            while self
                .current()
                .is_some_and(|character| character.is_ascii_digit())
            {
                raw.push(self.advance().unwrap_or_default());
            }
        }

        if self
            .current()
            .is_some_and(|character| matches!(character, 'e' | 'E'))
        {
            is_real = true;
            raw.push(self.advance().unwrap_or_default());
            if self
                .current()
                .is_some_and(|character| matches!(character, '+' | '-'))
            {
                raw.push(self.advance().unwrap_or_default());
            }
            let exponent_start = raw.len();
            while self
                .current()
                .is_some_and(|character| character.is_ascii_digit())
            {
                raw.push(self.advance().unwrap_or_default());
            }
            if raw.len() == exponent_start {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "expoente de número real incompleto",
                    start,
                )]);
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
            Some('\\') => {
                self.advance();
                self.scan_escape(start)?
            }
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
        let multiline = self.peek(1) == Some('"') && self.peek(2) == Some('"');
        self.advance();
        if multiline {
            self.advance();
            self.advance();
        }

        let mut value = String::new();
        loop {
            let Some(character) = self.current() else {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "texto não terminado",
                    start,
                )]);
            };
            if multiline && self.is_triple_quote() {
                self.advance();
                self.advance();
                self.advance();
                break;
            }
            if !multiline && character == '"' {
                self.advance();
                break;
            }
            if !multiline && matches!(character, '\n' | '\r') {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "texto de linha única não pode conter uma quebra de linha",
                    start,
                )]);
            }
            if character == '\\' {
                value.push(self.advance().unwrap_or('\\'));
                if let Some(escaped) = self.current() {
                    value.push(self.advance().unwrap_or(escaped));
                } else {
                    return Err(vec![self.error(
                        DiagnosticKind::Lexical,
                        "sequência de escape incompleta",
                        start,
                    )]);
                }
                continue;
            }
            if character == '{' {
                self.scan_interpolation(multiline, &mut value)?;
                continue;
            }
            value.push(self.advance().unwrap_or(character));
        }

        self.push(TokenKind::String(value), start);
        Ok(())
    }

    fn scan_interpolation(
        &mut self,
        multiline: bool,
        value: &mut String,
    ) -> Result<(), Vec<Diagnostic>> {
        let interpolation_start = self.location();
        let mut depth = 0usize;
        value.push(self.advance().unwrap_or('{'));
        loop {
            let Some(character) = self.current() else {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "interpolação não terminada",
                    interpolation_start,
                )]);
            };
            if !multiline && matches!(character, '\n' | '\r') {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "interpolação não terminada antes da quebra de linha",
                    interpolation_start,
                )]);
            }
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
                    value.push(self.advance().unwrap_or_default());
                }
                continue;
            }
            match character {
                '{' => {
                    depth = depth.saturating_add(1);
                    value.push(self.advance().unwrap_or('{'));
                }
                '}' if depth == 0 => {
                    value.push(self.advance().unwrap_or('}'));
                    return Ok(());
                }
                '}' => {
                    depth -= 1;
                    value.push(self.advance().unwrap_or('}'));
                }
                _ => value.push(self.advance().unwrap_or(character)),
            }
        }
    }

    fn copy_nested_string(&mut self, value: &mut String) -> Result<(), Vec<Diagnostic>> {
        let start = self.location();
        let multiline = self.peek(1) == Some('"') && self.peek(2) == Some('"');
        value.push(self.advance().unwrap_or('"'));
        if multiline {
            value.push(self.advance().unwrap_or('"'));
            value.push(self.advance().unwrap_or('"'));
        }
        loop {
            let Some(character) = self.current() else {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "texto dentro de interpolação não terminado",
                    start,
                )]);
            };
            if character == '\\' {
                value.push(self.advance().unwrap_or('\\'));
                if let Some(escaped) = self.current() {
                    value.push(self.advance().unwrap_or(escaped));
                }
                continue;
            }
            if multiline && self.is_triple_quote() {
                value.push(self.advance().unwrap_or('"'));
                value.push(self.advance().unwrap_or('"'));
                value.push(self.advance().unwrap_or('"'));
                return Ok(());
            }
            if !multiline && character == '"' {
                value.push(self.advance().unwrap_or('"'));
                return Ok(());
            }
            if !multiline && matches!(character, '\n' | '\r') {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "texto de linha única não terminado dentro da interpolação",
                    start,
                )]);
            }
            value.push(self.advance().unwrap_or(character));
        }
    }

    fn copy_nested_character(&mut self, value: &mut String) -> Result<(), Vec<Diagnostic>> {
        let start = self.location();
        value.push(self.advance().unwrap_or('\''));
        if self.current() == Some('\\') {
            value.push(self.advance().unwrap_or('\\'));
            if let Some(escaped) = self.current() {
                value.push(self.advance().unwrap_or(escaped));
            }
        } else if let Some(character) = self.current() {
            if matches!(character, '\n' | '\r') {
                return Err(vec![self.error(
                    DiagnosticKind::Lexical,
                    "literal de caractere não terminado dentro da interpolação",
                    start,
                )]);
            }
            value.push(self.advance().unwrap_or(character));
        }
        if self.current() != Some('\'') {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "literal de caractere não terminado dentro da interpolação",
                start,
            )]);
        }
        value.push(self.advance().unwrap_or('\''));
        Ok(())
    }

    fn scan_escape(&mut self, start: SourceLocation) -> Result<char, Vec<Diagnostic>> {
        let Some(character) = self.advance() else {
            return Err(vec![self.error(
                DiagnosticKind::Lexical,
                "sequência de escape incompleta",
                start,
            )]);
        };
        match character {
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            '0' => Ok('\0'),
            '\\' => Ok('\\'),
            '\'' => Ok('\''),
            '"' => Ok('"'),
            other => Err(vec![self.error(
                DiagnosticKind::Lexical,
                format!("sequência de escape desconhecida: \\{other}"),
                start,
            )]),
        }
    }

    fn scan_compound_operator(&mut self) -> Option<String> {
        const OPERATORS: [&str; 12] = [
            "::", ":=", "=>", "<=", ">=", "<>", "==", "!=", "&&", "||", "->", "..",
        ];
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

    fn skip_comment(&mut self) {
        if self.current() == Some('/') {
            self.advance();
            self.advance();
        } else {
            self.advance();
        }
        while self
            .current()
            .is_some_and(|character| !matches!(character, '\n' | '\r'))
        {
            self.advance();
        }
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
}

/// Converts Potigol source into tokens with one-based source locations.
pub fn lex(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    Scanner::new(source).scan()
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_ascii_digit()
}
