use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::ast::BinaryOp;
use crate::types::{SymbolId, TypedExpr, TypedProgram, TypedStatement};

pub type BlockId = u32;
pub type ValueId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Constant(i64),
    Value(ValueId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Binary {
        result: ValueId,
        op: BinaryOp,
        left: Operand,
        right: Operand,
    },

    Bind {
        symbol_id: SymbolId,
        value: Operand,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator {
    Return(Operand),

    Jump(BlockId),

    Branch {
        condition: Operand,
        then_block: BlockId,
        else_block: BlockId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    id: BlockId,
    instructions: Vec<Instruction>,
    terminator: Option<Terminator>,
}

impl BasicBlock {
    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn terminator(&self) -> Option<&Terminator> {
        self.terminator.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlFlowGraph {
    entry: BlockId,
    blocks: Vec<BasicBlock>,
}

impl ControlFlowGraph {
    pub fn entry(&self) -> BlockId {
        self.entry
    }

    pub fn blocks(&self) -> &[BasicBlock] {
        &self.blocks
    }

    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|block| block.id == id)
    }

    pub fn successors(&self, id: BlockId) -> Vec<BlockId> {
        let Some(block) = self.block(id) else {
            return Vec::new();
        };

        match block.terminator() {
            Some(Terminator::Jump(target)) => vec![*target],

            Some(Terminator::Branch {
                then_block,
                else_block,
                ..
            }) => vec![*then_block, *else_block],

            Some(Terminator::Return(_)) | None => Vec::new(),
        }
    }

    pub fn predecessors(&self, id: BlockId) -> Vec<BlockId> {
        self.blocks
            .iter()
            .filter_map(|block| {
                self.successors(block.id())
                    .contains(&id)
                    .then_some(block.id())
            })
            .collect()
    }

    pub fn reachable_blocks(&self) -> BTreeSet<BlockId> {
        let mut reachable = BTreeSet::new();
        let mut queue = VecDeque::from([self.entry]);

        while let Some(block) = queue.pop_front() {
            if !reachable.insert(block) {
                continue;
            }

            queue.extend(self.successors(block));
        }

        reachable
    }

    pub fn prune_unreachable(&mut self) {
        let reachable = self.reachable_blocks();

        self.blocks.retain(|block| reachable.contains(&block.id()));
    }
}

pub struct CfgBuilder {
    blocks: Vec<BasicBlock>,
    current: BlockId,
    next_value: ValueId,
    definitions: HashMap<SymbolId, Operand>,
}

impl Default for CfgBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CfgBuilder {
    pub fn new() -> Self {
        Self {
            blocks: vec![BasicBlock {
                id: 0,
                instructions: Vec::new(),
                terminator: None,
            }],
            current: 0,
            next_value: 0,
            definitions: HashMap::new(),
        }
    }

    pub fn build(mut self, program: &TypedProgram) -> ControlFlowGraph {
        self.lower_statements(program.statements());

        let mut graph = ControlFlowGraph {
            entry: 0,
            blocks: self.blocks,
        };

        graph.prune_unreachable();
        graph
    }

    fn lower_statements(&mut self, statements: &[TypedStatement]) {
        for statement in statements {
            if self.current_block().terminator.is_some() {
                break;
            }

            match statement {
                TypedStatement::Let { symbol, value } => {
                    let value = self.lower_expr(value);

                    self.current_block_mut()
                        .instructions
                        .push(Instruction::Bind {
                            symbol_id: symbol.id(),
                            value,
                        });

                    self.definitions.insert(symbol.id(), value);
                }

                TypedStatement::Return { value } => {
                    let value = self.lower_expr(value);

                    self.current_block_mut().terminator = Some(Terminator::Return(value));
                }

                TypedStatement::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    self.lower_if(condition, then_branch, else_branch);
                }
            }
        }
    }

    fn lower_if(
        &mut self,
        condition: &TypedExpr,
        then_branch: &[TypedStatement],
        else_branch: &[TypedStatement],
    ) {
        let condition = self.lower_expr(condition);

        let then_block = self.new_block();
        let else_block = self.new_block();
        let merge_block = self.new_block();

        self.current_block_mut().terminator = Some(Terminator::Branch {
            condition,
            then_block,
            else_block,
        });

        self.current = then_block;
        self.lower_statements(then_branch);

        if self.current_block().terminator.is_none() {
            self.current_block_mut().terminator = Some(Terminator::Jump(merge_block));
        }

        self.current = else_block;
        self.lower_statements(else_branch);

        if self.current_block().terminator.is_none() {
            self.current_block_mut().terminator = Some(Terminator::Jump(merge_block));
        }

        self.current = merge_block;
    }

    fn lower_expr(&mut self, expr: &TypedExpr) -> Operand {
        match expr {
            TypedExpr::Integer { value, .. } => Operand::Constant(*value),

            TypedExpr::Boolean { value, .. } => Operand::Constant(i64::from(*value)),

            TypedExpr::Symbol { symbol_id, .. } => *self
                .definitions
                .get(symbol_id)
                .expect("typed CFG referenced undefined symbol"),

            TypedExpr::Binary {
                left, op, right, ..
            } => {
                let left = self.lower_expr(left);
                let right = self.lower_expr(right);
                let result = self.allocate_value();

                self.current_block_mut()
                    .instructions
                    .push(Instruction::Binary {
                        result,
                        op: *op,
                        left,
                        right,
                    });

                Operand::Value(result)
            }
        }
    }

    fn new_block(&mut self) -> BlockId {
        let id = self.blocks.len() as BlockId;

        self.blocks.push(BasicBlock {
            id,
            instructions: Vec::new(),
            terminator: None,
        });

        id
    }

    fn allocate_value(&mut self) -> ValueId {
        let id = self.next_value;
        self.next_value += 1;
        id
    }

    fn current_block(&self) -> &BasicBlock {
        &self.blocks[self.current as usize]
    }

    fn current_block_mut(&mut self) -> &mut BasicBlock {
        &mut self.blocks[self.current as usize]
    }
}
