use std::collections::BTreeSet;

use helix::cfg::{CfgBuilder, ControlFlowGraph};
use helix::dominance::DominanceInfo;
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

#[test]
fn entry_dominates_all_reachable_blocks() {
    let cfg = build(
        "
        if 10 > 5 {
            return 1;
        } else {
            return 0;
        }
        ",
    );

    let dominance = DominanceInfo::compute(&cfg);

    for block in cfg.reachable_blocks() {
        assert!(dominance.dominates(cfg.entry(), block));
    }
}

#[test]
fn branch_children_are_immediately_dominated_by_entry() {
    let cfg = build(
        "
        if 10 > 5 {
            return 1;
        } else {
            return 0;
        }
        ",
    );

    let dominance = DominanceInfo::compute(&cfg);

    assert_eq!(dominance.immediate_dominator(1), Some(0));
    assert_eq!(dominance.immediate_dominator(2), Some(0));
}

#[test]
fn dominator_tree_reports_branch_children() {
    let cfg = build(
        "
        if 10 > 5 {
            return 1;
        } else {
            return 0;
        }
        ",
    );

    let dominance = DominanceInfo::compute(&cfg);

    assert_eq!(dominance.dominator_tree_children(0), vec![1, 2],);
}

#[test]
fn nested_branch_has_expected_dominance() {
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

    let dominance = DominanceInfo::compute(&cfg);

    assert!(dominance.dominates(0, 1));

    let nested_successors = cfg.successors(1);

    assert_eq!(nested_successors.len(), 2);

    for child in nested_successors {
        assert!(dominance.dominates(1, child));
        assert_eq!(dominance.immediate_dominator(child), Some(1),);
    }
}

#[test]
fn dominator_sets_include_self() {
    let cfg = build(
        "
        if 10 > 5 {
            return 1;
        } else {
            return 0;
        }
        ",
    );

    let dominance = DominanceInfo::compute(&cfg);

    assert_eq!(dominance.dominators(0), Some(&BTreeSet::from([0])),);

    assert_eq!(dominance.dominators(1), Some(&BTreeSet::from([0, 1])),);
}
