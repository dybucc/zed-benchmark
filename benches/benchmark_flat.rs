#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use std::time::Duration;

use criterion::{BatchSize, Criterion};
use criterion_macro::criterion;
use zed_benchmark::{bench_modified, bench_original};

mod benchmark_utils;

pub use benchmark_utils::{clone_repo, new_repo_path, produce_edits};

fn custom_criterion() -> Criterion {
    // Had to enable this after the first few runs got me some warnings on the
    // default target time not being enough.
    Criterion::default().measurement_time(Duration::from_secs(25))
}

#[criterion(custom_criterion())]
fn benchmark_flat(c: &mut Criterion) {
    let mut g = c.benchmark_group("compare");
    let repo_path = new_repo_path();
    let repo = clone_repo(repo_path.path());

    g.bench_function("original", |b| {
        b.iter_batched(
            || produce_edits(&repo),
            bench_original,
            BatchSize::SmallInput,
        );
    });
    g.bench_function("modified", |b| {
        b.iter_batched(
            || produce_edits(&repo),
            bench_modified,
            BatchSize::SmallInput,
        );
    });

    g.finish();
}
