use std::collections::HashMap;

use crate::ast::BinaryOp;
use crate::ir::{IrInstruction, IrProgram, IrValue};
use crate::types::SymbolId;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("undefined runtime symbol: {0}")]
    UndefinedSymbol(SymbolId),

    #[error("division by zero")]
    DivisionByZero,

    #[error("program completed without return")]
    MissingReturn,
}

pub struct Interpreter {
    values: HashMap<SymbolId, i64>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn execute(&mut self, program: &IrProgram) -> Result<i64, RuntimeError> {
        for instruction in program.instructions() {
            match instruction {
                IrInstruction::Bind { symbol_id, value } => {
                    let value = self.eval(value)?;

                    self.values.insert(*symbol_id, value);
                }

                IrInstruction::Return { value } => {
                    return self.eval(value);
                }
            }
        }

        Err(RuntimeError::MissingReturn)
    }

    fn eval(&self, value: &IrValue) -> Result<i64, RuntimeError> {
        match value {
            IrValue::Constant { value, .. } => Ok(*value),

            IrValue::Symbol { symbol_id, .. } => self
                .values
                .get(symbol_id)
                .copied()
                .ok_or(RuntimeError::UndefinedSymbol(*symbol_id)),

            IrValue::Binary {
                left, op, right, ..
            } => {
                let left = self.eval(left)?;

                let right = self.eval(right)?;

                match op {
                    BinaryOp::Add => Ok(left + right),

                    BinaryOp::Subtract => Ok(left - right),

                    BinaryOp::Multiply => Ok(left * right),

                    BinaryOp::Divide => {
                        if right == 0 {
                            Err(RuntimeError::DivisionByZero)
                        } else {
                            Ok(left / right)
                        }
                    }
                }
            }
        }
    }
}
