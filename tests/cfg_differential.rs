use helix::cfg::{CfgBuilder, ControlFlowGraph};
use helix::cfg_opt::CfgOptimizer;
use helix::cfg_runtime::CfgInterpreter;
use helix::lexer::Lexer;
use helix::parser::Parser;
use helix::types::TypeChecker;

fn build(source: &str) -> ControlFlowGraph {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap();

    let mut checker = TypeChecker::new();
    let typed = checker.check(&program).unwrap();

    CfgBuilder::new().build(&typed)
}

fn assert_equivalent(source: &str) {
    let cfg = build(source);

    let before = CfgInterpreter::execute(&cfg).unwrap();

    let (optimized, _) = CfgOptimizer::optimize(&cfg);

    let after = CfgInterpreter::execute(&optimized).unwrap();

    assert_eq!(
        before, after,
        "optimizer changed program semantics:\n{source}",
    );
}

#[test]
fn preserves_arithmetic_result() {
    assert_equivalent(
        "
        let x = 10 + 20;
        return x * 2;
        ",
    );
}

#[test]
fn preserves_true_branch() {
    assert_equivalent(
        "
        let x = 10;

        if x > 5 {
            return x + 1;
        } else {
            return 0;
        }
        ",
    );
}

#[test]
fn preserves_false_branch() {
    assert_equivalent(
        "
        let x = 2;

        if x > 5 {
            return 100;
        } else {
            return x + 3;
        }
        ",
    );
}

#[test]
fn preserves_phi_result() {
    assert_equivalent(
        "
        let x = 0;

        if 7 > 3 {
            x = 11;
        } else {
            x = 22;
        }

        return x;
        ",
    );
}

#[test]
fn preserves_nested_control_flow() {
    assert_equivalent(
        "
        let x = 3;

        if x > 0 {
            if x < 5 {
                return x * 10;
            } else {
                return 99;
            }
        } else {
            return 0;
        }
        ",
    );
}

#[test]
fn preserves_assignment_after_merge() {
    assert_equivalent(
        "
        let x = 1;

        if 4 == 4 {
            x = 10;
        } else {
            x = 20;
        }

        let y = x + 7;
        return y;
        ",
    );
}
