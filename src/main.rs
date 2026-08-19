use helix::ir::Lowerer;
use helix::lexer::Lexer;
use helix::parser::Parser;
use helix::runtime::Interpreter;
use helix::types::TypeChecker;

fn main() {
    let source = "let x = 10; let y = x + 20; return y * 2;";

    let mut lexer = Lexer::new(source);

    let tokens = lexer.tokenize().expect("lexing failed");

    let mut parser = Parser::new(tokens);

    let program = parser.parse_program().expect("parsing failed");

    let mut checker = TypeChecker::new();

    let typed = checker.check(&program).expect("semantic analysis failed");

    let ir = Lowerer::lower(&typed);

    let mut interpreter = Interpreter::new();

    let result = interpreter.execute(&ir).expect("execution failed");

    println!("{result}");
}
