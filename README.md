# Zed editor LSP request benchmark

The benchmark tests out some changes to the way Zed processes pending `lsp_types::Edit`s returned
from the language server. In the current Zed worktree, it traverses the whole array while performing
lookups in linear time within two other contiguous-memory-based containers. In this benchmark, we
compare both this method and a more time-efficient, hash-based traversal.

The results turned out to not be significant enough to warrant the change from a sequential search
in overarching containers to an O(1) lookup in a hash table.

The resuslts presented in the GitHub pages associated with the repo were performed on a M4 Apple
Silicon machine, sitting idly without any `homebrew` services running, and following the advice in
the [standard library developer's guide].

## Methodology

Because of the nature of edititions that the language server issues back to the client, emulating
the same types of editions required also emulating changes to some form of sorce code.

With that premise, the `benchmark_utils` module contains an entry-point function `produce_edits()`
that fetches, among the 100 most recent commits in the Zed repo, one random revision, and diffs that
with the latest commit in `HEAD` on the Zed GitHub remote. Then it parses only the changes that were
deemed to be modifications, skipping removals, file additions, and changes from regular files to
symbolic links. Then it fetches the specific blobs (non-executables) that were produced from that
second IR step, and diffs their actual contents. The result of that diff is then parsed into
`lsp_types::Edit`s. Because the benchmark tries to emulate the same production stress conditions as
those seen in an actual editing session in Zed, the choice for the 100 most recent commits provides
the benchmark framework with plenty of sample material to source from, as each iteration Criterion
runs uses a different run of the `produce_edits()` routine, which allows choosing
semi-deterministically among all commits in the `main` codebase.

## Running the benchmark

The benchmark uses the Criterion framework in Rust, so reading the relevant documentation both at
the [Criterion API docs], and in the [Criterion guide] is recommended.

Beyond that, simply running `cargo bench` at the command line, and optionally providing arguments
for any one of the two benchmarks should be enough.

The benchmark uses bootstrap sampling, and to more easily visualize whether there was a performance
regression or not, it can run with flat or linear sampling. In Criterion, the former will produce a
chart with the runnning time of each iteration as a point in 2-dimensional space, while the latter
will produce a regression chart where the running time of all iterations is plot against an
approximate the framework provides to easily check whether there was a lot of system noise involved
in the measurements.

See also the comments left in the source code for each benchmark for information on the choices of
Criterion objects, including sample tweaking and measurement time.

[standard library developer's guide]: https://std-dev-guide.rust-lang.org/development/perf-benchmarking.html
[Criterion API docs]: https://docs.rs/criterion/latest/criterion/
[Criterion guide]: https://criterion-rs.github.io/book/criterion_rs.html
