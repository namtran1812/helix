use crate::ast::{BinaryOp, Expr, Program, Statement};
use crate::lexer::Token;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error("unexpected token: {0:?}")]
    UnexpectedToken(Token),

    #[error("unexpected end of input")]
    UnexpectedEof,
}

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();

        while !matches!(self.peek(), Some(Token::Eof)) {
            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek() {
            Some(Token::Let) => self.parse_let(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::If) => self.parse_if(),
            Some(token) => Err(ParseError::UnexpectedToken(token.clone())),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn parse_let(&mut self) -> Result<Statement, ParseError> {
        self.advance();

        let name = match self.advance() {
            Some(Token::Identifier(name)) => name,
            Some(token) => return Err(ParseError::UnexpectedToken(token)),
            None => return Err(ParseError::UnexpectedEof),
        };

        self.expect(Token::Equal)?;

        let value = self.parse_expression(0)?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Let { name, value })
    }

    fn parse_return(&mut self) -> Result<Statement, ParseError> {
        self.advance();

        let value = self.parse_expression(0)?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Return(value))
    }

    fn parse_if(&mut self) -> Result<Statement, ParseError> {
        self.advance();

        let condition = self.parse_expression(0)?;

        self.expect(Token::LeftBrace)?;
        let then_branch = self.parse_block()?;

        self.expect(Token::Else)?;
        self.expect(Token::LeftBrace)?;
        let else_branch = self.parse_block()?;

        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();

        while !matches!(self.peek(), Some(Token::RightBrace)) {
            if matches!(self.peek(), Some(Token::Eof) | None) {
                return Err(ParseError::UnexpectedEof);
            }

            statements.push(self.parse_statement()?);
        }

        self.expect(Token::RightBrace)?;

        Ok(statements)
    }

    fn parse_expression(&mut self, min_precedence: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;

        while let Some((op, precedence)) = self.current_binary_op() {
            if precedence < min_precedence {
                break;
            }

            self.advance();

            let right = self.parse_expression(precedence + 1)?;

            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.advance() {
            Some(Token::Integer(value)) => Ok(Expr::Integer(value)),
            Some(Token::Identifier(name)) => Ok(Expr::Identifier(name)),
            Some(Token::LeftParen) => {
                let expr = self.parse_expression(0)?;

                self.expect(Token::RightParen)?;

                Ok(expr)
            }
            Some(token) => Err(ParseError::UnexpectedToken(token)),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8)> {
        match self.peek()? {
            Token::EqualEqual => Some((BinaryOp::Equal, 1)),
            Token::BangEqual => Some((BinaryOp::NotEqual, 1)),
            Token::Less => Some((BinaryOp::Less, 1)),
            Token::LessEqual => Some((BinaryOp::LessEqual, 1)),
            Token::Greater => Some((BinaryOp::Greater, 1)),
            Token::GreaterEqual => Some((BinaryOp::GreaterEqual, 1)),

            Token::Plus => Some((BinaryOp::Add, 2)),
            Token::Minus => Some((BinaryOp::Subtract, 2)),

            Token::Star => Some((BinaryOp::Multiply, 3)),
            Token::Slash => Some((BinaryOp::Divide, 3)),

            _ => None,
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        match self.advance() {
            Some(token) if token == expected => Ok(()),
            Some(token) => Err(ParseError::UnexpectedToken(token)),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();

        if token.is_some() {
            self.position += 1;
        }

        token
    }
}
