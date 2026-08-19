use helix::ir::Lowerer;
use helix::lexer::Lexer;
use helix::parser::Parser;
use helix::runtime::SsaInterpreter;
use helix::ssa::{Operand, Optimizer, SsaInstruction, SsaLowerer};
use helix::types::TypeChecker;

fn compile(source: &str) -> helix::ssa::SsaProgram {
    let mut lexer = Lexer::new(source);

    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);

    let ast = parser.parse_program().unwrap();

    let mut checker = TypeChecker::new();

    let typed = checker.check(&ast).unwrap();

    let ir = Lowerer::lower(&typed);

    SsaLowerer::new().lower(&ir)
}

#[test]
fn lowers_expression_tree_into_ssa() {
    let ssa = compile("let x = 10 + 20; return x * 2;");

    assert_eq!(ssa.instruction_count(), 3);

    assert!(matches!(
        ssa.instructions()[0],
        SsaInstruction::Binary { .. }
    ));

    assert!(matches!(
        ssa.instructions()[1],
        SsaInstruction::Binary { .. }
    ));

    assert!(matches!(
        ssa.instructions()[2],
        SsaInstruction::Return { .. }
    ));
}

#[test]
fn constant_folding_reduces_program() {
    let ssa = compile("let x = 10 + 20; return x * 2;");

    let (optimized, stats) = Optimizer::optimize(&ssa);

    assert_eq!(stats.before, 3);

    assert_eq!(stats.after, 1);

    assert_eq!(stats.constants_folded, 2);

    assert!(stats.reduction_percent() > 60.0);

    assert_eq!(
        optimized.instructions(),
        &[SsaInstruction::Return {
            value: Operand::Constant(60),
        }]
    );
}

#[test]
fn dead_computation_is_eliminated() {
    let ssa = compile("let unused = 10 + 20; return 7;");

    let (optimized, stats) = Optimizer::optimize(&ssa);

    assert_eq!(optimized.instruction_count(), 1);

    assert!(stats.instructions_eliminated >= 1);

    assert_eq!(
        optimized.instructions(),
        &[SsaInstruction::Return {
            value: Operand::Constant(7),
        }]
    );
}

#[test]
fn optimized_ssa_preserves_result() {
    let ssa = compile("let x = 10 + 20; let y = x * 3; return y - 5;");

    let (optimized, _) = Optimizer::optimize(&ssa);

    let mut original_vm = SsaInterpreter::new();

    let mut optimized_vm = SsaInterpreter::new();

    let before = original_vm.execute(&ssa).unwrap();

    let after = optimized_vm.execute(&optimized).unwrap();

    assert_eq!(before, 85);

    assert_eq!(after, 85);
}

#[test]
fn division_by_zero_is_not_folded_away() {
    let ssa = compile("return 10 / 0;");

    let (optimized, _) = Optimizer::optimize(&ssa);

    let mut interpreter = SsaInterpreter::new();

    assert_eq!(
        interpreter.execute(&optimized,),
        Err(helix::runtime::RuntimeError::DivisionByZero)
    );
}

#[test]
fn optimizer_reports_instruction_reduction() {
    let ssa = compile("let a = 1 + 2; let b = a * 4; let dead = 100 + 200; return b + 5;");

    let (optimized, stats) = Optimizer::optimize(&ssa);

    assert!(optimized.instruction_count() < ssa.instruction_count());

    assert!(stats.constants_folded >= 3);

    assert!(stats.reduction_percent() > 50.0);
}
