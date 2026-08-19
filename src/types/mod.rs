use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Program, Statement};

pub type SymbolId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    I64,
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
            Self::Integer { ty, .. } | Self::Symbol { ty, .. } | Self::Binary { ty, .. } => *ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedStatement {
    Let { symbol: Symbol, value: TypedExpr },

    Return { value: TypedExpr },
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
        let mut statements = Vec::with_capacity(program.statements.len());

        let mut saw_return = false;

        for statement in &program.statements {
            match statement {
                Statement::Let { name, value } => {
                    if self.symbols_by_name.contains_key(name) {
                        return Err(SemanticError::DuplicateBinding(name.clone()));
                    }

                    let typed_value = self.check_expr(value)?;

                    let symbol = Symbol {
                        id: self.next_symbol_id,
                        name: name.clone(),
                        ty: typed_value.ty(),
                    };

                    self.next_symbol_id += 1;

                    self.symbols_by_name.insert(name.clone(), symbol.clone());

                    self.symbols.push(symbol.clone());

                    statements.push(TypedStatement::Let {
                        symbol,
                        value: typed_value,
                    });
                }

                Statement::Return(value) => {
                    let typed_value = self.check_expr(value)?;

                    statements.push(TypedStatement::Return { value: typed_value });

                    saw_return = true;
                }
            }
        }

        if !saw_return {
            return Err(SemanticError::MissingReturn);
        }

        Ok(TypedProgram {
            statements,
            symbols: self.symbols.clone(),
        })
    }

    fn check_expr(&self, expr: &Expr) -> Result<TypedExpr, SemanticError> {
        match expr {
            Expr::Integer(value) => Ok(TypedExpr::Integer {
                value: *value,
                ty: Type::I64,
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
                let typed_left = self.check_expr(left)?;

                let typed_right = self.check_expr(right)?;

                Ok(TypedExpr::Binary {
                    left: Box::new(typed_left),
                    op: *op,
                    right: Box::new(typed_right),
                    ty: Type::I64,
                })
            }
        }
    }
}
