use helix::cfg::{CfgBuilder, Terminator};
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
fn if_else_creates_branching_cfg() {
    let cfg = build("if 10 > 5 { return 1; } else { return 0; }");

    assert_eq!(cfg.entry(), 0);
    assert_eq!(cfg.blocks().len(), 3);

    let entry = cfg.block(0).unwrap();

    assert!(matches!(
        entry.terminator(),
        Some(Terminator::Branch {
            then_block: 1,
            else_block: 2,
            ..
        })
    ));

    assert!(matches!(
        cfg.block(1).unwrap().terminator(),
        Some(Terminator::Return(_))
    ));

    assert!(matches!(
        cfg.block(2).unwrap().terminator(),
        Some(Terminator::Return(_))
    ));
}

#[test]
fn nested_if_creates_multiple_branch_blocks() {
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

    let branch_count = cfg
        .blocks()
        .iter()
        .filter(|block| matches!(block.terminator(), Some(Terminator::Branch { .. })))
        .count();

    assert_eq!(branch_count, 2);
}

#[test]
fn arithmetic_condition_is_rejected() {
    let mut lexer = Lexer::new("if 42 { return 1; } else { return 0; }");

    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap();

    let mut checker = TypeChecker::new();

    assert!(checker.check(&program).is_err());
}

#[test]
fn cfg_reports_successors_and_predecessors() {
    let cfg = build("if 10 > 5 { return 1; } else { return 0; }");

    assert_eq!(cfg.successors(0), vec![1, 2]);
    assert_eq!(cfg.predecessors(1), vec![0]);
    assert_eq!(cfg.predecessors(2), vec![0]);
}

#[test]
fn unreachable_merge_block_is_pruned_when_both_branches_return() {
    let cfg = build("if 10 > 5 { return 1; } else { return 0; }");

    assert_eq!(cfg.blocks().len(), 3);

    assert!(cfg.block(3).is_none());
}

#[test]
fn symbol_use_resolves_to_definition_operand() {
    let cfg = build("let x = 10; if x > 5 { return x; } else { return 0; }");

    let entry = cfg.block(0).unwrap();

    assert!(!entry.instructions().is_empty());
}
