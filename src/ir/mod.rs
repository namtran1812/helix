use crate::ast::BinaryOp;
use crate::types::{SymbolId, Type, TypedExpr, TypedProgram, TypedStatement};

#[derive(Debug, Clone, PartialEq)]
pub enum IrValue {
    Constant {
        value: i64,
        ty: Type,
    },

    Boolean {
        value: bool,
        ty: Type,
    },

    Symbol {
        symbol_id: SymbolId,
        ty: Type,
    },

    Binary {
        left: Box<IrValue>,
        op: BinaryOp,
        right: Box<IrValue>,
        ty: Type,
    },
}

impl IrValue {
    pub fn ty(&self) -> Type {
        match self {
            Self::Constant { ty, .. }
            | Self::Boolean { ty, .. }
            | Self::Symbol { ty, .. }
            | Self::Binary { ty, .. } => *ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrInstruction {
    Bind { symbol_id: SymbolId, value: IrValue },

    Return { value: IrValue },
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrProgram {
    instructions: Vec<IrInstruction>,
}

impl IrProgram {
    pub fn instructions(&self) -> &[IrInstruction] {
        &self.instructions
    }
}

pub struct Lowerer;

impl Lowerer {
    pub fn lower(program: &TypedProgram) -> IrProgram {
        let instructions = program
            .statements()
            .iter()
            .map(|statement| match statement {
                TypedStatement::Let { symbol, value } => IrInstruction::Bind {
                    symbol_id: symbol.id(),
                    value: lower_expr(value),
                },

                TypedStatement::Return { value } => IrInstruction::Return {
                    value: lower_expr(value),
                },

                TypedStatement::If { .. } => {
                    panic!("control flow must be lowered through CfgBuilder")
                }
            })
            .collect();

        IrProgram { instructions }
    }
}

fn lower_expr(expr: &TypedExpr) -> IrValue {
    match expr {
        TypedExpr::Integer { value, ty } => IrValue::Constant {
            value: *value,
            ty: *ty,
        },

        TypedExpr::Boolean { value, ty } => IrValue::Boolean {
            value: *value,
            ty: *ty,
        },

        TypedExpr::Symbol { symbol_id, ty, .. } => IrValue::Symbol {
            symbol_id: *symbol_id,
            ty: *ty,
        },

        TypedExpr::Binary {
            left,
            op,
            right,
            ty,
        } => IrValue::Binary {
            left: Box::new(lower_expr(left)),
            op: *op,
            right: Box::new(lower_expr(right)),
            ty: *ty,
        },
    }
}
