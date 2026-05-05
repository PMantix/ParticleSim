// simulation/morphology.rs
//
// Phase 4 morphology metrics. Quantifies the geometric state of the
// Li-metal / electrolyte interface so we can correlate it with the
// observed amplitude→impedance behaviour later in the study.
//
// Design per docs/EIS_AMPLITUDE_STUDY_PLAN.md Phase 4.1.
//
// Status: SCAFFOLDING.
//   - `interface_roughness_rms_angstroms` is implemented and exercised in
//     a unit test against the flat validation scenario (expect < 5 Å).
//   - The other three metrics are stubbed with sentinel values; the
//     stub returns and inline TODOs document what they should do.
//   - There is no CSV / GUI integration yet (Phase 4.2). The metrics
//     are pure functions on `&[Body]` so they can be wired into either
//     when the time comes.

use crate::body::{Body, Species};

/// Snapshot of Li-metal interface geometry at a single instant.
///
/// All values are reported in simulator units (Å, particles, ratios).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MorphologyMetrics {
    /// Arc length of the Li-metal / electrolyte boundary normalized by
    /// lateral domain extent. Flat reference electrode = 1.0; rough/mossy = >> 1.0.
    ///
    /// Implementation plan: marching squares on a binary Li-metal occupancy
    /// field at chosen grid resolution (5 Å suggested per the plan).
    pub interface_arc_length_per_unit_lateral: f32,

    /// RMS deviation of the Li-metal frontier from its mean position.
    /// For vertical foils (our validation scenario), this is the RMS of
    /// per-y-bin extreme-x coordinates.
    pub interface_roughness_rms_angstroms: f32,

    /// Fraction of LithiumMetal particles disconnected from the percolating
    /// cluster touching the foil. Computed via connected-component analysis
    /// on a particle-proximity graph with a cutoff radius derived from
    /// species data.
    pub dead_li_fraction: f32,

    /// Count of LithiumMetal particles within one neighbor radius of an
    /// electrolyte-species particle (proxy for active surface area).
    pub accessible_surface_atoms: u32,
}

/// Compute all four morphology metrics from the current particle population.
///
/// Pure function over `bodies` — no global state, no time dependence.
/// Cheap enough to call once per N frames (default N = 1000) without
/// significant simulation slowdown.
pub fn compute_morphology_metrics(bodies: &[Body]) -> MorphologyMetrics {
    MorphologyMetrics {
        interface_arc_length_per_unit_lateral: stub_arc_length(bodies),
        interface_roughness_rms_angstroms: roughness_rms_angstroms(bodies),
        dead_li_fraction: stub_dead_li_fraction(bodies),
        accessible_surface_atoms: accessible_surface_atoms(bodies),
    }
}

/// Cutoff multiplier for the accessibility predicate. Centers within
/// `(r_self + r_other) * ACCESSIBLE_CONTACT_FACTOR` are "in contact".
pub const ACCESSIBLE_CONTACT_FACTOR: f32 = 1.3;

/// True if `s` is a *liquid* electrolyte species (ions or solvent molecules).
/// Excludes solid electrolytes (LLZO/LLZT/S40B) and SEI — those represent
/// passivation, not the bulk electrolyte the metal is "exposed" to.
pub fn is_liquid_electrolyte(s: Species) -> bool {
    matches!(
        s,
        Species::LithiumIon
            | Species::ElectrolyteAnion
            | Species::EC
            | Species::DMC
            | Species::VC
            | Species::FEC
            | Species::EMC
    )
}

/// True iff `li` is a LithiumMetal body that has at least one liquid-electrolyte
/// neighbor within `(r_self + r_other) * ACCESSIBLE_CONTACT_FACTOR`. Returns
/// false for non-LithiumMetal species. Naive O(N) over `bodies`; intended for
/// per-particle visualization, not hot-path use.
pub fn is_li_metal_accessible(li: &Body, bodies: &[Body]) -> bool {
    if li.species != Species::LithiumMetal {
        return false;
    }
    bodies.iter().any(|e| {
        is_liquid_electrolyte(e.species) && {
            let cutoff = (li.radius + e.radius) * ACCESSIBLE_CONTACT_FACTOR;
            (li.pos - e.pos).mag_sq() < cutoff * cutoff
        }
    })
}

