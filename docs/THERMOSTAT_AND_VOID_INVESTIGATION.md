# Thermostat & Void-Formation Investigation Plan

**Status:** Open. Deferred from the EIS amplitude-study work (`feature/eis-amplitude-study`) on 2026-05-01. To be picked up in a separate session when bandwidth allows.

**Goal:** Determine why the bulk-electrolyte scenario (`measurement_configs/bulk_electrolyte.toml`) shows large seed-to-seed variability in steady-state liquid temperature, and whether the temperature drift is downstream of a structural problem (void formation / clustering) rooted in the inter-particle force model.

**Why it matters:** Every EIS measurement that follows depends on a well-defined operating temperature. Today, T_liquid varies from 222 K to 295 K across seeds at a 295 K target — a 30 % range that translates into 4× variability in measured D_Li⁺. If this is a force-model issue, fixing it tightens every downstream measurement. If it's intrinsic to the simulator's regime, knowing that lets us design measurements around it.

---

## Observed symptoms

Running `measure_diffusivity` with the default 50 ks equilibration + 200 ks measurement on `measurement_configs/bulk_electrolyte.toml` produces seed-dependent steady-state temperatures and diffusion coefficients far outside reproducibility:

| Seed | Final T after 50 ks equilibration | D_Li⁺ (×10⁻² Å²/fs) | Notes |
|---|---|---|---|
| 0xC0FFEE | ~295 K | 6.95 | Clean reference; T fluctuates ±5 K during measurement |
| 0xDEADBEEF | 283 K | 6.54 | Cold trough at ~120 ks |
| (run 3 seed) | 273 K | 3.64 | ⟨r²⟩ briefly *decreases* (wall-clamping artifact) |
| (run 4 seed) | **222 K** | 1.86 | Pathological; T crashes below DMC m.p. |
| (run 5 seed) | 295 K → drifts cold | 5.49 | Stable until ~110 ks then cold drift |

Plots of all five runs are at `images/bulk_electrolyte_diffusivity/msd_bulk_electrolyte_run{2..5}.png`. The Python source is at `images/bulk_electrolyte_diffusivity/msd_plot_run{2..5}.py` (uncommitted, gitignored — data is hand-pasted into numpy arrays).

The thermostat (`src/simulation/thermal.rs::apply_thermostat`) fires every step (`thermostat_interval_fs = 1.0` < `dt = 5.0`), and applies Maxwell-Boltzmann rescaling to liquid species (Li⁺, anion, EC, DMC) with COM drift removed. **Despite firing every step, it does not hold the target temperature.**

In the GUI (`cargo run --release --bin particle_sim -- --scenario measurement_configs/bulk_electrolyte.toml`), the user observed that the EC/DMC organize into a 2D quasi-crystalline lattice within ~20–25 ks, after which **voids and locally over-concentrated regions** are visible. The user's hypothesis is that these structural defects are the *cause* of the thermal drift, not a consequence of it.

---

## Working hypothesis

The repulsive part of the inter-particle force model is too weak or doesn't scale steeply enough at short range, allowing particles to:

1. **Approach too closely** in attractive regions (Coulomb-attractive species pairs, e.g. Li⁺ and PF6⁻; or polar-solvent pairing). This converts kinetic energy to potential energy faster than the thermostat re-injects it on the next rescale, depleting translational KE → apparent cooling.
2. **Form persistent clusters and voids** because the entropic cost of unmixing is not adequately resisted by short-range repulsion. Once formed, these structures couple to slow modes that the thermostat cannot reach.
3. **Concentrate KE inhomogeneously.** The thermostat rescales using a single COM-corrected liquid temperature, but local hot spots (collisions in dense regions) and cold spots (vibrational cages around clusters) coexist. Rescaling with a global factor doesn't fix this.

If correct, the thermal drift would be a *symptom* of structural pathology in the force model. Fixing the force model should restore both spatial uniformity and thermal stability simultaneously.

**Alternative hypotheses to keep in mind:**

- The thermostat's COM-removal is broken (e.g., scales velocities but doesn't subtract COM consistently with how `compute_liquid_temperature` does), causing apparent T to drift while real KE is held constant. Test: log internal-thermostat-T and `compute_liquid_temperature` at the same instant; they should agree.
- Berendsen-style scaling with a time constant that's too long (rescales partially, not fully). Read the rescaling formula in `apply_thermostat` to check.
- The `enable_out_of_plane = false` setting isn't propagated to all force terms (Z-direction modes still draining energy).
- Bound-electron KE (the `Body::electrons[].vel` reservoir) drains energy from the translational pool. Thermostat reaches translational only.

