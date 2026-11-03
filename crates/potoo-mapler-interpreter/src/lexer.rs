use crate::{Diagnostic, DiagnosticKind};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TokenKind {
    Word(String),
    Integer(i64),
    Real(f64),
    String(String),
    Symbol(&'static str),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

pub(crate) fn lex(source: &str, max_string_bytes: usize) -> Result<Vec<Token>, Diagnostic> {
    Lexer {
        characters: source.chars().collect(),
        index: 0,
        line: 1,
        column: 1,
        max_string_bytes,
    }
    .lex()
}

struct Lexer {
    characters: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    max_string_bytes: usize,
}

impl Lexer {
    fn lex(mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        while let Some(character) = self.peek() {
            if character.is_whitespace() {
                self.advance();
                continue;
            }
            if character == '/' && self.peek_next() == Some('/') {
                while !matches!(self.peek(), None | Some('\n')) {
                    self.advance();
                }
                continue;
            }

            let line = self.line;
            let column = self.column;
            let kind = if character.is_alphabetic() || character == '_' {
                self.word()
            } else if character.is_ascii_digit() {
                self.number(line, column)?
            } else if character == '"' {
                self.string(line, column)?
            } else {
                self.symbol(line, column)?
            };
            tokens.push(Token { kind, line, column });
        }
        tokens.push(Token {
            kind: TokenKind::End,
            line: self.line,
            column: self.column,
        });
        Ok(tokens)
    }

    fn word(&mut self) -> TokenKind {
        let mut word = String::new();
        while self
            .peek()
            .is_some_and(|character| character.is_alphanumeric() || character == '_')
        {
            word.push(self.advance().unwrap_or_default());
        }
        TokenKind::Word(word)
    }

    fn number(&mut self, line: usize, column: usize) -> Result<TokenKind, Diagnostic> {
        let start = self.index;
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance();
        }
        let real = self.peek() == Some('.')
            && self
                .peek_next()
                .is_some_and(|character| character.is_ascii_digit());
        if real {
            self.advance();
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.advance();
            }
        }
        let text: String = self.characters[start..self.index].iter().collect();
        if real {
            let value = text.parse::<f64>().map_err(|_| {
                Diagnostic::new(
                    DiagnosticKind::Lexical,
                    "literal real inválido",
                    line,
                    column,
                )
            })?;
            if !value.is_finite() {
                return Err(Diagnostic::new(
                    DiagnosticKind::Lexical,
                    "literal real fora do intervalo permitido",
                    line,
                    column,
                ));
            }
            Ok(TokenKind::Real(value))
        } else {
            text.parse::<i64>().map(TokenKind::Integer).map_err(|_| {
                Diagnostic::new(
                    DiagnosticKind::Lexical,
                    "literal inteiro fora do intervalo permitido",
                    line,
                    column,
                )
            })
        }
    }

    fn string(&mut self, line: usize, column: usize) -> Result<TokenKind, Diagnostic> {
        self.advance();
        let mut value = String::new();
        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    return Ok(TokenKind::String(value));
                }
                Some('\n' | '\r') | None => {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Lexical,
                        "texto sem aspas de fechamento",
                        line,
                        column,
                    ));
                }
                Some(character) => {
                    if value
                        .len()
                        .checked_add(character.len_utf8())
                        .is_none_or(|length| length > self.max_string_bytes)
                    {
                        return Err(Diagnostic::new(
                            DiagnosticKind::Lexical,
                            "texto excede o limite permitido",
                            line,
                            column,
                        ));
                    }
                    self.advance();
                    value.push(character);
                }
            }
        }
    }

    fn symbol(&mut self, line: usize, column: usize) -> Result<TokenKind, Diagnostic> {
        let first = self.advance().unwrap_or_default();
        let second = self.peek();
        let token = match (first, second) {
            ('<', Some('-')) => {
                self.advance();
                "<-"
            }
            ('<', Some('=')) => {
                self.advance();
                "<="
            }
            ('>', Some('=')) | ('=', Some('>')) => {
                self.advance();
                ">="
            }
            ('.', Some('.')) => {
                self.advance();
                ".."
            }
            ('+', _) => "+",
            ('-', _) => "-",
            ('*', _) => "*",
            ('/', _) => "/",
            ('=', _) => "=",
            ('<', _) => "<",
            ('>', _) => ">",
            ('(', _) => "(",
            (')', _) => ")",
            ('[', _) => "[",
            (']', _) => "]",
            (',', _) => ",",
            (':', _) => ":",
            (';', _) => ";",
            ('.', _) => ".",
            _ => {
                return Err(Diagnostic::new(
                    DiagnosticKind::Lexical,
                    format!("símbolo inesperado '{first}'"),
                    line,
                    column,
                ));
            }
        };
        Ok(TokenKind::Symbol(token))
    }

    fn peek(&self) -> Option<char> {
        self.characters.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.characters.get(self.index + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.index += 1;
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(character)
    }
}