// ---------------------------------------------------------------------------
// Implemented metric: interface roughness RMS
// ---------------------------------------------------------------------------

/// Compute RMS deviation of the Li-metal frontier from its mean per foil
/// group, averaged across foil groups.
///
/// Algorithm:
/// 1. Filter bodies to Li-metal / FoilMetal species.
/// 2. Split into two foil groups by the sign of `pos.x` (matches the
///    validation scenario's left/right foil convention; will need
///    generalization for >2 foils or non-x-aligned cells).
/// 3. For each group, bin bodies by `y` coordinate (1 Å bins).
/// 4. For each bin, take the extreme-x coordinate (rightmost for the
///    left foil, leftmost for the right foil — i.e. the frontier facing
///    the electrolyte).
/// 5. Compute RMS of (frontier_x − mean_frontier_x) within the group.
/// 6. Average across the two groups.
///
/// A perfectly flat foil yields 0.0; a fully mossy/dendritic surface
/// yields tens of Å.
fn roughness_rms_angstroms(bodies: &[Body]) -> f32 {
    const Y_BIN: f32 = 1.0; // Å

    let metal: Vec<&Body> = bodies
        .iter()
        .filter(|b| matches!(b.species, Species::LithiumMetal | Species::FoilMetal))
        .collect();
    if metal.is_empty() {
        return 0.0;
    }

    let mut left_max: std::collections::HashMap<i32, f32> = Default::default();
    let mut right_min: std::collections::HashMap<i32, f32> = Default::default();
    for b in &metal {
        let bin = (b.pos.y / Y_BIN).floor() as i32;
        if b.pos.x < 0.0 {
            // Left foil: track rightmost frontier.
            left_max
                .entry(bin)
                .and_modify(|x| {
                    if b.pos.x > *x {
                        *x = b.pos.x;
                    }
                })
                .or_insert(b.pos.x);
        } else {
            // Right foil: track leftmost frontier.
            right_min
                .entry(bin)
                .and_modify(|x| {
                    if b.pos.x < *x {
                        *x = b.pos.x;
                    }
                })
                .or_insert(b.pos.x);
        }
    }

    let rms = |frontiers: &std::collections::HashMap<i32, f32>| -> Option<f32> {
        if frontiers.len() < 2 {
            return None;
        }
        let mean: f32 = frontiers.values().copied().sum::<f32>() / frontiers.len() as f32;
        let var: f32 =
            frontiers.values().map(|x| (x - mean).powi(2)).sum::<f32>() / frontiers.len() as f32;
        Some(var.sqrt())
    };

    match (rms(&left_max), rms(&right_min)) {
        (Some(l), Some(r)) => 0.5 * (l + r),
        (Some(l), None) => l,
        (None, Some(r)) => r,
        (None, None) => 0.0,
    }
}

// ---------------------------------------------------------------------------
// Stubs — to implement in Phase 4 follow-up
// ---------------------------------------------------------------------------

fn stub_arc_length(_bodies: &[Body]) -> f32 {
    // TODO Phase 4: marching squares on Li-metal occupancy grid.
    // Return 1.0 (flat reference) until implemented so callers don't
    // mistake an unimplemented stub for a "perfectly flat" measurement.
    f32::NAN
}

fn stub_dead_li_fraction(_bodies: &[Body]) -> f32 {
    // TODO Phase 4: connected-component analysis on a particle-proximity
    // graph with cutoff `2.5 × Li_metal_radius`. Use existing
    // `cell_list.rs:find_neighbors_within` rather than rebuilding.
    f32::NAN
}

