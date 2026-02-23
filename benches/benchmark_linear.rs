#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use std::time::Duration;

use criterion::{BatchSize, Criterion, SamplingMode};
use criterion_macro::criterion;
use zed_benchmark::{bench_modified, bench_original};

mod benchmark_utils;

pub use benchmark_utils::{clone_repo, new_repo_path, produce_edits};

fn custom_criterion() -> Criterion {
    // Had to enable this after the first few runs got me some warnings on the
    // default target time not being enough. The sample size is smaller because
    // this `Criterion` object is used with the benchmark that uses linear
    // sampling instead; With the default 100 samples, it would take 19-20 min.
    Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .sample_size(10)
}

#[criterion(custom_criterion())]
fn benchmark_linear(c: &mut Criterion) {
    let mut g = c.benchmark_group("compare");
    let repo_path = new_repo_path();
    let repo = clone_repo(repo_path.path());

    // This produces the regression chart, which makes it more visually obvious
    // whether the benchmark underwent significant noise or if it was relatively
    // good.
    g.sampling_mode(SamplingMode::Linear);

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
