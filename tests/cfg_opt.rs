use helix::cfg::{CfgBuilder, Terminator};
use helix::cfg_opt::CfgOptimizer;
use helix::lexer::Lexer;
use helix::parser::Parser;
use helix::types::TypeChecker;

fn build(source: &str) -> helix::cfg::ControlFlowGraph {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap();

    let mut checker = TypeChecker::new();
    let typed = checker.check(&program).unwrap();

    CfgBuilder::new().build(&typed)
}

#[test]
fn constant_condition_folds_branch() {
    let cfg = build("if 10 > 5 { return 1; } else { return 0; }");

    let (optimized, stats) = CfgOptimizer::optimize(&cfg);

    assert_eq!(stats.branches_folded, 1);
    assert_eq!(optimized.blocks().len(), 2);

    assert!(matches!(
        optimized.block(0).unwrap().terminator(),
        Some(Terminator::Jump(1))
    ));
}

#[test]
fn false_condition_removes_then_branch() {
    let cfg = build("if 10 < 5 { return 1; } else { return 0; }");

    let (optimized, stats) = CfgOptimizer::optimize(&cfg);

    assert_eq!(stats.branches_folded, 1);
    assert_eq!(optimized.blocks().len(), 2);

    assert!(matches!(
        optimized.block(0).unwrap().terminator(),
        Some(Terminator::Jump(2))
    ));
}

#[test]
fn nested_constant_branches_are_pruned() {
    let cfg = build(
        "
        if 10 > 5 {
            if 3 < 4 {
                return 1;
            } else {
                return 2;
            }
        } else {
            return 0;
        }
        ",
    );

    let before = cfg.blocks().len();

    let (optimized, stats) = CfgOptimizer::optimize(&cfg);

    assert!(optimized.blocks().len() < before);
    assert!(stats.branches_folded >= 2);
    assert!(stats.block_reduction_percent() > 0.0);
}

#[test]
fn optimizer_reports_instruction_statistics() {
    let cfg = build(
        "
        let x = 10 + 20;
        if x > 5 {
            return x;
        } else {
            return 0;
        }
        ",
    );

    let (_, stats) = CfgOptimizer::optimize(&cfg);

    assert!(stats.instructions_after <= stats.instructions_before);

    assert!(stats.constants_propagated > 0);
}
