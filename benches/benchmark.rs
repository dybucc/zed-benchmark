#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use criterion::{BatchSize, Criterion};
use criterion_macro::criterion;
use zed_benchmark::{bench_modified, bench_original, produce_edits};

#[criterion]
fn benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("compare");

    g.bench_function("original", |b| {
        b.iter_batched(produce_edits, bench_original, BatchSize::SmallInput);
    });
    g.bench_function("modified", |b| {
        b.iter_batched(produce_edits, bench_modified, BatchSize::SmallInput);
    });

    g.finish();
}
