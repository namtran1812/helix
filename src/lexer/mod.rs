#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Let,
    Return,
    Identifier(String),
    Integer(i64),
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    Semicolon,
    LeftParen,
    RightParen,
    Eof,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LexError {
    #[error("unexpected character: {0}")]
    UnexpectedCharacter(char),

    #[error("invalid integer literal")]
    InvalidInteger,
}

pub struct Lexer<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let done = token == Token::Eof;

            tokens.push(token);

            if done {
                break;
            }
        }

        Ok(tokens)
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace();

        let Some(byte) = self.peek() else {
            return Ok(Token::Eof);
        };

        match byte {
            b'+' => {
                self.position += 1;
                Ok(Token::Plus)
            }
            b'-' => {
                self.position += 1;
                Ok(Token::Minus)
            }
            b'*' => {
                self.position += 1;
                Ok(Token::Star)
            }
            b'/' => {
                self.position += 1;
                Ok(Token::Slash)
            }
            b'=' => {
                self.position += 1;
                Ok(Token::Equal)
            }
            b';' => {
                self.position += 1;
                Ok(Token::Semicolon)
            }
            b'(' => {
                self.position += 1;
                Ok(Token::LeftParen)
            }
            b')' => {
                self.position += 1;
                Ok(Token::RightParen)
            }
            b'0'..=b'9' => self.lex_integer(),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_identifier(),
            _ => Err(LexError::UnexpectedCharacter(byte as char)),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }

    fn lex_integer(&mut self) -> Result<Token, LexError> {
        let start = self.position;

        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.position += 1;
        }

        let text =
            std::str::from_utf8(&self.input[start..self.position]).expect("integer token is ASCII");

        let value = text.parse::<i64>().map_err(|_| LexError::InvalidInteger)?;

        Ok(Token::Integer(value))
    }

    fn lex_identifier(&mut self) -> Result<Token, LexError> {
        let start = self.position;

        while matches!(
            self.peek(),
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
        ) {
            self.position += 1;
        }

        let text = std::str::from_utf8(&self.input[start..self.position])
            .expect("identifier token is ASCII");

        Ok(match text {
            "let" => Token::Let,
            "return" => Token::Return,
            _ => Token::Identifier(text.to_string()),
        })
    }
}
