use std::collections::{HashMap, HashSet};

use crate::ast::BinaryOp;
use crate::cfg::{BlockId, ControlFlowGraph, Instruction, Operand, Terminator, ValueId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lattice {
    Unknown,
    Constant(i64),
    Overdefined,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CfgOptimizationStats {
    pub blocks_before: usize,
    pub blocks_after: usize,
    pub instructions_before: usize,
    pub instructions_after: usize,
    pub phis_before: usize,
    pub phis_after: usize,
    pub branches_folded: usize,
    pub constants_propagated: usize,
    pub phis_eliminated: usize,
    pub dead_instructions_removed: usize,
}

impl CfgOptimizationStats {
    pub fn block_reduction_percent(&self) -> f64 {
        reduction_percent(self.blocks_before, self.blocks_after)
    }

    pub fn instruction_reduction_percent(&self) -> f64 {
        reduction_percent(self.instructions_before, self.instructions_after)
    }
}

pub struct CfgOptimizer;

impl CfgOptimizer {
    pub fn optimize(graph: &ControlFlowGraph) -> (ControlFlowGraph, CfgOptimizationStats) {
        let mut graph = graph.clone();

        let blocks_before = graph.blocks().len();
        let instructions_before = instruction_count(&graph);
        let phis_before = phi_count(&graph);

        let (values, executable) = analyze(&graph);

        let constants_propagated = rewrite_operands(&mut graph, &values, &executable);

        let branches_folded = fold_branches(&mut graph, &values, &executable);

        graph.prune_unreachable();

        let phis_eliminated = simplify_phis(&mut graph);

        let dead_instructions_removed = eliminate_dead_instructions(&mut graph);

        let stats = CfgOptimizationStats {
            blocks_before,
            blocks_after: graph.blocks().len(),
            instructions_before,
            instructions_after: instruction_count(&graph),
            phis_before,
            phis_after: phi_count(&graph),
            branches_folded,
            constants_propagated,
            phis_eliminated,
            dead_instructions_removed,
        };

        (graph, stats)
    }
}

fn analyze(graph: &ControlFlowGraph) -> (HashMap<ValueId, Lattice>, HashSet<BlockId>) {
    let mut values = HashMap::new();
    let mut executable = HashSet::from([graph.entry()]);

    loop {
        let old_values = values.clone();
        let old_executable = executable.clone();

        for block in graph.blocks() {
            if !executable.contains(&block.id()) {
                continue;
            }

            for instruction in block.instructions() {
                evaluate_instruction(instruction, &mut values, &executable);
            }

            match block.terminator() {
                Some(Terminator::Jump(target)) => {
                    executable.insert(*target);
                }

                Some(Terminator::Branch {
                    condition,
                    then_block,
                    else_block,
                }) => match lattice_of(*condition, &values) {
                    Lattice::Constant(value) => {
                        executable.insert(if value != 0 { *then_block } else { *else_block });
                    }

                    Lattice::Overdefined => {
                        executable.insert(*then_block);
                        executable.insert(*else_block);
                    }

                    Lattice::Unknown => {}
                },

                Some(Terminator::Return(_)) | None => {}
            }
        }

        if old_values == values && old_executable == executable {
            break;
        }
    }

    (values, executable)
}

fn evaluate_instruction(
    instruction: &Instruction,
    values: &mut HashMap<ValueId, Lattice>,
    executable: &HashSet<BlockId>,
) {
    match instruction {
        Instruction::Binary {
            result,
            op,
            left,
            right,
        } => {
            let next = evaluate_binary(*op, lattice_of(*left, values), lattice_of(*right, values));

            merge_value(values, *result, next);
        }

        Instruction::Phi {
            result, incomings, ..
        } => {
            let mut value = Lattice::Unknown;

            for (predecessor, operand) in incomings {
                if !executable.contains(predecessor) {
                    continue;
                }

                value = merge_lattice(value, lattice_of(*operand, values));
            }

            merge_value(values, *result, value);
        }

        Instruction::Bind { .. } => {}
    }
}

fn lattice_of(operand: Operand, values: &HashMap<ValueId, Lattice>) -> Lattice {
    match operand {
        Operand::Constant(value) => Lattice::Constant(value),

        Operand::Value(id) => values.get(&id).copied().unwrap_or(Lattice::Unknown),
    }
}

fn merge_value(values: &mut HashMap<ValueId, Lattice>, id: ValueId, next: Lattice) {
    let current = values.get(&id).copied().unwrap_or(Lattice::Unknown);

    values.insert(id, merge_lattice(current, next));
}

fn merge_lattice(left: Lattice, right: Lattice) -> Lattice {
    match (left, right) {
        (Lattice::Unknown, value) | (value, Lattice::Unknown) => value,

        (Lattice::Constant(left), Lattice::Constant(right)) if left == right => {
            Lattice::Constant(left)
        }

        (Lattice::Constant(_), Lattice::Constant(_))
        | (Lattice::Overdefined, _)
        | (_, Lattice::Overdefined) => Lattice::Overdefined,
    }
}

fn evaluate_binary(op: BinaryOp, left: Lattice, right: Lattice) -> Lattice {
    match (left, right) {
        (Lattice::Constant(left), Lattice::Constant(right)) => fold_binary(op, left, right)
            .map(Lattice::Constant)
            .unwrap_or(Lattice::Overdefined),

        (Lattice::Overdefined, _) | (_, Lattice::Overdefined) => Lattice::Overdefined,

        _ => Lattice::Unknown,
    }
}

fn rewrite_operands(
    graph: &mut ControlFlowGraph,
    values: &HashMap<ValueId, Lattice>,
    executable: &HashSet<BlockId>,
) -> usize {
    let mut propagated = 0;

    for block in graph.blocks_mut() {
        if !executable.contains(&block.id()) {
            continue;
        }

        for instruction in block.instructions_mut() {
            match instruction {
                Instruction::Binary { left, right, .. } => {
                    propagated += rewrite_operand(left, values);

                    propagated += rewrite_operand(right, values);
                }

                Instruction::Phi { incomings, .. } => {
                    for (_, operand) in incomings {
                        propagated += rewrite_operand(operand, values);
                    }
                }

                Instruction::Bind { value, .. } => {
                    propagated += rewrite_operand(value, values);
                }
            }
        }

        if let Some(terminator) = block.terminator_mut() {
            match terminator {
                Terminator::Return(value)
                | Terminator::Branch {
                    condition: value, ..
                } => {
                    propagated += rewrite_operand(value, values);
                }

                Terminator::Jump(_) => {}
            }
        }
    }

    propagated
}

fn rewrite_operand(operand: &mut Operand, values: &HashMap<ValueId, Lattice>) -> usize {
    let Operand::Value(id) = *operand else {
        return 0;
    };

    let Some(Lattice::Constant(value)) = values.get(&id) else {
        return 0;
    };

    *operand = Operand::Constant(*value);
    1
}

fn fold_branches(
    graph: &mut ControlFlowGraph,
    values: &HashMap<ValueId, Lattice>,
    executable: &HashSet<BlockId>,
) -> usize {
    let mut folded = 0;

    for block in graph.blocks_mut() {
        if !executable.contains(&block.id()) {
            continue;
        }

        let replacement = match block.terminator() {
            Some(Terminator::Branch {
                condition,
                then_block,
                else_block,
            }) => match lattice_of(*condition, values) {
                Lattice::Constant(value) => Some(Terminator::Jump(if value != 0 {
                    *then_block
                } else {
                    *else_block
                })),

                _ => None,
            },

            _ => None,
        };

        if let Some(replacement) = replacement {
            *block.terminator_mut() = Some(replacement);
            folded += 1;
        }
    }

    folded
}

fn simplify_phis(graph: &mut ControlFlowGraph) -> usize {
    let predecessors: HashMap<_, _> = graph
        .blocks()
        .iter()
        .map(|block| (block.id(), graph.predecessors(block.id())))
        .collect();

    let mut replacements = HashMap::<ValueId, Operand>::new();

    let mut eliminated = 0;

    for block in graph.blocks_mut() {
        let live_predecessors: HashSet<_> = predecessors
            .get(&block.id())
            .into_iter()
            .flatten()
            .copied()
            .collect();

        for instruction in block.instructions_mut() {
            if let Instruction::Phi {
                result, incomings, ..
            } = instruction
            {
                incomings.retain(|(predecessor, _)| live_predecessors.contains(predecessor));

                let trivial_phi = incomings.len() == 1
                    || (!incomings.is_empty()
                        && incomings
                            .iter()
                            .all(|(_, operand)| *operand == incomings[0].1));

                if trivial_phi {
                    replacements.insert(*result, incomings[0].1);
                }
            }
        }
    }

    if replacements.is_empty() {
        return 0;
    }

    for block in graph.blocks_mut() {
        for instruction in block.instructions_mut() {
            rewrite_instruction_replacements(instruction, &replacements);
        }

        if let Some(terminator) = block.terminator_mut() {
            rewrite_terminator_replacements(terminator, &replacements);
        }

        let before = block.instructions().len();

        block.instructions_mut().retain(|instruction| {
            !matches!(
                instruction,
                Instruction::Phi {
                    result,
                    ..
                } if replacements.contains_key(result)
            )
        });

        eliminated += before - block.instructions().len();
    }

    eliminated
}

fn rewrite_instruction_replacements(
    instruction: &mut Instruction,
    replacements: &HashMap<ValueId, Operand>,
) {
    match instruction {
        Instruction::Binary { left, right, .. } => {
            resolve_replacement(left, replacements);

            resolve_replacement(right, replacements);
        }

        Instruction::Phi { incomings, .. } => {
            for (_, operand) in incomings {
                resolve_replacement(operand, replacements);
            }
        }

        Instruction::Bind { value, .. } => {
            resolve_replacement(value, replacements);
        }
    }
}

fn rewrite_terminator_replacements(
    terminator: &mut Terminator,
    replacements: &HashMap<ValueId, Operand>,
) {
    match terminator {
        Terminator::Return(value)
        | Terminator::Branch {
            condition: value, ..
        } => {
            resolve_replacement(value, replacements);
        }

        Terminator::Jump(_) => {}
    }
}

fn resolve_replacement(operand: &mut Operand, replacements: &HashMap<ValueId, Operand>) {
    let mut visited = HashSet::new();

    while let Operand::Value(id) = *operand {
        if !visited.insert(id) {
            break;
        }

        let Some(next) = replacements.get(&id) else {
            break;
        };

        *operand = *next;
    }
}

fn eliminate_dead_instructions(graph: &mut ControlFlowGraph) -> usize {
    let mut live = HashSet::<ValueId>::new();

    /*
     * Seed liveness from terminators and phi inputs.
     */
    for block in graph.blocks() {
        if let Some(terminator) = block.terminator() {
            match terminator {
                Terminator::Return(value)
                | Terminator::Branch {
                    condition: value, ..
                } => {
                    mark_cfg_operand(*value, &mut live);
                }

                Terminator::Jump(_) => {}
            }
        }

        for instruction in block.instructions() {
            if let Instruction::Phi { incomings, .. } = instruction {
                for (_, operand) in incomings {
                    mark_cfg_operand(*operand, &mut live);
                }
            }
        }
    }

    /*
     * Fixed point because values may be defined in
     * predecessor blocks.
     */
    loop {
        let before = live.len();

        for block in graph.blocks().iter().rev() {
            for instruction in block.instructions().iter().rev() {
                match instruction {
                    Instruction::Binary {
                        result,
                        left,
                        right,
                        ..
                    } => {
                        if live.contains(result) {
                            mark_cfg_operand(*left, &mut live);

                            mark_cfg_operand(*right, &mut live);
                        }
                    }

                    Instruction::Phi {
                        result, incomings, ..
                    } => {
                        if live.contains(result) {
                            for (_, operand) in incomings {
                                mark_cfg_operand(*operand, &mut live);
                            }
                        }
                    }

                    Instruction::Bind { value, .. } => {
                        mark_cfg_operand(*value, &mut live);
                    }
                }
            }
        }

        if live.len() == before {
            break;
        }
    }

    let mut removed = 0;

    for block in graph.blocks_mut() {
        let before = block.instructions().len();

        block.instructions_mut().retain(|instruction| {
            match instruction {
                Instruction::Binary { result, .. } | Instruction::Phi { result, .. } => {
                    live.contains(result)
                }

                /*
                 * Bind retains source-level state.
                 * We eliminate these later once CFG
                 * SSA fully replaces symbol binds.
                 */
                Instruction::Bind { .. } => true,
            }
        });

        removed += before - block.instructions().len();
    }

    removed
}

fn mark_cfg_operand(operand: Operand, live: &mut HashSet<ValueId>) {
    if let Operand::Value(id) = operand {
        live.insert(id);
    }
}

fn instruction_count(graph: &ControlFlowGraph) -> usize {
    graph
        .blocks()
        .iter()
        .map(|block| block.instructions().len())
        .sum()
}

fn phi_count(graph: &ControlFlowGraph) -> usize {
    graph
        .blocks()
        .iter()
        .flat_map(|block| block.instructions())
        .filter(|instruction| matches!(instruction, Instruction::Phi { .. }))
        .count()
}

fn reduction_percent(before: usize, after: usize) -> f64 {
    if before == 0 {
        return 0.0;
    }

    100.0 * (before - after) as f64 / before as f64
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

        BinaryOp::Equal => Some(i64::from(left == right)),
        BinaryOp::NotEqual => Some(i64::from(left != right)),
        BinaryOp::Less => Some(i64::from(left < right)),
        BinaryOp::LessEqual => Some(i64::from(left <= right)),
        BinaryOp::Greater => Some(i64::from(left > right)),
        BinaryOp::GreaterEqual => Some(i64::from(left >= right)),
    }
}
