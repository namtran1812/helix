use helix::ir::Lowerer;
use helix::lexer::Lexer;
use helix::parser::Parser;
use helix::runtime::{Interpreter, RuntimeError};
use helix::types::{SemanticError, Type, TypeChecker};

fn parse(source: &str) -> helix::ast::Program {
    let mut lexer = Lexer::new(source);

    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);

    parser.parse_program().unwrap()
}

#[test]
fn resolves_symbols_and_infers_i64() {
    let program = parse("let x = 10; let y = x + 2; return y;");

    let mut checker = TypeChecker::new();

    let typed = checker.check(&program).unwrap();

    assert_eq!(typed.symbols().len(), 2);

    assert_eq!(typed.symbols()[0].name(), "x");

    assert_eq!(typed.symbols()[0].ty(), Type::I64);

    assert_eq!(typed.symbols()[1].name(), "y");
}

#[test]
fn undefined_identifier_is_rejected() {
    let program = parse("return missing + 1;");

    let mut checker = TypeChecker::new();

    assert_eq!(
        checker.check(&program),
        Err(SemanticError::UndefinedIdentifier("missing".into(),))
    );
}

#[test]
fn duplicate_binding_is_rejected() {
    let program = parse("let x = 1; let x = 2; return x;");

    let mut checker = TypeChecker::new();

    assert_eq!(
        checker.check(&program),
        Err(SemanticError::DuplicateBinding("x".into(),))
    );
}

#[test]
fn use_before_definition_is_rejected() {
    let program = parse("let y = x + 1; let x = 2; return y;");

    let mut checker = TypeChecker::new();

    assert_eq!(
        checker.check(&program),
        Err(SemanticError::UndefinedIdentifier("x".into(),))
    );
}

#[test]
fn missing_return_is_rejected() {
    let program = parse("let x = 10;");

    let mut checker = TypeChecker::new();

    assert_eq!(checker.check(&program), Err(SemanticError::MissingReturn));
}

#[test]
fn interpreter_executes_typed_program() {
    let program = parse("let x = 10; let y = x + 20; return y * 2;");

    let mut checker = TypeChecker::new();

    let typed = checker.check(&program).unwrap();

    let ir = Lowerer::lower(&typed);

    let mut interpreter = Interpreter::new();

    assert_eq!(interpreter.execute(&ir).unwrap(), 60);
}

#[test]
fn division_by_zero_is_runtime_error() {
    let program = parse("return 10 / 0;");

    let mut checker = TypeChecker::new();

    let typed = checker.check(&program).unwrap();

    let ir = Lowerer::lower(&typed);

    let mut interpreter = Interpreter::new();

    assert_eq!(interpreter.execute(&ir), Err(RuntimeError::DivisionByZero));
}
