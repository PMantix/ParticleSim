# EIS DOE results

## Layout

- `sin/doe-NNN_*.log` — historical DOE runs (commits before 6c40728) using
  `I(t) = A·sin(ωt)`. The integrated charge under sin is `A·(1−cos(ωt))/ω`,
  always ≥ 0, which biases the cell to a permanently positively-charged
  state. This produces:
    - asymmetric V_cell envelopes
    - persistent small negative Re(Z) at HF
    - artifactual +Im(Z) at extreme LF (cell pinned at saturation)
    - "saturation breakdown" with |Z| collapse + phase past −90° at high amp

  These results remain valid for trend analysis but the absolute Z values
  cannot be directly compared to a real cell — the bias inflates Re(Z) and
  inverts its sign in the saturation regime.

- `doe-cos-NNN_*.log` (top level) — DOE runs after commit 6c40728 using
  `I(t) = A·cos(ωt)`. Integrated charge swings symmetrically between −A/ω
  and +A/ω, so the cell oscillates around true equilibrium. These are the
  results to use for Phase 1.3 Nyquist plots and Phase 5 amplitude scans.

## Companion data

- Time-series CSVs from sin runs: `eis_timeseries/sin/`
- Deep-dive single-case captures: `doe_results/eis_single_case_deep_*_sin/`
  vs `doe_results/eis_single_case_deep_*_cos/`

## Switching back to sin

Don't. The sin convention is mathematically self-consistent but produces a
biased steady state that no longer corresponds to "the cell at equilibrium
with a small AC perturbation" — the prerequisite of linear EIS theory.