---

## Reproduction

```sh
# Branch
git checkout feature/eis-amplitude-study

# Reference run (seed 0xC0FFEE) — should produce T_eq ≈ 295 K, D ≈ 7e-2 Å²/fs
cargo run --release --bin measure_diffusivity -- \
    --scenario measurement_configs/bulk_electrolyte.toml \
    --equilibrate-fs 50000 --log-every-fs 5000 \
    --measure-fs 50000 --sample-every-fs 1000

# Pathological run (some seed produces T_eq ≈ 222 K)
# Try a few seeds to find one that exhibits the cold-crash behavior
cargo run --release --bin measure_diffusivity -- \
    --scenario measurement_configs/bulk_electrolyte.toml \
    --equilibrate-fs 50000 --log-every-fs 5000 \
    --measure-fs 50000 --sample-every-fs 1000 \
    --seed 0x12345678   # try several values

# GUI to watch void formation visually
cargo run --release --bin particle_sim -- \
    --scenario measurement_configs/bulk_electrolyte.toml
```

The user has confirmed in the GUI that voids and clusters form within ~20–25 ks of starting from random initial placement. This is the structural feature to investigate.

---

## Diagnostic experiments

In rough order of cheapness × discrimination power. Run the cheap ones first; they may close the question on their own.

### D1. Visual confirmation + screenshot pipeline

Use the GUI with the bulk scenario, take screenshots at t = 0, 5, 15, 30, 50 ks. Confirm: (a) initial random placement, (b) caging into pseudo-lattice, (c) void/cluster emergence, (d) steady-state inhomogeneity.

If voids do not appear in *every* run that has thermal drift, the structural hypothesis is incomplete — drift may have a separate cause.

### D2. Energy-component logging

