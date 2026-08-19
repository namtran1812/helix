use helix::cfg::{CfgBuilder, Instruction, Operand, Terminator};
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

#[test]
fn divergent_branch_assignments_create_phi() {
    let cfg = build(
        "
        let x = 0;

        if 10 > 5 {
            x = 10;
        } else {
            x = 20;
        }

        return x;
        ",
    );

    let merge = cfg
        .blocks()
        .iter()
        .find(|block| {
            block
                .instructions()
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Phi { .. }))
        })
        .expect("expected phi merge block");

    let (phi_result, incomings) = merge
        .instructions()
        .iter()
        .find_map(|instruction| match instruction {
            Instruction::Phi {
                result, incomings, ..
            } => Some((*result, incomings)),
            _ => None,
        })
        .expect("expected phi");

    assert_eq!(incomings.len(), 2);

    assert!(matches!(
        merge.terminator(),
        Some(
            Terminator::Return(
                Operand::Value(value)
            )
        ) if *value == phi_result
    ));
}

#[test]
fn identical_branch_values_do_not_create_phi() {
    let cfg = build(
        "
        let x = 7;

        if 10 > 5 {
            x = 7;
        } else {
            x = 7;
        }

        return x;
        ",
    );

    let count = cfg
        .blocks()
        .iter()
        .flat_map(|block| block.instructions())
        .filter(|instruction| matches!(instruction, Instruction::Phi { .. }))
        .count();

    assert_eq!(count, 0);
}

#[test]
fn phi_inputs_are_cfg_predecessors() {
    let cfg = build(
        "
        let x = 0;

        if 10 > 5 {
            x = 1;
        } else {
            x = 2;
        }

        return x;
        ",
    );

    let merge = cfg
        .blocks()
        .iter()
        .find(|block| {
            block
                .instructions()
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Phi { .. }))
        })
        .expect("expected phi merge");

    let incomings = merge
        .instructions()
        .iter()
        .find_map(|instruction| match instruction {
            Instruction::Phi { incomings, .. } => Some(incomings),
            _ => None,
        })
        .unwrap();

    let predecessors = cfg.predecessors(merge.id());

    assert_eq!(incomings.len(), predecessors.len());

    for (predecessor, _) in incomings {
        assert!(predecessors.contains(predecessor));
    }
}

#[test]
fn one_live_branch_does_not_require_phi() {
    let cfg = build(
        "
        let x = 0;

        if 10 > 5 {
            return 1;
        } else {
            x = 20;
        }

        return x;
        ",
    );

    let count = cfg
        .blocks()
        .iter()
        .flat_map(|block| block.instructions())
        .filter(|instruction| matches!(instruction, Instruction::Phi { .. }))
        .count();

    assert_eq!(count, 0);
}
