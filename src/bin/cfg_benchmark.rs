use std::collections::BTreeMap;

use helix::cfg::{CfgBuilder, ControlFlowGraph};
use helix::cfg_opt::CfgOptimizer;
use helix::cfg_runtime::CfgInterpreter;
use helix::lexer::Lexer;
use helix::parser::Parser;
use helix::types::TypeChecker;

#[derive(Default)]
struct Totals {
    programs: usize,
    equivalent: usize,

    blocks_before: usize,
    blocks_after: usize,

    instructions_before: usize,
    instructions_after: usize,

    phis_before: usize,
    phis_after: usize,

    branches_folded: usize,
    constants_propagated: usize,
    phis_eliminated: usize,
    dead_instructions_removed: usize,
}

fn build(source: &str) -> ControlFlowGraph {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap();

    let mut checker = TypeChecker::new();
    let typed = checker.check(&program).unwrap();

    CfgBuilder::new().build(&typed)
}

fn literal(value: i64) -> String {
    if value < 0 {
        format!("0 - {}", value.unsigned_abs())
    } else {
        value.to_string()
    }
}

fn programs() -> Vec<String> {
    let mut corpus = Vec::new();

    /*
     * Deterministic arithmetic workloads.
     */
    for a in -10_i64..=10 {
        for b in -10_i64..=10 {
            let a = literal(a);
            let b = literal(b);

            corpus.push(format!(
                "
                let x = {a} + {b};
                return x * 2;
                "
            ));
        }
    }

    /*
     * Constant branch workloads.
     */
    for a in -10_i64..=10 {
        for b in -10_i64..=10 {
            let a = literal(a);
            let b = literal(b);

            corpus.push(format!(
                "
                let x = {a};

                if x > {b} {{
                    return x + 1;
                }} else {{
                    return x - 1;
                }}
                "
            ));
        }
    }

    /*
     * Phi-producing assignments.
     */
    for a in -8_i64..=8 {
        for b in -8_i64..=8 {
            let a_text = literal(a);
            let b_text = literal(b);

            corpus.push(format!(
                "
                let x = 0;

                if {a_text} >= {b_text} {{
                    x = {a_text} + 3;
                }} else {{
                    x = {b_text} - 2;
                }}

                return x;
                "
            ));
        }
    }

    /*
     * Nested branch workloads.
     */
    for x in -8_i64..=8 {
        let x = literal(x);

        corpus.push(format!(
            "
            let x = {x};

            if x > 0 {{
                if x < 5 {{
                    return x * 10;
                }} else {{
                    return x + 100;
                }}
            }} else {{
                return 0 - x;
            }}
            "
        ));
    }

    /*
     * Dead-expression workloads.
     */
    for x in 1_i64..=100 {
        corpus.push(format!(
            "
            let unused = {x} * {x};
            let result = {x} + 7;
            return result;
            "
        ));
    }

    corpus
}

fn percent(before: usize, after: usize) -> f64 {
    if before == 0 {
        return 0.0;
    }

    100.0 * (before - after) as f64 / before as f64
}

fn main() {
    let corpus = programs();

    let mut totals = Totals::default();

    let mut outcomes = BTreeMap::<i64, usize>::new();

    for source in &corpus {
        let cfg = build(source);

        let before_result = CfgInterpreter::execute(&cfg).expect("unoptimized execution failed");

        let (optimized, stats) = CfgOptimizer::optimize(&cfg);

        let after_result = CfgInterpreter::execute(&optimized).expect("optimized execution failed");

        assert_eq!(before_result, after_result, "semantic mismatch:\n{source}");

        *outcomes.entry(after_result).or_default() += 1;

        totals.programs += 1;
        totals.equivalent += 1;

        totals.blocks_before += stats.blocks_before;
        totals.blocks_after += stats.blocks_after;

        totals.instructions_before += stats.instructions_before;
        totals.instructions_after += stats.instructions_after;

        totals.phis_before += stats.phis_before;
        totals.phis_after += stats.phis_after;

        totals.branches_folded += stats.branches_folded;

        totals.constants_propagated += stats.constants_propagated;

        totals.phis_eliminated += stats.phis_eliminated;

        totals.dead_instructions_removed += stats.dead_instructions_removed;
    }

    println!("metric,value");

    println!("programs,{}", totals.programs);

    println!("semantically_equivalent,{}", totals.equivalent);

    println!("blocks_before,{}", totals.blocks_before);

    println!("blocks_after,{}", totals.blocks_after);

    println!(
        "block_reduction_percent,{:.2}",
        percent(totals.blocks_before, totals.blocks_after,)
    );

    println!("instructions_before,{}", totals.instructions_before);

    println!("instructions_after,{}", totals.instructions_after);

    println!(
        "instruction_reduction_percent,{:.2}",
        percent(totals.instructions_before, totals.instructions_after,)
    );

    println!("phis_before,{}", totals.phis_before);

    println!("phis_after,{}", totals.phis_after);

    println!(
        "phi_reduction_percent,{:.2}",
        percent(totals.phis_before, totals.phis_after,)
    );

    println!("branches_folded,{}", totals.branches_folded);

    println!("constants_propagated,{}", totals.constants_propagated);

    println!("phis_eliminated,{}", totals.phis_eliminated);

    println!(
        "dead_instructions_removed,{}",
        totals.dead_instructions_removed
    );

    println!("distinct_results,{}", outcomes.len());
}
