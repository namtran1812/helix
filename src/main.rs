use helix::lexer::Lexer;
use helix::parser::Parser;

fn main() {
    let source = "let x = 10; return x + 20 * 2;";

    let mut lexer = Lexer::new(source);

    let tokens = lexer.tokenize().expect("lexing failed");

    let mut parser = Parser::new(tokens);

    let program = parser.parse_program().expect("parsing failed");

    println!("{program:#?}");
}