/// Count LithiumMetal particles that have at least one *liquid electrolyte*
/// neighbor (ion or solvent molecule) within
/// `(r_self + r_other) * ACCESSIBLE_CONTACT_FACTOR`.
///
/// Solid-electrolyte and SEI species don't count: they represent passivation,
/// not the "exposed-to-electrolyte" surface this metric tries to measure.
///
/// Naive O(N_li × N_electrolyte). For the validation cell (~10³ Li metal,
/// ~10⁴ electrolyte) that's ~10⁷ comparisons — well under the per-frame cost
/// of one collision pass. If it ever becomes hot, switch to `cell_list`.
fn accessible_surface_atoms(bodies: &[Body]) -> u32 {
    bodies
        .iter()
        .filter(|b| b.species == Species::LithiumMetal && is_li_metal_accessible(b, bodies))
        .count() as u32
}

#[cfg(all(test, feature = "unit_tests"))]
mod tests {
    use super::*;
    use crate::body::Body;
    use ultraviolet::Vec2;

    /// Helper: build a flat foil column at fixed x with N bodies stacked vertically.
    fn flat_foil_column(x: f32, n: usize, species: Species) -> Vec<Body> {
        (0..n)
            .map(|i| {
                let y = -50.0 + (i as f32) * 2.0;
                let mut b = Body::new(
                    Vec2::new(x, y),
                    Vec2::zero(),
                    species.mass(),
                    species.radius(),
                    0.0,
                    species,
                );
                b.id = (i + 1) as u64;
                b
            })
            .collect()
    }

    #[test]
    fn roughness_is_zero_for_flat_foils() {
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        bodies.extend(flat_foil_column(150.0, 50, Species::FoilMetal));
        let m = compute_morphology_metrics(&bodies);
        assert!(
            m.interface_roughness_rms_angstroms < 0.5,
            "perfectly flat foil should give roughness < 0.5 Å, got {}",
            m.interface_roughness_rms_angstroms
        );
    }

    /// Helper: build a single column of arbitrary species at fixed x with offset y.
    fn column(x: f32, n: usize, y0: f32, dy: f32, species: Species) -> Vec<Body> {
        (0..n)
            .map(|i| {
                let y = y0 + (i as f32) * dy;
                let mut b = Body::new(
                    Vec2::new(x, y),
                    Vec2::zero(),
                    species.mass(),
                    species.radius(),
                    0.0,
                    species,
                );
                b.id = (10_000 + i) as u64;
                b
            })
            .collect()
    }

    /// Cutoff for the (Li-metal, EC) pair under CONTACT_FACTOR=1.3.
    /// Used to position electrolyte particles deterministically just inside
    /// or just outside reach.
    const LI_EC_CUTOFF: f32 = (1.52 + 2.5) * 1.3; // ≈ 5.226 Å

    #[test]
    fn accessible_surface_zero_with_no_electrolyte() {
        let bodies = flat_foil_column(-150.0, 50, Species::LithiumMetal);
        let m = compute_morphology_metrics(&bodies);
        assert_eq!(m.accessible_surface_atoms, 0);
    }

    #[test]
    fn accessible_surface_counts_only_frontier() {
        // 5 columns of LithiumMetal at x = -150, -148, -146, -144, -142, 50 atoms each.
        // Only the outermost column (x = -142) is within (LI_EC_CUTOFF ≈ 5.23) of EC at x = -138.
        let mut bodies = Vec::new();
        for k in 0..5 {
            let x = -150.0 + (k as f32) * 2.0;
            bodies.extend(flat_foil_column(x, 50, Species::LithiumMetal));
        }
        bodies.extend(column(-138.0, 50, -50.0, 2.0, Species::EC));

        // Sanity: distance from x=-142 to x=-138 is 4.0 (< 5.23, reaches);
        // distance from x=-144 to x=-138 is 6.0 (> 5.23, doesn't).
        assert!(4.0 < LI_EC_CUTOFF && 6.0 > LI_EC_CUTOFF);

        let m = compute_morphology_metrics(&bodies);
        assert_eq!(
            m.accessible_surface_atoms, 50,
            "only the frontier column (50 atoms) should count"
        );
    }

