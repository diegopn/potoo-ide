use crate::{Diagnostic, DiagnosticKind};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TokenKind {
    Identifier(String),
    Integer(i64),
    Real(f64),
    String(String),
    Symbol(char),
    Operator(String),
    Newline,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

struct Scanner {
    characters: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
}

impl Scanner {
    fn new(source: &str) -> Self {
        Self {
            characters: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
        }
    }

    fn scan(mut self) -> Result<Vec<Token>, Vec<Diagnostic>> {
        while let Some(character) = self.current() {
            let line = self.line;
            let column = self.column;
            match character {
                '\n' => {
                    self.advance();
                    self.push(TokenKind::Newline, line, column);
                }
                '\r' if self.peek(1) == Some('\n') => {
                    self.advance();
                }
                '\r' => {
                    self.advance();
                    self.push(TokenKind::Newline, line, column);
                }
                ' ' | '\t' | '\u{000c}' => {
                    self.advance();
                }
                '/' if self.peek(1) == Some('/') => self.skip_comment(),
                '"' => self.scan_string(line, column)?,
                character if is_identifier_start(character) => {
                    self.scan_identifier(line, column);
                    if self.tokens.last().is_some_and(|token| {
                        matches!(&token.kind, TokenKind::Identifier(name) if name == "fimalgoritmo")
                    }) {
                        break;
                    }
                }
                character if character.is_ascii_digit() => {
                    self.scan_number(line, column)?;
                }
                '.' if self.peek(1).is_some_and(|next| next.is_ascii_digit()) => {
                    self.scan_number(line, column)?;
                }
                '<' | '>' => self.scan_comparison(line, column),
                '+' | '-' | '*' | '/' | '\\' | '^' | '%' | '=' => {
                    self.advance();
                    self.push(TokenKind::Operator(character.to_string()), line, column);
                }
                '(' | ')' | '[' | ']' | ',' | ':' | ';' => {
                    self.advance();
                    self.push(TokenKind::Symbol(character), line, column);
                }
                '.' if self.peek(1) == Some('.') => {
                    self.advance();
                    self.advance();
                    self.push(TokenKind::Operator("..".to_owned()), line, column);
                }
                '.' => {
                    self.advance();
                    self.push(TokenKind::Symbol('.'), line, column);
                }
                other => {
                    return Err(vec![Diagnostic::new(
                        DiagnosticKind::Lexical,
                        format!("caractere inválido: '{other}'"),
                        line,
                        column,
                    )]);
                }
            }
        }
        self.push(TokenKind::Eof, self.line, self.column);
        Ok(self.tokens)
    }

    fn scan_identifier(&mut self, line: usize, column: usize) {
        let start = self.index;
        self.advance();
        while self.current().is_some_and(is_identifier_continue) {
            self.advance();
        }
        let identifier = self.characters[start..self.index]
            .iter()
            .collect::<String>()
            .to_lowercase();
        self.push(TokenKind::Identifier(identifier), line, column);
    }

    fn scan_number(&mut self, line: usize, column: usize) -> Result<(), Vec<Diagnostic>> {
        let start = self.index;
        let mut has_decimal = false;
        if self.current() == Some('.') {
            has_decimal = true;
            self.advance();
        }
        while self
            .current()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance();
        }
        if !has_decimal && self.current() == Some('.') && self.peek(1) != Some('.') {
            has_decimal = true;
            self.advance();
            while self
                .current()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.advance();
            }
        }
        if self
            .current()
            .is_some_and(|character| matches!(character, 'e' | 'E'))
        {
            let exponent_start = self.index;
            let mut exponent_index = self.index + 1;
            if self
                .characters
                .get(exponent_index)
                .is_some_and(|character| matches!(character, '+' | '-'))
            {
                exponent_index += 1;
            }
            let digit_start = exponent_index;
            while self
                .characters
                .get(exponent_index)
                .is_some_and(|character| character.is_ascii_digit())
            {
                exponent_index += 1;
            }
            if exponent_index > digit_start {
                has_decimal = true;
                self.index = exponent_index;
                self.column += exponent_index - exponent_start;
            }
        }

        let raw = self.characters[start..self.index]
            .iter()
            .collect::<String>();
        let kind = if has_decimal {
            match raw.parse::<f64>() {
                Ok(value) if value.is_finite() => TokenKind::Real(value),
                _ => {
                    return Err(vec![Diagnostic::new(
                        DiagnosticKind::Lexical,
                        "número real inválido ou fora do limite",
                        line,
                        column,
                    )]);
                }
            }
        } else {
            match raw.parse::<i64>() {
                Ok(value) => TokenKind::Integer(value),
                Err(_) => {
                    return Err(vec![Diagnostic::new(
                        DiagnosticKind::Lexical,
                        "número inteiro fora do limite",
                        line,
                        column,
                    )]);
                }
            }
        };
        self.push(kind, line, column);
        Ok(())
    }

    fn scan_string(&mut self, line: usize, column: usize) -> Result<(), Vec<Diagnostic>> {
        self.advance();
        let mut value = String::new();
        let mut closed = false;
        while let Some(character) = self.current() {
            match character {
                '"' if self.peek(1) == Some('"') => {
                    self.advance();
                    self.advance();
                    value.push('"');
                }
                '"' => {
                    self.advance();
                    closed = true;
                    break;
                }
                '\n' | '\r' => break,
                _ => {
                    self.advance();
                    value.push(character);
                }
            }
        }
        if !closed {
            return Err(vec![Diagnostic::new(
                DiagnosticKind::Lexical,
                "texto não terminado antes do fim da linha",
                line,
                column,
            )]);
        }
        self.push(TokenKind::String(value), line, column);
        Ok(())
    }

    fn scan_comparison(&mut self, line: usize, column: usize) {
        let character = self.advance().expect("current character exists");
        let operator = match (character, self.current()) {
            ('<', Some('-')) => {
                self.advance();
                "<-"
            }
            ('<', Some('=')) => {
                self.advance();
                "<="
            }
            ('<', Some('>')) => {
                self.advance();
                "<>"
            }
            ('>', Some('=')) => {
                self.advance();
                ">="
            }
            ('<', _) => "<",
            ('>', _) => ">",
            _ => unreachable!("only comparison characters enter this function"),
        };
        self.push(TokenKind::Operator(operator.to_owned()), line, column);
    }

    fn skip_comment(&mut self) {
        self.advance();
        self.advance();
        while self
            .current()
            .is_some_and(|character| character != '\n' && character != '\r')
        {
            self.advance();
        }
    }

    fn current(&self) -> Option<char> {
        self.characters.get(self.index).copied()
    }

    fn peek(&self, distance: usize) -> Option<char> {
        self.characters.get(self.index + distance).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.current()?;
        self.index += 1;
        if character == '\n' || character == '\r' && self.current() != Some('\n') {
            self.line += 1;
            self.column = 1;
        } else if character == '\r' && self.current() == Some('\n') {
            // CRLF is counted as one newline when the following LF is consumed.
        } else {
            self.column += 1;
        }
        Some(character)
    }

    fn push(&mut self, kind: TokenKind, line: usize, column: usize) {
        self.tokens.push(Token { kind, line, column });
    }
}

