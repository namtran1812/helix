use std::collections::{BTreeSet, HashMap};

use crate::cfg::{BlockId, ControlFlowGraph};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DominanceInfo {
    dominators: HashMap<BlockId, BTreeSet<BlockId>>,
    immediate_dominators: HashMap<BlockId, BlockId>,
    dominance_frontiers: HashMap<BlockId, BTreeSet<BlockId>>,
}

impl DominanceInfo {
    pub fn compute(cfg: &ControlFlowGraph) -> Self {
        let reachable = cfg.reachable_blocks();
        let entry = cfg.entry();

        let mut dominators = HashMap::new();

        for &block in &reachable {
            if block == entry {
                dominators.insert(block, BTreeSet::from([entry]));
            } else {
                dominators.insert(block, reachable.clone());
            }
        }

        loop {
            let mut changed = false;

            for &block in &reachable {
                if block == entry {
                    continue;
                }

                let predecessors: Vec<_> = cfg
                    .predecessors(block)
                    .into_iter()
                    .filter(|pred| reachable.contains(pred))
                    .collect();

                let mut new_set = if let Some(first) = predecessors.first() {
                    dominators.get(first).cloned().unwrap_or_default()
                } else {
                    BTreeSet::new()
                };

                for predecessor in predecessors.iter().skip(1) {
                    let predecessor_dominators =
                        dominators.get(predecessor).cloned().unwrap_or_default();

                    new_set = new_set
                        .intersection(&predecessor_dominators)
                        .copied()
                        .collect();
                }

                new_set.insert(block);

                if dominators.get(&block) != Some(&new_set) {
                    dominators.insert(block, new_set);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        let immediate_dominators = Self::compute_immediate_dominators(entry, &dominators);

        let dominance_frontiers =
            Self::compute_dominance_frontiers(cfg, &reachable, &immediate_dominators);

        Self {
            dominators,
            immediate_dominators,
            dominance_frontiers,
        }
    }

    pub fn dominates(&self, dominator: BlockId, block: BlockId) -> bool {
        self.dominators
            .get(&block)
            .is_some_and(|set| set.contains(&dominator))
    }

    pub fn dominators(&self, block: BlockId) -> Option<&BTreeSet<BlockId>> {
        self.dominators.get(&block)
    }

    pub fn immediate_dominator(&self, block: BlockId) -> Option<BlockId> {
        self.immediate_dominators.get(&block).copied()
    }

    pub fn dominance_frontier(&self, block: BlockId) -> Option<&BTreeSet<BlockId>> {
        self.dominance_frontiers.get(&block)
    }

    pub fn dominator_tree_children(&self, block: BlockId) -> Vec<BlockId> {
        let mut children: Vec<_> = self
            .immediate_dominators
            .iter()
            .filter_map(|(&child, &parent)| (parent == block).then_some(child))
            .collect();

        children.sort_unstable();
        children
    }

    fn compute_immediate_dominators(
        entry: BlockId,
        dominators: &HashMap<BlockId, BTreeSet<BlockId>>,
    ) -> HashMap<BlockId, BlockId> {
        let mut result = HashMap::new();

        for (&block, block_dominators) in dominators {
            if block == entry {
                continue;
            }

            let strict: Vec<_> = block_dominators
                .iter()
                .copied()
                .filter(|candidate| *candidate != block)
                .collect();

            let immediate = strict.iter().copied().find(|candidate| {
                strict.iter().all(|other| {
                    other == candidate
                        || !dominators
                            .get(other)
                            .is_some_and(|set| set.contains(candidate))
                })
            });

            if let Some(immediate) = immediate {
                result.insert(block, immediate);
            }
        }

        result
    }

    fn compute_dominance_frontiers(
        cfg: &ControlFlowGraph,
        reachable: &BTreeSet<BlockId>,
        immediate_dominators: &HashMap<BlockId, BlockId>,
    ) -> HashMap<BlockId, BTreeSet<BlockId>> {
        let mut frontiers: HashMap<_, _> = reachable
            .iter()
            .copied()
            .map(|block| (block, BTreeSet::new()))
            .collect();

        for &block in reachable {
            let predecessors: Vec<_> = cfg
                .predecessors(block)
                .into_iter()
                .filter(|pred| reachable.contains(pred))
                .collect();

            if predecessors.len() < 2 {
                continue;
            }

            let Some(&block_idom) = immediate_dominators.get(&block) else {
                continue;
            };

            for predecessor in predecessors {
                let mut runner = predecessor;

                while runner != block_idom {
                    frontiers.entry(runner).or_default().insert(block);

                    let Some(&parent) = immediate_dominators.get(&runner) else {
                        break;
                    };

                    if parent == runner {
                        break;
                    }

                    runner = parent;
                }
            }
        }

        frontiers
    }
}
