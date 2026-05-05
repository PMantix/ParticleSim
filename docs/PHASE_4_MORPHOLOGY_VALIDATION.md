# Phase 4 Morphology — Validation Report

**Status:** in-progress (one of three metrics complete and validated).
**Branch:** `feature/eis-amplitude-study`.
**Sources:** `src/simulation/morphology.rs`, `src/bin/morphology_demo.rs`, `scripts/plot_morphology_demo.py`, `images/morphology_validation/`.

This is the validation log for the four morphology metrics defined in `src/simulation/morphology.rs`. Each metric ships with: (a) implementation, (b) unit tests in the existing `morphology` test module, (c) a synthetic-scenario panel set rendered by `morphology_demo` + `plot_morphology_demo.py`, and (d) a section in this report (algorithm, scenario table, figure, judgment, open tunables).

User reviews each metric section here and greenlights or requests adjustment before the next metric proceeds.

## Metric inventory

| # | Metric | Status | Section |
|---|---|---|---|
| roughness | `interface_roughness_rms_angstroms` | landed (pre-existing) | not re-validated here |
| 1 | `interface_arc_length_per_unit_lateral` | stub (`f32::NAN`) | pending |
| 2 | `dead_li_fraction` | stub (`f32::NAN`) | pending |
| 3 | `accessible_surface_atoms` | implemented + validated | below |

---

## Metric #3 — `accessible_surface_atoms`

**Source:** `src/simulation/morphology.rs:accessible_surface_atoms`.

### Algorithm

A LithiumMetal particle is "accessible" iff at least one *liquid electrolyte* particle is within `(r_self + r_other) * ACCESSIBLE_CONTACT_FACTOR` of it. The aggregate metric is the count of accessible LithiumMetal particles.

- *Liquid electrolyte* species: `LithiumIon | ElectrolyteAnion | EC | DMC | VC | FEC | EMC`. Solid electrolytes (`LLZO/LLZT/S40B`) and `SEI` deliberately do **not** count: those represent passivation, not the bulk electrolyte the metal is "exposed to".
- `ACCESSIBLE_CONTACT_FACTOR = 1.3` — neighbor centers within 30% past pure geometric contact. Wide enough to catch first-shell solvation/passivation neighbors, narrow enough that an atom several Å inside the bulk metal is not falsely flagged.
- `FoilMetal` is **not** counted. The metric specifically measures *plated* / *deposited* Li, not the current collector.
- Implementation is a naive O(N_LiMetal × N_electrolyte) double-loop with early break on first hit. For the validation cell (~10³ Li metal, ~10⁴ electrolyte) that is ~10⁷ comparisons — well under the per-frame cost of one collision pass. Switch to `cell_list` if it ever becomes hot.

### Unit tests

Run with `cargo test --features unit_tests --release morphology`. All seven morphology tests pass; the four new tests added for #3 are:

| Test | Asserts |
|---|---|
| `accessible_surface_zero_with_no_electrolyte` | flat foil + no electrolyte → 0 |
| `accessible_surface_counts_only_frontier` | 5-column block + EC frontier → only outermost column counts (50) |
| `accessible_surface_counts_with_lithium_ion` | same geometry with Li⁺ in place of EC → still 50 (predicate covers ions) |
| `accessible_surface_grows_with_protrusions` | adding 5 protruding Li atoms strictly increases count |
| `accessible_surface_counts_dead_li_island` | isolated 10-atom Li chain in EC bath → 10 |

### Synthetic-scenario validation

`morphology_demo --metric accessible_surface_atoms` builds 6 synthetic scenarios, evaluates the metric, and writes JSON + a multi-panel PNG.

![accessible_surface_atoms](../images/morphology_validation/accessible_surface_atoms.png)

| Scenario | Description | Expected | Computed | Judgment |
|---|---|---:|---:|---|
| `flat_no_electrolyte` | 5-column LithiumMetal block, no electrolyte | 0 | 0 | PASS |
| `flat_with_electrolyte` | block + EC frontier 4 Å past outermost column | 50 | 50 | PASS |
| `mossy_with_electrolyte` | baseline + 10 protruding Li atoms 2 Å closer to EC | 60 | 60 | PASS |
| `buried_no_reach` | block + EC located far past the cutoff | 0 | 0 | PASS |
| `dead_li_island` | 10-atom Li chain sandwiched between EC layers | 10 | 10 | PASS |
| `realistic_cell` | foils + 30 plated Li + dense EC/DMC/Li⁺/anion bulk | 28 ± 4 | 26 | PASS |

