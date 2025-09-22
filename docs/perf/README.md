# Performance Harness

The performance harness automates the build and runtime measurements required for
profiling Link Time Optimization (LTO) and codegen-unit combinations. It sweeps
the `{lto ∈ [false, thin, fat]} × {codegen-units ∈ [1, 4, 8, 16]}` matrix and
captures comparable metrics for each configuration.

## Running the harness

```sh
cargo run --bin perf_harness
```

The harness performs the following steps for every LTO/codegen-unit pair:

1. Builds the `particle_sim` release binary with `cargo build --timings=json`
   while forcing the desired `-C lto` and `-C codegen-units` flags via
   `RUSTFLAGS`. Each build uses an isolated `target/perf/lto-<lto>-cgu-<N>`
   directory to guarantee clean measurements.
2. Records build wall-clock time, parses the `cargo-timings` JSON report, and
   captures the resulting binary size.
3. Compiles the lightweight `runtime_probe` benchmark binary with the same
   compiler settings and executes it to measure simulation throughput.

> **Note:** Running the full matrix performs 12 clean release builds. The first
> invocation may take several minutes depending on host hardware. Subsequent
> runs benefit from the cached source downloads and linked dependencies but
> still rebuild the crate for each configuration.

## Output artifacts

The harness writes two CSV files under `docs/perf/`:

- `lto-build-matrix.csv` – build wall-clock time, Cargo timing totals, and final
  binary size for each configuration.
- `runtime-benchmarks.csv` – runtime metrics from the `runtime_probe` benchmark
  including total wall-clock time, mean step duration, and slowdown factor.

Each run overwrites the CSV files, making it easy to commit refreshed data
alongside code changes. The CSV headers are stable, so they can be imported into
spreadsheets or plotting tools directly.

## Related tooling

- `scripts/runtime_probe.rs` – small benchmark binary that steps a canonical
  simulation workload and emits JSON with timing data.
- `Cargo.toml` profiles – use `cargo build --profile release-thin` or `cargo
  build --profile release-fast` for ad hoc comparisons without running the full
  harness.
