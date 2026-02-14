use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use lsp_types::Edit;
use zed_benchmark::{bench_modified, bench_original};

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

criterion_group!(benches, benchmark);
criterion_main!(benches);

fn produce_edits() -> Vec<Vec<(Edit, bool)>> {
    todo!()
}