    #[test]
    fn accessible_surface_counts_with_lithium_ion() {
        // Same 5-column geometry but electrolyte = LithiumIon (radius 0.76).
        // LI_LI+ cutoff = (1.52 + 0.76) * 1.3 ≈ 2.96. Place ions at x = -140 (2 Å from frontier).
        let mut bodies = Vec::new();
        for k in 0..5 {
            let x = -150.0 + (k as f32) * 2.0;
            bodies.extend(flat_foil_column(x, 50, Species::LithiumMetal));
        }
        bodies.extend(column(-140.0, 50, -50.0, 2.0, Species::LithiumIon));

        let m = compute_morphology_metrics(&bodies);
        assert_eq!(
            m.accessible_surface_atoms, 50,
            "frontier should also be detected via LithiumIon neighbors"
        );
    }

    #[test]
    fn accessible_surface_grows_with_protrusions() {
        // Baseline: 5 columns + EC frontier at x=-138 → count = 50.
        let mut bodies = Vec::new();
        for k in 0..5 {
            let x = -150.0 + (k as f32) * 2.0;
            bodies.extend(flat_foil_column(x, 50, Species::LithiumMetal));
        }
        bodies.extend(column(-138.0, 50, -50.0, 2.0, Species::EC));
        let baseline = compute_morphology_metrics(&bodies).accessible_surface_atoms;
        assert_eq!(baseline, 50);

        // Add 5 dendrite Li atoms at x=-140 (2 Å further toward EC at -138, distance 2 < 5.23).
        // These are new Li atoms that ARE accessible.
        for i in 0..5 {
            let mut b = Body::new(
                Vec2::new(-140.0, (i as f32) * 4.0 - 8.0),
                Vec2::zero(),
                Species::LithiumMetal.mass(),
                Species::LithiumMetal.radius(),
                0.0,
                Species::LithiumMetal,
            );
            b.id = (20_000 + i) as u64;
            bodies.push(b);
        }
        let with_dendrites = compute_morphology_metrics(&bodies).accessible_surface_atoms;
        assert!(
            with_dendrites > baseline,
            "protrusions should increase accessible count: {} -> {}",
            baseline,
            with_dendrites
        );
    }

    #[test]
    fn accessible_surface_counts_dead_li_island() {
        // Isolated 10-atom Li chain in EC bath. Each Li sandwiched by ECs above/below.
        // Li at (4i, 0) for i=0..9. EC at (4i, ±3) for i=0..9 — distance 3 < 5.23 ✓.
        let mut bodies = Vec::new();
        for i in 0..10 {
            let mut b = Body::new(
                Vec2::new((i as f32) * 4.0, 0.0),
                Vec2::zero(),
                Species::LithiumMetal.mass(),
                Species::LithiumMetal.radius(),
                0.0,
                Species::LithiumMetal,
            );
            b.id = (i + 1) as u64;
            bodies.push(b);
        }
        let mut next_id = 30_000u64;
        for i in 0..10 {
            for dy in [-3.0, 3.0] {
                let mut b = Body::new(
                    Vec2::new((i as f32) * 4.0, dy),
                    Vec2::zero(),
                    Species::EC.mass(),
                    Species::EC.radius(),
                    0.0,
                    Species::EC,
                );
                b.id = next_id;
                next_id += 1;
                bodies.push(b);
            }
        }
        let m = compute_morphology_metrics(&bodies);
        assert_eq!(
            m.accessible_surface_atoms, 10,
            "all 10 atoms in an isolated Li island surrounded by EC should count"
        );
    }

    #[test]
    fn roughness_grows_with_perturbation() {
        // Same flat foils, but with one column body bumped outward 5 Å.
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        bodies.extend(flat_foil_column(150.0, 50, Species::FoilMetal));
        let baseline = compute_morphology_metrics(&bodies);

        bodies[10].pos.x += 5.0; // bump one body of left foil rightward
        let perturbed = compute_morphology_metrics(&bodies);

        assert!(
            perturbed.interface_roughness_rms_angstroms > baseline.interface_roughness_rms_angstroms,
            "perturbation should increase roughness ({} -> {})",
            baseline.interface_roughness_rms_angstroms,
            perturbed.interface_roughness_rms_angstroms
        );
    }
}
