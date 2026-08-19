use std::collections::{HashMap, HashSet};

use crate::ast::BinaryOp;
use crate::ir::{IrInstruction, IrProgram, IrValue};
use crate::types::SymbolId;

pub type ValueId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Constant(i64),
    Value(ValueId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsaInstruction {
    Binary {
        result: ValueId,
        op: BinaryOp,
        left: Operand,
        right: Operand,
    },
    Copy {
        result: ValueId,
        source: Operand,
    },
    Return {
        value: Operand,
    },
}

impl SsaInstruction {
    pub fn result(&self) -> Option<ValueId> {
        match self {
            Self::Binary { result, .. } | Self::Copy { result, .. } => Some(*result),
            Self::Return { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsaProgram {
    instructions: Vec<SsaInstruction>,
}

impl SsaProgram {
    pub fn new(instructions: Vec<SsaInstruction>) -> Self {
        Self { instructions }
    }

    pub fn instructions(&self) -> &[SsaInstruction] {
        &self.instructions
    }

    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }
}

pub struct SsaLowerer {
    next_value: ValueId,
    symbols: HashMap<SymbolId, Operand>,
    instructions: Vec<SsaInstruction>,
}

impl Default for SsaLowerer {
    fn default() -> Self {
        Self::new()
    }
}

impl SsaLowerer {
    pub fn new() -> Self {
        Self {
            next_value: 0,
            symbols: HashMap::new(),
            instructions: Vec::new(),
        }
    }

    pub fn lower(mut self, program: &IrProgram) -> SsaProgram {
        for instruction in program.instructions() {
            match instruction {
                IrInstruction::Bind { symbol_id, value } => {
                    let operand = self.lower_value(value);
                    self.symbols.insert(*symbol_id, operand);
                }

                IrInstruction::Return { value } => {
                    let operand = self.lower_value(value);

                    self.instructions
                        .push(SsaInstruction::Return { value: operand });
                }
            }
        }

        SsaProgram::new(self.instructions)
    }

    fn lower_value(&mut self, value: &IrValue) -> Operand {
        match value {
            IrValue::Constant { value, .. } => Operand::Constant(*value),

            IrValue::Symbol { symbol_id, .. } => *self
                .symbols
                .get(symbol_id)
                .expect("typed IR referenced undefined symbol"),

            IrValue::Binary {
                left, op, right, ..
            } => {
                let left = self.lower_value(left);
                let right = self.lower_value(right);

                let result = self.allocate_value();

                self.instructions.push(SsaInstruction::Binary {
                    result,
                    op: *op,
                    left,
                    right,
                });

                Operand::Value(result)
            }
        }
    }

    fn allocate_value(&mut self) -> ValueId {
        let id = self.next_value;
        self.next_value += 1;
        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptimizationStats {
    pub before: usize,
    pub after: usize,
    pub constants_folded: usize,
    pub copies_propagated: usize,
    pub instructions_eliminated: usize,
}

impl OptimizationStats {
    pub fn reduction_percent(&self) -> f64 {
        if self.before == 0 {
            return 0.0;
        }

        100.0 * (self.before - self.after) as f64 / self.before as f64
    }
}

pub struct Optimizer;

impl Optimizer {
    pub fn optimize(program: &SsaProgram) -> (SsaProgram, OptimizationStats) {
        let before = program.instruction_count();

        let (folded, constants_folded) = constant_fold(program);

        let (propagated, copies_propagated) = copy_propagate(&folded);

        let (optimized, instructions_eliminated) = dead_code_eliminate(&propagated);

        let after = optimized.instruction_count();

        (
            optimized,
            OptimizationStats {
                before,
                after,
                constants_folded,
                copies_propagated,
                instructions_eliminated,
            },
        )
    }
}

fn constant_fold(program: &SsaProgram) -> (SsaProgram, usize) {
    let mut constants = HashMap::<ValueId, i64>::new();

    let mut folded = 0;

    let mut instructions = Vec::with_capacity(program.instructions.len());

    for instruction in &program.instructions {
        match instruction {
            SsaInstruction::Binary {
                result,
                op,
                left,
                right,
            } => {
                let left = resolve_constant(*left, &constants);

                let right = resolve_constant(*right, &constants);

                if let (Operand::Constant(lhs), Operand::Constant(rhs)) = (left, right) {
                    if let Some(value) = fold_binary(*op, lhs, rhs) {
                        constants.insert(*result, value);

                        instructions.push(SsaInstruction::Copy {
                            result: *result,
                            source: Operand::Constant(value),
                        });

                        folded += 1;
                        continue;
                    }
                }

                instructions.push(SsaInstruction::Binary {
                    result: *result,
                    op: *op,
                    left,
                    right,
                });
            }

            SsaInstruction::Copy { result, source } => {
                let source = resolve_constant(*source, &constants);

                if let Operand::Constant(value) = source {
                    constants.insert(*result, value);
                }

                instructions.push(SsaInstruction::Copy {
                    result: *result,
                    source,
                });
            }

            SsaInstruction::Return { value } => {
                instructions.push(SsaInstruction::Return {
                    value: resolve_constant(*value, &constants),
                });
            }
        }
    }

    (SsaProgram::new(instructions), folded)
}

fn resolve_constant(operand: Operand, constants: &HashMap<ValueId, i64>) -> Operand {
    match operand {
        Operand::Value(id) => constants
            .get(&id)
            .copied()
            .map(Operand::Constant)
            .unwrap_or(operand),

        Operand::Constant(_) => operand,
    }
}

fn copy_propagate(program: &SsaProgram) -> (SsaProgram, usize) {
    let mut copies = HashMap::<ValueId, Operand>::new();

    let mut propagated = 0;

    let mut instructions = Vec::with_capacity(program.instructions.len());

    for instruction in &program.instructions {
        match instruction {
            SsaInstruction::Copy { result, source } => {
                let source = resolve_copy(*source, &copies);

                copies.insert(*result, source);

                instructions.push(SsaInstruction::Copy {
                    result: *result,
                    source,
                });
            }

            SsaInstruction::Binary {
                result,
                op,
                left,
                right,
            } => {
                let resolved_left = resolve_copy(*left, &copies);

                let resolved_right = resolve_copy(*right, &copies);

                if resolved_left != *left {
                    propagated += 1;
                }

                if resolved_right != *right {
                    propagated += 1;
                }

                instructions.push(SsaInstruction::Binary {
                    result: *result,
                    op: *op,
                    left: resolved_left,
                    right: resolved_right,
                });
            }

            SsaInstruction::Return { value } => {
                let resolved = resolve_copy(*value, &copies);

                if resolved != *value {
                    propagated += 1;
                }

                instructions.push(SsaInstruction::Return { value: resolved });
            }
        }
    }

    (SsaProgram::new(instructions), propagated)
}

fn resolve_copy(mut operand: Operand, copies: &HashMap<ValueId, Operand>) -> Operand {
    let mut visited = HashSet::new();

    while let Operand::Value(id) = operand {
        if !visited.insert(id) {
            break;
        }

        let Some(next) = copies.get(&id) else {
            break;
        };

        operand = *next;
    }

    operand
}

fn dead_code_eliminate(program: &SsaProgram) -> (SsaProgram, usize) {
    let mut live = HashSet::<ValueId>::new();

    let mut kept = Vec::with_capacity(program.instructions.len());

    let mut eliminated = 0;

    for instruction in program.instructions.iter().rev() {
        match instruction {
            SsaInstruction::Return { value } => {
                mark_operand(*value, &mut live);

                kept.push(instruction.clone());
            }

            SsaInstruction::Binary {
                result,
                left,
                right,
                ..
            } => {
                if live.remove(result) {
                    mark_operand(*left, &mut live);

                    mark_operand(*right, &mut live);

                    kept.push(instruction.clone());
                } else {
                    eliminated += 1;
                }
            }

            SsaInstruction::Copy { result, source } => {
                if live.remove(result) {
                    mark_operand(*source, &mut live);

                    kept.push(instruction.clone());
                } else {
                    eliminated += 1;
                }
            }
        }
    }

    kept.reverse();

    (SsaProgram::new(kept), eliminated)
}

fn mark_operand(operand: Operand, live: &mut HashSet<ValueId>) {
    if let Operand::Value(id) = operand {
        live.insert(id);
    }
}

fn fold_binary(op: BinaryOp, left: i64, right: i64) -> Option<i64> {
    match op {
        BinaryOp::Add => left.checked_add(right),

        BinaryOp::Subtract => left.checked_sub(right),

        BinaryOp::Multiply => left.checked_mul(right),

        BinaryOp::Divide => {
            if right == 0 {
                None
            } else {
                left.checked_div(right)
            }
        }
    }
}
