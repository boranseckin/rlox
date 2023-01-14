use substring::Substring;

use crate::token::{Token, Type, Literal};
use crate::report;

pub struct Scanner {
    source: String,
    pub tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new(source: String) -> Scanner {
        Scanner { source, tokens: vec!(), start: 0, current: 0, line: 1 }
    }

    pub fn scan_tokens(&mut self) {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.tokens.push(
            Token::new(
                Type::EOF,
                String::from(""),
                None,
                self.line
            )
        );
    }

    fn advance(&mut self) -> char {
        let temp = self.current;
        self.current += 1;

        match self.source.chars().nth(temp) {
            Some(char) => char,
            None => panic!("tried to advance past end of the file."),
        }
    }

    fn peek(&self) -> char {
        match self.source.chars().nth(self.current) {
            Some(char) => char,
            None => panic!("tried to peek past end of the file."),
        }
    }

    fn peek_next(&self) -> char {
        match self.source.chars().nth(self.current + 1) {
            Some(char) => char,
            None => panic!("tried to peek next past end of the file."),
        }
    }

    fn match_next(&mut self, expected: char) -> bool {
        match self.source.chars().nth(self.current) {
            Some(char) if char == expected => {
                self.current += 1;
                true
            },
            Some(_) => false,
            None => false,
        }
    }

    fn add_token(&mut self, r#type: Type, literal: Option<Literal>) {
        let text = self.source.substring(self.start, self.current);
        self.tokens.push(
            Token::new(
                r#type,
                String::from(text),
                literal,
                self.line
            )
        );
    }

    fn is_at_end(&self) -> bool {
       self.current >= self.source.len().try_into().unwrap()
    }

    fn string(&mut self) {
        let start = (self.line, self.start);

        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\n' {
                self.line += 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            report(start.0, Some(start.1), "Unterminated string.");
            return;
        }

        self.advance();  // Move to the closing double quotes.