Add an instrumentation hook (gated behind a new feature flag `energy_debug` matching the project's existing `thermostat_debug` pattern) that logs per-step (or every-N-step):

- Total KE (all species)
- Translational liquid KE (the quantity the thermostat targets)
- Bound-electron KE (`Σ 0.5 m_e |electrons[i].vel|²`)
- LJ potential energy
- Coulomb potential energy

Plot all five over a full run. If total energy is conserved while components shift, the thermostat is working but redistributing energy oddly. If total energy drifts (despite the thermostat), there's leakage or creation in a force kernel.

**Files:** `src/simulation/forces.rs` (PE accumulation hooks), `src/simulation/simulation.rs::step` (call site), `src/config.rs` (feature flag).

### D3. Pair correlation function g(r)

Sample g(r) at the end of equilibration (t = 50 ks) for each species pair (Li⁺-Li⁺, EC-EC, Li⁺-PF6⁻, etc.). A liquid shows a broad first peak and decay to 1.0 at long range. A glass shows multiple peaks. A void-forming system shows g(r) > 1 at short separations + g(r) < 1 at intermediate separations + recovery at long range.

Compare g(r) for the "good" run (seed C0FFEE, T ≈ 295 K) and the "pathological" run (seed → T ≈ 222 K). If g(r) differs *only in the short-range repulsive zone*, that confirms the force-model hypothesis.

Implementation: a one-shot Rust binary or Python post-processing script. Probably easier as a Python script that ingests body positions exported from the simulator at t = 50 ks.

### D4. Close-contact pair count

Count the number of body pairs within `0.5 × (r_i + r_j)` (i.e., severely overlapping). In a healthy MD system this should be zero (or very low for soft particles). Count over time during the run. If non-zero or growing, the repulsive force is failing to enforce the hard-core radius.

**Files:** add a helper in `src/diagnostics/`, run it from the same per-step hook as D2.

### D5. Force-distance verification

For each species pair, sample the force magnitude as a function of separation distance over `[0.1·σ, 5·σ]` where σ is the LJ sigma. Plot. Compare with expected:

- LJ: `F(r) ∝ σ^12 / r^13 - σ^6 / r^7`
- Coulomb: `F(r) ∝ q₁ q₂ / r²` (with whatever softcore is configured)

If observed F(r) is shallower than expected at short range, that's the smoking gun.

**Files:** `src/simulation/forces.rs` (or equivalent — look for `lj_force`, `coulomb_force`, `compute_forces`). Check the `e_sq` or softening parameter in `quadtree.rs:374` for the Coulomb side; look for any clamps or short-range cutoffs in the LJ side.

### D6. Thermostat formula audit

Read `src/simulation/thermal.rs::apply_thermostat` line-by-line. Verify:

- It computes T using the same liquid-species set and same COM removal as `compute_liquid_temperature` in `src/simulation/utils.rs`.
- Velocity rescaling uses the correct factor: for a target T_target, `scale = sqrt(T_target / T_current)`, not Berendsen-with-time-constant unless that's intended.
- The rescale is applied only to liquid species (per CLAUDE.md), and metals are correctly excluded.
- COM velocity is preserved across the rescale (rescaling thermal velocities, not the COM).

Look for off-by-one or unit errors. Compare against a textbook (e.g., Frenkel & Smit, "Understanding Molecular Simulation").

---

## Files to read first

- `src/simulation/thermal.rs` — the thermostat implementation (~200 lines, focused)
- `src/simulation/utils.rs` lines 70–105 — `compute_liquid_temperature` and `initialize_liquid_velocities_to_temperature`
- `src/simulation/forces.rs` — LJ + polar forces
- `src/simulation/collision.rs` — soft-collision system (separate from forces; may be conflicting)
- `src/quadtree/quadtree.rs` line 374 — Coulomb softcore
- `src/config.rs` lines 155–175 — potential-related constants
- `src/species.rs` — per-species LJ parameters
- `init_config.toml` — to compare with bulk-only TOML
- `docs/EIS_AMPLITUDE_STUDY_PLAN.md` — the parent project this blocks

---

## Acceptance criteria

The investigation can close as either *fixed* or *characterized-and-accepted*:

**Fixed:**
- T_liquid stays within ±3 K of the 295 K target across at least 5 seeds during a 200 ks run.
- D_Li⁺ varies <15 % across the same 5 seeds.
- No persistent voids or clusters visible in the GUI after 50 ks.
- Existing test suite still passes (`cargo test --features unit_tests`).
- Whatever code change implements the fix is committed with a clear explanation of root cause.

**Characterized-and-accepted:**
- Diagnostics D1–D6 (or whichever subset suffices) have produced a defensible explanation for *why* the simulator can't hold isothermal conditions in this scenario.
- The amplitude-study spec (`docs/EIS_AMPLITUDE_STUDY_PLAN.md`) is updated with a clear note in the similitude / open-questions section: simulator runs in a thermally-floppy state with seed-dependent steady-state T; amplitude comparisons must average over multiple seeds at each amplitude.

---

## Out of scope for this investigation

- **Implementing periodic boundary conditions.** The wall-clamping artifact (visible as ⟨r²⟩ briefly *decreasing* in some runs) is a separate issue; PBC is a much larger refactor and not required to answer the thermostat question.
- **Re-tuning particle counts, density, or molarity** of the bulk-electrolyte scenario. The current composition matches the project's default scenario; comparing apples-to-apples requires keeping it fixed.
- **Replacing the LJ + polar force model with a different potential (e.g., Buckingham, Morse).** That's a modeling decision beyond the scope of "fix the thermostat."
- **Changing `enable_out_of_plane` or other top-level switches** without first verifying they're propagated correctly. Flag any that aren't, but don't make them the focus.
- **Adding new species.** Limit changes to the species that already exist in the bulk-electrolyte TOML (Li⁺, PF6⁻, EC, DMC).

---

## How to hand off back to the EIS work

When the investigation closes:

1. Update the **Open Questions** section of `docs/EIS_AMPLITUDE_STUDY_PLAN.md` with the finding — either a quantitative thermostat tolerance (if fixed) or a characterized noise floor (if accepted).
2. If the simulator was modified, re-run the Phase 0a reference measurement (`--seed 0xC0FFEE`, 200 ks) and update `docs/eis_similitude.md` with the new D_Li⁺ value.
3. If thermal stability is now reproducible across seeds, tighten the Phase 1 acceptance criterion in the EIS plan from `min(fit_r2_v, fit_r2_i) > 0.95` toward something stricter (e.g., 0.98).
4. Either way, leave a one-line breadcrumb at the top of *this* file pointing to the resolving commit / PR.
