use std::collections::HashMap;

use crate::ast::BinaryOp;
use crate::cfg::{BlockId, ControlFlowGraph, Instruction, Operand, Terminator, ValueId};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CfgRuntimeError {
    #[error("CFG block does not exist: {0}")]
    MissingBlock(BlockId),

    #[error("SSA value does not exist: {0}")]
    MissingValue(ValueId),

    #[error("phi node has no incoming value for predecessor")]
    MissingPhiInput,

    #[error("division by zero")]
    DivisionByZero,

    #[error("CFG terminated without return")]
    MissingReturn,
}

pub struct CfgInterpreter;

impl CfgInterpreter {
    pub fn execute(graph: &ControlFlowGraph) -> Result<i64, CfgRuntimeError> {
        let mut values = HashMap::<ValueId, i64>::new();

        let mut current = graph.entry();
        let mut predecessor: Option<BlockId> = None;

        loop {
            let block = graph
                .block(current)
                .ok_or(CfgRuntimeError::MissingBlock(current))?;

            /*
             * Phi nodes conceptually execute simultaneously
             * on block entry. Evaluate them before ordinary
             * instructions using the incoming CFG edge.
             */
            let mut phi_results = Vec::new();

            for instruction in block.instructions() {
                if let Instruction::Phi {
                    result, incomings, ..
                } = instruction
                {
                    let predecessor = predecessor.ok_or(CfgRuntimeError::MissingPhiInput)?;

                    let operand = incomings
                        .iter()
                        .find_map(|(block, operand)| (*block == predecessor).then_some(*operand))
                        .ok_or(CfgRuntimeError::MissingPhiInput)?;

                    let value = resolve_operand(operand, &values)?;

                    phi_results.push((*result, value));
                }
            }

            for (result, value) in phi_results {
                values.insert(result, value);
            }

            for instruction in block.instructions() {
                match instruction {
                    Instruction::Binary {
                        result,
                        op,
                        left,
                        right,
                    } => {
                        let left = resolve_operand(*left, &values)?;

                        let right = resolve_operand(*right, &values)?;

                        let value = execute_binary(*op, left, right)?;

                        values.insert(*result, value);
                    }

                    Instruction::Phi { .. } => {}

                    /*
                     * Bind represents source-level bookkeeping.
                     * All value dependencies are already encoded
                     * through operands, so it has no runtime effect.
                     */
                    Instruction::Bind { .. } => {}
                }
            }

            match block.terminator().ok_or(CfgRuntimeError::MissingReturn)? {
                Terminator::Return(value) => {
                    return resolve_operand(*value, &values);
                }

                Terminator::Jump(target) => {
                    predecessor = Some(current);
                    current = *target;
                }

                Terminator::Branch {
                    condition,
                    then_block,
                    else_block,
                } => {
                    let condition = resolve_operand(*condition, &values)?;

                    predecessor = Some(current);

                    current = if condition != 0 {
                        *then_block
                    } else {
                        *else_block
                    };
                }
            }
        }
    }
}

fn resolve_operand(
    operand: Operand,
    values: &HashMap<ValueId, i64>,
) -> Result<i64, CfgRuntimeError> {
    match operand {
        Operand::Constant(value) => Ok(value),

        Operand::Value(id) => values
            .get(&id)
            .copied()
            .ok_or(CfgRuntimeError::MissingValue(id)),
    }
}

fn execute_binary(op: BinaryOp, left: i64, right: i64) -> Result<i64, CfgRuntimeError> {
    match op {
        BinaryOp::Add => Ok(left + right),
        BinaryOp::Subtract => Ok(left - right),
        BinaryOp::Multiply => Ok(left * right),

        BinaryOp::Divide => {
            if right == 0 {
                Err(CfgRuntimeError::DivisionByZero)
            } else {
                Ok(left / right)
            }
        }

        BinaryOp::Equal => Ok(i64::from(left == right)),
        BinaryOp::NotEqual => Ok(i64::from(left != right)),
        BinaryOp::Less => Ok(i64::from(left < right)),
        BinaryOp::LessEqual => Ok(i64::from(left <= right)),
        BinaryOp::Greater => Ok(i64::from(left > right)),
        BinaryOp::GreaterEqual => Ok(i64::from(left >= right)),
    }
}
