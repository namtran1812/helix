# Helix

Helix is an optimizing compiler written in Rust that implements a small typed language and a compiler middle-end centered on control-flow graphs, SSA-style values, dominance analysis, phi nodes, sparse conditional constant propagation, and dead-code elimination.

## Architecture

Source
  -> Lexer
  -> Parser
  -> Typed AST
  -> Semantic Analysis
  -> CFG Construction
  -> Dominance / Dominance Frontiers
  -> SSA / Phi Nodes
  -> SCCP
  -> Branch Folding
  -> Unreachable-Block Pruning
  -> Phi Simplification
  -> Dead-Code Elimination
  -> Optimized CFG

## Features

- Precedence-aware lexer and parser
- Typed AST and semantic analysis
- Symbol resolution and assignments
- Control-flow graph construction
- Predecessor/successor and reachability analysis
- Dominators, immediate dominators, dominator trees, dominance frontiers
- SSA-style values and phi nodes
- Sparse conditional constant propagation
- Constant propagation and branch folding
- Unreachable-block pruning
- Trivial phi elimination
- Dead-code elimination
- CFG interpreter
- Optimized-vs-unoptimized differential validation
- Deterministic optimizer benchmark corpus

## Correctness Validation

Helix validates compiler transformations by executing each program before and after CFG optimization and checking that both executions return the same result.

Current deterministic corpus:

- Programs evaluated: 1,288
- Semantically equivalent: 1,288
- Equivalence rate: 100%
- Distinct results: 135

## Optimization Results

Across the 1,288-program deterministic corpus:

| Metric | Before | After | Reduction |
|---|---:|---:|---:|
| Basic blocks | 3,105 | 2,332 | 24.90% |
| Instructions | 7,004 | 1,677 | 76.06% |
| Phi nodes | 289 | 0 | 100.00% |

Additional optimizer activity:

| Optimization | Count |
|---|---:|
| Branches folded | 755 |
| Constants propagated | 5,323 |
| Phi nodes eliminated | 289 |
| Dead instructions removed | 3,768 |

These measurements describe the checked-in deterministic benchmark corpus and are not intended as general compiler-performance claims.

## Run Tests

cargo test

## Strict Static Analysis

cargo clippy --all-targets --all-features -- -D warnings

## Check Formatting

cargo fmt --check

## Run Optimizer Benchmark

cargo run --release --bin cfg_benchmark

Benchmark output is stored in:

experiments/cfg_optimizer.csv

## Project Structure

src/
  ast/             AST
  cfg/             Control-flow graph
  dominance/       Dominator and frontier analysis
  ir/              Intermediate representation
  lexer/           Lexer
  parser/          Parser
  runtime/         Runtime
  ssa/             SSA representation and optimizer
  types/           Type and semantic analysis
  cfg_opt.rs       CFG optimization pipeline
  cfg_runtime.rs   CFG interpreter
  bin/
    cfg_benchmark.rs

tests/
  cfg.rs
  cfg_differential.rs
  cfg_opt.rs
  dominance.rs
  frontend.rs
  semantics.rs
  ssa.rs

experiments/
  cfg_optimizer.csv

## Status

Helix v1.0 implements a typed compiler middle-end with CFG/SSA optimization, dominance analysis, differential semantic validation, and reproducible quantitative evaluation.
