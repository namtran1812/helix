use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Program, Statement};

pub type SymbolId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    I64,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    id: SymbolId,
    name: String,
    ty: Type,
}

impl Symbol {
    pub fn id(&self) -> SymbolId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ty(&self) -> Type {
        self.ty
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedExpr {
    Integer {
        value: i64,
        ty: Type,
    },

    Boolean {
        value: bool,
        ty: Type,
    },

    Symbol {
        symbol_id: SymbolId,
        name: String,
        ty: Type,
    },

    Binary {
        left: Box<TypedExpr>,
        op: BinaryOp,
        right: Box<TypedExpr>,
        ty: Type,
    },
}

impl TypedExpr {
    pub fn ty(&self) -> Type {
        match self {
            Self::Integer { ty, .. }
            | Self::Boolean { ty, .. }
            | Self::Symbol { ty, .. }
            | Self::Binary { ty, .. } => *ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedStatement {
    Let {
        symbol: Symbol,
        value: TypedExpr,
    },

    Return {
        value: TypedExpr,
    },

    If {
        condition: TypedExpr,
        then_branch: Vec<TypedStatement>,
        else_branch: Vec<TypedStatement>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    statements: Vec<TypedStatement>,
    symbols: Vec<Symbol>,
}

impl TypedProgram {
    pub fn statements(&self) -> &[TypedStatement] {
        &self.statements
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SemanticError {
    #[error("undefined identifier: {0}")]
    UndefinedIdentifier(String),

    #[error("duplicate binding: {0}")]
    DuplicateBinding(String),

    #[error("program does not contain a return statement")]
    MissingReturn,

    #[error("if condition must have type bool")]
    NonBooleanCondition,

    #[error("invalid operands for binary operator")]
    InvalidBinaryOperands,
}

pub struct TypeChecker {
    symbols_by_name: HashMap<String, Symbol>,

    symbols: Vec<Symbol>,

    next_symbol_id: SymbolId,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            symbols_by_name: HashMap::new(),

            symbols: Vec::new(),

            next_symbol_id: 0,
        }
    }

    pub fn check(&mut self, program: &Program) -> Result<TypedProgram, SemanticError> {
        let statements = self.check_block(&program.statements)?;

        if !block_returns(&program.statements) {
            return Err(SemanticError::MissingReturn);
        }

        Ok(TypedProgram {
            statements,
            symbols: self.symbols.clone(),
        })
    }

    fn check_block(
        &mut self,
        statements: &[Statement],
    ) -> Result<Vec<TypedStatement>, SemanticError> {
        let mut typed = Vec::with_capacity(statements.len());

        for statement in statements {
            match statement {
                Statement::Let { name, value } => {
                    if self.symbols_by_name.contains_key(name) {
                        return Err(SemanticError::DuplicateBinding(name.clone()));
                    }

                    let value = self.check_expr(value)?;

                    let symbol = Symbol {
                        id: self.next_symbol_id,

                        name: name.clone(),

                        ty: value.ty(),
                    };

                    self.next_symbol_id += 1;

                    self.symbols_by_name.insert(name.clone(), symbol.clone());

                    self.symbols.push(symbol.clone());

                    typed.push(TypedStatement::Let { symbol, value });
                }

                Statement::Return(value) => {
                    typed.push(TypedStatement::Return {
                        value: self.check_expr(value)?,
                    });
                }

                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let condition = self.check_expr(condition)?;

                    if condition.ty() != Type::Bool {
                        return Err(SemanticError::NonBooleanCondition);
                    }

                    /*
                     * Preserve the outer environment
                     * while checking each branch.
                     *
                     * Branch-local bindings do not leak
                     * into sibling branches.
                     */
                    let outer_symbols = self.symbols_by_name.clone();

                    let then_branch = self.check_block(then_branch)?;

                    self.symbols_by_name = outer_symbols.clone();

                    let else_branch = self.check_block(else_branch)?;

                    self.symbols_by_name = outer_symbols;

                    typed.push(TypedStatement::If {
                        condition,
                        then_branch,
                        else_branch,
                    });
                }
            }
        }

        Ok(typed)
    }

    fn check_expr(&self, expr: &Expr) -> Result<TypedExpr, SemanticError> {
        match expr {
            Expr::Integer(value) => Ok(TypedExpr::Integer {
                value: *value,
                ty: Type::I64,
            }),

            Expr::Boolean(value) => Ok(TypedExpr::Boolean {
                value: *value,
                ty: Type::Bool,
            }),

            Expr::Identifier(name) => {
                let symbol = self
                    .symbols_by_name
                    .get(name)
                    .ok_or_else(|| SemanticError::UndefinedIdentifier(name.clone()))?;

                Ok(TypedExpr::Symbol {
                    symbol_id: symbol.id(),

                    name: symbol.name().to_string(),

                    ty: symbol.ty(),
                })
            }

            Expr::Binary { left, op, right } => {
                let left = self.check_expr(left)?;

                let right = self.check_expr(right)?;

                let ty = binary_result_type(*op, left.ty(), right.ty())?;

                Ok(TypedExpr::Binary {
                    left: Box::new(left),

                    op: *op,

                    right: Box::new(right),

                    ty,
                })
            }
        }
    }
}

fn binary_result_type(op: BinaryOp, left: Type, right: Type) -> Result<Type, SemanticError> {
    match op {
        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
            if left == Type::I64 && right == Type::I64 {
                Ok(Type::I64)
            } else {
                Err(SemanticError::InvalidBinaryOperands)
            }
        }

        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            if left == Type::I64 && right == Type::I64 {
                Ok(Type::Bool)
            } else {
                Err(SemanticError::InvalidBinaryOperands)
            }
        }

        BinaryOp::Equal | BinaryOp::NotEqual => {
            if left == right {
                Ok(Type::Bool)
            } else {
                Err(SemanticError::InvalidBinaryOperands)
            }
        }
    }
}

fn block_returns(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Return(_) => true,

        Statement::If {
            then_branch,
            else_branch,
            ..
        } => block_returns(then_branch) && block_returns(else_branch),

        Statement::Let { .. } => false,
    })
}