        // Literal does not include the double quotes unlike the lexeme.
        let value = self.source.substring(self.start + 1, self.current - 1);
        self.add_token(Type::STRING, Some(Literal::String(String::from(value))));
    }

    fn number(&mut self) {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        if self.peek() == '.' {
            if self.peek_next().is_ascii_digit() {
                self.advance();  // Consume the dot.

                while self.peek().is_ascii_digit() {
                    self.advance();
                }
            } else {
                report(self.line, Some(self.start), "Unterminated number.");
            }
        }

        let value: f32 = self.source.substring(self.start, self.current).parse().unwrap();
        self.add_token(Type::NUMBER, Some(Literal::Float(value)));
    }

    fn identifier(&mut self) {
        while self.peek().is_alphanumeric() {
            self.advance();
        }

        let value = self.source.substring(self.start, self.current);
        let token_type = match value {
            "and"    => Type::AND,
            "class"  => Type::CLASS,
            "else"   => Type::ELSE,
            "false"  => Type::FALSE,
            "for"    => Type::FOR,
            "fun"    => Type::FUN,
            "if"     => Type::IF,
            "nil"    => Type::NIL,
            "or"     => Type::OR,
            "print"  => Type::PRINT,
            "return" => Type::RETURN,
            "super"  => Type::SUPER,
            "this"   => Type::THIS,
            "true"   => Type::TRUE,
            "var"    => Type::VAR,
            "while"  => Type::WHILE,
            _        => Type::IDENTIFIER,
        };

        self.add_token(token_type, None);
    }

    fn scan_token(&mut self) {
        let c = self.advance();
        match c {
            // One character tokens
            '(' => self.add_token(Type::LEFT_PAREN, None),
            ')' => self.add_token(Type::RIGHT_PAREN, None),
            '{' => self.add_token(Type::LEFT_BRACE, None),
            '}' => self.add_token(Type::RIGHT_BRACE, None),
            ',' => self.add_token(Type::COMMA, None),
            '.' => self.add_token(Type::DOT, None),
            '-' => self.add_token(Type::MINUS, None),
            '+' => self.add_token(Type::PLUS, None),
            ';' => self.add_token(Type::SEMICOLON, None),
            '*' => self.add_token(Type::STAR, None),

            // Two character tokens
            '!' => {
                if self.match_next('=') {
                    self.add_token(Type::BANG_EQUAL, None);
                } else {
                    self.add_token(Type::BANG, None)
                };
            },
            '=' => {
                if self.match_next('=') {
                    self.add_token(Type::EQUAL_EQUAL, None);
                } else {
                    self.add_token(Type::EQUAL, None)
                };
            },
            '<' => {
                if self.match_next('=') {
                    self.add_token(Type::LESS_EQUAL, None);
                } else {
                    self.add_token(Type::LESS, None)
                };
            },
            '>' => {
                if self.match_next('=') {
                    self.add_token(Type::GREATER_EQUAL, None);
                } else {
                    self.add_token(Type::GREATER, None)
                };
            },
            '/' => {
                if self.match_next('/') {
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(Type::SLASH, None);
                }
            },

            // Ignore whitespace
            ' ' | '\r' | '\t' => {},

            // Update line counter
            '\n' => self.line += 1,

            // String
            '"' => self.string(),

            _ => {
                // Numbers
                if c.is_ascii_digit() {
                    self.number();
                // Identifiers
                } else if c.is_alphabetic() || c == '_' {
                    self.identifier();
                // Unknown
                } else {
                    report(
                        self.line,
                        Some(self.current),
                        format!("Unexpected character {}.", c).as_str()
                    ); 
                }
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn advance() {
        let mut scanner = Scanner::new(String::from("abcdefg"));

        assert_eq!(scanner.current, 0);

        let result = scanner.advance();
        assert_eq!(result, 'a');
        assert_eq!(scanner.current, 1);

        let result = scanner.advance();
        assert_eq!(result, 'b');
        assert_eq!(scanner.current, 2);
    }

    #[test]
    #[should_panic(expected = "tried to advance past end of the file.")]
    fn advance_eof() {
        let mut scanner = Scanner::new(String::from("a"));

        scanner.advance();
        scanner.advance();
    }

    #[test]
    fn match_next_truthy() {
        let mut scanner = Scanner::new(String::from("!="));
        scanner.advance();  // Move to the first char
        let result = scanner.match_next('=');
        assert!(result); 
        assert_eq!(scanner.current, 2);
    }

    #[test]
    fn match_next_faulty() {
        let mut scanner = Scanner::new(String::from("!a"));
        scanner.advance();  // Move to the first char

        let result = scanner.match_next('=');
        assert!(!result);
        assert_eq!(scanner.current, 1);  // Should not move the current
    }

    #[test]
    fn match_next_eof() {
        let mut scanner = Scanner::new(String::from("a"));
        scanner.advance();  // Move to the first char

        let result = scanner.match_next('b');
        assert!(!result);
        assert_eq!(scanner.current, 1);  // Should not move the current
    }

    #[test]
    fn peek() {
        let mut scanner = Scanner::new(String::from("abc"));
        scanner.advance();

        let result = scanner.peek();
        assert_eq!(result, 'b');
        assert_eq!(scanner.current, 1);  // Should not move the current
    }

    #[test]
    #[should_panic(expected = "tried to peek past end of the file.")]
    fn peek_eof() {
        let mut scanner = Scanner::new(String::from("a"));
        scanner.advance();
        scanner.peek();
     }

    #[test]
    fn peek_next() {
        let mut scanner = Scanner::new(String::from("abc"));
        scanner.advance();

        let result = scanner.peek_next();
        assert_eq!(result, 'c');
        assert_eq!(scanner.current, 1);  // Should not move the current
    }

    #[test]
    #[should_panic(expected = "tried to peek next past end of the file.")]
    fn peek_next_eof() {
        let scanner = Scanner::new(String::from("a"));
        scanner.peek_next();
     }

    #[test]
    fn is_at_end() {
        let mut scanner = Scanner::new(String::from("ab"));

        scanner.advance();
        assert!(!scanner.is_at_end());

        scanner.advance();
        assert!(scanner.is_at_end());
    }
}