The first five scenarios are deterministic. The sixth (`realistic_cell`) is a stress test on a multi-species mixed body collection at validation-cell density (~2000 electrolyte particles). The exact accessibility count depends on which y-positions of plated Li happen to land near a Li⁺-only neighborhood (Li⁺ has a smaller cutoff because of its small radius); 26 of 30 is within tolerance.

### Real-sim verification

The saved-state files in `saved_state/` (`base_electrolyte.bin.gz`, `HGPU_electrolyte.bin.gz`, `LLZO_electrolyte.bin.gz`) use a `Body` schema that pre-dates the addition of the `lithium_content` field and currently fail to deserialize against the `serde` defaults. They contain bulk-electrolyte-only configurations with no `LithiumMetal`, so a successful load would still return `accessible_surface_atoms = 0`, which doesn't load-bear the predicate.

Verification on a *currently-loadable* real-sim snapshot is therefore not done here. The `realistic_cell` synthetic scenario substitutes by exercising the metric on a multi-species body collection at validation-cell density. Once a Phase 5 (EIS amplitude study) run produces a saved state under the current schema with non-trivial plated Li, this report should be re-run with `morphology_demo --load-state <path>` to add a real-sim panel.

### Tunables (no DOE done — defaults are arbitrary-but-reasonable)

| Constant | Default | Where | Notes |
|---|---|---|---|
| `ACCESSIBLE_CONTACT_FACTOR` | 1.3 | `src/simulation/morphology.rs` | Neighbor centers within 30% past pure geometric contact. Smaller → only direct contact counts; larger → second-shell counts. |
| Electrolyte species set | `LithiumIon \| ElectrolyteAnion \| EC \| DMC \| VC \| FEC \| EMC` | `is_liquid_electrolyte()` | Excludes solid electrolyte and SEI, intentionally. |

If the user requests a different cutoff convention or species set, change the constant and the predicate; tests and report should be re-run.

### Judgment

PASS. All five deterministic synthetic scenarios produce exactly the expected count. The `realistic_cell` stress test produces a sensible non-trivial count within tolerance. Unit tests cover the predicate's edge cases (no-electrolyte, frontier-only, ion vs. solvent neighbor, protrusion sensitivity, isolated-island).

**Greenlight requested before proceeding to Metric #1 (`interface_arc_length_per_unit_lateral`).**

---

## Metric #1 — `interface_arc_length_per_unit_lateral`

**Status:** stub (`f32::NAN`). Will be filled in once #3 is greenlit.

Planned algorithm (from the scaffold's docstring): marching squares on a binary Li-metal occupancy grid; report `total_contour_length / (2 × y_extent)`. Flat reference electrode → 1.0; mossy/dendritic ≫ 1.0.

Expected pre-merge work: a small grid-resolution DOE (mentioned by the user) to characterize how the metric tracks ground truth across grid resolutions of 2 / 5 / 10 Å. The 5 Å default in the scaffold's docstring is a placeholder.

## Metric #2 — `dead_li_fraction`

**Status:** stub (`f32::NAN`). Will be filled in once #1 is greenlit.

Planned algorithm: connected-component analysis on a particle-proximity graph with cutoff `2.5 × Li_metal_radius`. Use existing `cell_list.rs:find_neighbors_within` rather than rebuilding spatial structure. Report fraction of LithiumMetal particles disconnected from the percolating cluster touching the foil.

## How to re-run this report

```bash
# unit tests
cargo test --features unit_tests --release morphology

# synthetic-scenario eval + JSON
cargo run --release --bin morphology_demo -- --metric accessible_surface_atoms

# render PNG
python3 scripts/plot_morphology_demo.py --metric accessible_surface_atoms

# (future) real-sim panel — once a current-schema saved state with Li metal exists:
# cargo run --release --bin morphology_demo -- --metric accessible_surface_atoms \
#   --load-state path/to/saved_state.bin.gz
```

## References

- `src/simulation/morphology.rs` — implementation of all four metrics.
- `src/bin/morphology_demo.rs` — synthetic-scenario harness.
- `scripts/plot_morphology_demo.py` — multi-panel renderer.
- `images/morphology_validation/` — JSON + PNG outputs.
- `docs/EIS_AMPLITUDE_STUDY_PLAN.md` Phase 4 — original metric specification.
- `docs/EIS_DCR_PULSE_PLAN.md` Phase 5 — downstream consumer of these metrics for plating diagnostics.