fn is_identifier_start(character: char) -> bool {
    character.is_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

pub(crate) fn scan(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    Scanner::new(source).scan()
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, scan};
    use crate::DiagnosticKind;

    #[test]
    fn lexes_case_insensitive_identifiers_without_folding_literals() {
        let tokens =
            scan("ALGORITMO \"MiXeD\"\nVAR idade: Inteiro").expect("the source should tokenize");

        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(name) if name == "algoritmo"));
        assert!(matches!(&tokens[1].kind, TokenKind::String(value) if value == "MiXeD"));
        assert!(matches!(&tokens[3].kind, TokenKind::Identifier(name) if name == "var"));
        assert!(matches!(&tokens[4].kind, TokenKind::Identifier(name) if name == "idade"));
        assert!(matches!(&tokens[6].kind, TokenKind::Identifier(name) if name == "inteiro"));
    }

    #[test]
    fn tracks_locations_after_line_and_trailing_comments() {
        let tokens =
            scan("var idade: inteiro // anotação\n  inicio").expect("the source should tokenize");
        let inicio = tokens
            .iter()
            .find(|token| matches!(&token.kind, TokenKind::Identifier(name) if name == "inicio"))
            .expect("the second-line keyword should be present");

        assert_eq!((inicio.line, inicio.column), (2, 3));
    }

    #[test]
    fn reports_unterminated_string_and_unknown_character_locations() {
        let string_error =
            scan("algoritmo \"x\n").expect_err("an unterminated string should be rejected");
        assert_eq!(string_error[0].kind, DiagnosticKind::Lexical);
        assert_eq!((string_error[0].line, string_error[0].column), (1, 11));

        let character_error =
            scan("inicio\n  @\n").expect_err("an unsupported character should be rejected");
        assert_eq!(character_error[0].kind, DiagnosticKind::Lexical);
        assert_eq!((character_error[0].line, character_error[0].column), (2, 3));
    }

    #[test]
    fn recognizes_visu_alg_operators_and_statement_punctuation() {
        let tokens = scan(r"<- <= <> >= > < + - * / \ ^ % = ( ) , : ;")
            .expect("all documented operators should tokenize");
        let operators = tokens
            .iter()
            .filter_map(|token| match &token.kind {
                TokenKind::Operator(operator) => Some(operator.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            operators,
            [
                "<-", "<=", "<>", ">=", ">", "<", "+", "-", "*", "/", "\\", "^", "%", "="
            ]
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Symbol('('))
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::Symbol(':'))
        );
    }
}
