use helix::ast::{BinaryOp, Expr, Statement};
use helix::lexer::{Lexer, Token};
use helix::parser::Parser;

#[test]
fn lexer_tokenizes_program() {
    let mut lexer = Lexer::new("let x = 10 + 20; return x;");

    let tokens = lexer.tokenize().unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Let,
            Token::Identifier("x".into()),
            Token::Equal,
            Token::Integer(10),
            Token::Plus,
            Token::Integer(20),
            Token::Semicolon,
            Token::Return,
            Token::Identifier("x".into()),
            Token::Semicolon,
            Token::Eof,
        ]
    );
}

#[test]
fn parser_respects_operator_precedence() {
    let mut lexer = Lexer::new("return 1 + 2 * 3;");

    let mut parser = Parser::new(lexer.tokenize().unwrap());

    let program = parser.parse_program().unwrap();

    assert_eq!(
        program.statements[0],
        Statement::Return(Expr::Binary {
            left: Box::new(Expr::Integer(1),),
            op: BinaryOp::Add,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Integer(2),),
                op: BinaryOp::Multiply,
                right: Box::new(Expr::Integer(3),),
            },),
        },)
    );
}

#[test]
fn parser_handles_parentheses() {
    let mut lexer = Lexer::new("return (1 + 2) * 3;");

    let mut parser = Parser::new(lexer.tokenize().unwrap());

    let program = parser.parse_program().unwrap();

    assert_eq!(program.statements.len(), 1);
}

#[test]
fn parser_handles_let_and_return() {
    let mut lexer = Lexer::new("let x = 7; return x + 1;");

    let mut parser = Parser::new(lexer.tokenize().unwrap());

    let program = parser.parse_program().unwrap();

    assert_eq!(program.statements.len(), 2);
}
