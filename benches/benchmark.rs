#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use criterion::{BatchSize, Criterion};
use criterion_macro::criterion;
use lsp_types::Edit;
use zed_benchmark::{bench_modified, bench_original};

#[criterion]
fn benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("compare");
    let edits = produce_edits();

    g.bench_function("original", |b| {
        b.iter_batched(|| edits.clone(), bench_original, BatchSize::SmallInput);
    });
    g.bench_function("modified", |b| {
        b.iter_batched(|| edits.clone(), bench_modified, BatchSize::SmallInput);
    });

    g.finish();
}

fn produce_edits() -> Vec<Vec<(Edit, bool)>> {
    // 1. Get a tree diff of a commit range in the `zed-industries/zed` repo.
    // 2. Save the files from the diff in memory.
    // 3. Perform diffing of each file with `imara-diff`.
    // 4. Semi-deterministically pick one file to be the `active_entry`.
    // 5. For each file, build a vector of `Edit`s with the ranges and text
    //    contained in the lines that the diffing results provide.
    // 6. Gather all file vectors into a single output file vector.
    todo!()
}

fn get_commits() {}

fn save_files() {}

fn diff_files() {}

fn pick_active_entry() {}

fn build_edits() {}

fn gather_edits() {}
