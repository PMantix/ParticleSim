// simulation/morphology.rs
//
// Phase 4 morphology metrics. Quantifies the geometric state of the
// Li-metal / electrolyte interface so we can correlate it with the
// observed amplitude→impedance behaviour later in the study.
//
// Design per docs/EIS_AMPLITUDE_STUDY_PLAN.md Phase 4.1.
//
// Status: in-progress.
//   - `interface_roughness_rms_angstroms` is implemented and tested.
//   - `accessible_surface_atoms` (#3) is implemented and tested
//     (see docs/PHASE_4_MORPHOLOGY_VALIDATION.md).
//   - `interface_arc_length_per_unit_lateral` (#1) is implemented as a
//     frontier-trace algorithm (single-valued y → x); does not yet
//     handle overhangs/islands. See report for the limitation note.
//   - `dead_li_fraction` (#2) is implemented via union-find on a
//     metal-proximity graph; naive O(N²) edge construction.
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
        interface_arc_length_per_unit_lateral: interface_arc_length(bodies),
        interface_roughness_rms_angstroms: roughness_rms_angstroms(bodies),
        dead_li_fraction: dead_li_fraction(bodies),
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
// Shared frontier-extraction helper (used by roughness + arc length)
// ---------------------------------------------------------------------------

/// Default Y-bin width for the arc-length metric. The metric depends weakly
/// on this for moderately rough interfaces and more strongly for highly
/// dendritic ones (smaller bins resolve finer features). 5 Å is the
/// scaffold's documented default; see the grid-resolution DOE in
/// `docs/PHASE_4_MORPHOLOGY_VALIDATION.md`.
pub const ARC_LENGTH_DEFAULT_Y_BIN_ANGSTROMS: f32 = 5.0;

/// One frontier point: y-bin center plus extreme-x position of the metal
/// frontier facing the electrolyte at that y.
#[derive(Clone, Copy, Debug)]
pub struct FrontierPoint {
    pub y: f32,
    pub x: f32,
}

/// Extract the per-y-bin frontier for the left (x < 0) and right (x ≥ 0)
/// foil groups. Returns `(left_frontier, right_frontier)`, each sorted by y.
///
/// "Frontier" = the metal coordinate at each y-bin facing the electrolyte —
/// rightmost x for the left foil, leftmost x for the right foil. Builds on
/// the same convention as the roughness metric so the two are directly
/// comparable.
pub fn extract_metal_frontiers(
    bodies: &[Body],
    y_bin: f32,
) -> (Vec<FrontierPoint>, Vec<FrontierPoint>) {
    use std::collections::HashMap;
    let mut left_max: HashMap<i32, f32> = HashMap::new();
    let mut right_min: HashMap<i32, f32> = HashMap::new();

    for b in bodies
        .iter()
        .filter(|b| matches!(b.species, Species::LithiumMetal | Species::FoilMetal))
    {
        let bin = (b.pos.y / y_bin).floor() as i32;
        if b.pos.x < 0.0 {
            left_max
                .entry(bin)
                .and_modify(|x| {
                    if b.pos.x > *x {
                        *x = b.pos.x;
                    }
                })
                .or_insert(b.pos.x);
        } else {
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

    let to_sorted = |m: HashMap<i32, f32>| -> Vec<FrontierPoint> {
        let mut v: Vec<FrontierPoint> = m
            .into_iter()
            .map(|(bin, x)| FrontierPoint {
                y: (bin as f32 + 0.5) * y_bin,
                x,
            })
            .collect();
        v.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
        v
    };
    (to_sorted(left_max), to_sorted(right_min))
}

// ---------------------------------------------------------------------------
// Implemented metric: interface arc length per unit lateral extent
// ---------------------------------------------------------------------------

/// Arc length of the Li-metal frontier per side, normalized by lateral extent,
/// averaged across the two foil groups. Flat foils → 1.0; mossy/dendritic ≫ 1.0.
///
/// Algorithm v1 (frontier-trace):
/// 1. Extract per-y-bin frontier for left/right foil groups via
///    [`extract_metal_frontiers`] using
///    [`ARC_LENGTH_DEFAULT_Y_BIN_ANGSTROMS`] as bin width.
/// 2. For each side, sum segment lengths between consecutive frontier
///    points: `Σ √((Δx)² + (Δy)²)`.
/// 3. Normalize by `(y_max − y_min)` of the side's frontier.
/// 4. Average across sides.
///
/// **Limitation vs true marching squares:** assumes the interface is a
/// single-valued function `y → x`. Overhangs, isolated dendrite tips, and
/// detached islands are collapsed to their extreme-x point per y-bin and
/// the connecting contour between them is ignored. For moderate-roughness
/// regimes (the validation cell + early-cycle plating) this is fine; for
/// late-stage dendritic morphology, switch to true marching squares.
pub fn interface_arc_length(bodies: &[Body]) -> f32 {
    interface_arc_length_with_bin(bodies, ARC_LENGTH_DEFAULT_Y_BIN_ANGSTROMS)
}

/// Configurable-bin variant of [`interface_arc_length`] for resolution
/// sweeps and tuning.
pub fn interface_arc_length_with_bin(bodies: &[Body], y_bin: f32) -> f32 {
    let (left, right) = extract_metal_frontiers(bodies, y_bin);

    let per_side = |frontier: &[FrontierPoint]| -> Option<f32> {
        if frontier.len() < 2 {
            return None;
        }
        let total: f32 = frontier
            .windows(2)
            .map(|w| {
                let dx = w[1].x - w[0].x;
                let dy = w[1].y - w[0].y;
                (dx * dx + dy * dy).sqrt()
            })
            .sum();
        let y_extent = frontier.last().unwrap().y - frontier.first().unwrap().y;
        if y_extent <= 0.0 {
            return None;
        }
        Some(total / y_extent)
    };

    match (per_side(&left), per_side(&right)) {
        (Some(l), Some(r)) => 0.5 * (l + r),
        (Some(l), None) => l,
        (None, Some(r)) => r,
        (None, None) => 0.0,
    }
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
// Implemented metric: dead_li_fraction
// ---------------------------------------------------------------------------

/// Cutoff factor for the metal proximity graph: a pair of metal atoms is
/// considered connected iff their center distance is below
/// `DEAD_LI_CUTOFF_FACTOR * Species::LithiumMetal.radius()`.
///
/// Default 2.5 — about 25% past pure geometric contact (2 r ≈ 3.04 Å), so a
/// thin SEI gap on the order of < 0.7 Å keeps the cluster connected, but a
/// 2 Å gap (typical for fully detached islands) does not.
pub const DEAD_LI_CUTOFF_FACTOR: f32 = 2.5;

/// Iterative union-find with path halving + union-by-rank.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self { parent: (0..n).collect(), rank: vec![0; n] }
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        match self.rank[ra].cmp(&self.rank[rb]) {
            std::cmp::Ordering::Less => self.parent[ra] = rb,
            std::cmp::Ordering::Greater => self.parent[rb] = ra,
            std::cmp::Ordering::Equal => {
                self.parent[rb] = ra;
                self.rank[ra] += 1;
            }
        }
    }
}

/// Per-body classification: `Some(true)` if the body is a LithiumMetal that is
/// disconnected from every FoilMetal-containing component (i.e. "dead Li").
/// `Some(false)` if it is a LithiumMetal connected to a foil. `None` for
/// non-LithiumMetal bodies.
///
/// Returned vector is parallel to `bodies` so callers can join with positions
/// for visualization without re-running the connectivity analysis.
pub fn classify_li_metal_dead(bodies: &[Body]) -> Vec<Option<bool>> {
    let r_li = Species::LithiumMetal.radius();
    let cutoff = DEAD_LI_CUTOFF_FACTOR * r_li;
    let cutoff_sq = cutoff * cutoff;

    // Indices of all metal particles (Li or Foil) in `bodies`.
    let metal_idx: Vec<usize> = bodies
        .iter()
        .enumerate()
        .filter(|(_, b)| matches!(b.species, Species::LithiumMetal | Species::FoilMetal))
        .map(|(i, _)| i)
        .collect();

    let mut result = vec![None; bodies.len()];
    if metal_idx.is_empty() {
        return result;
    }

    let mut uf = UnionFind::new(metal_idx.len());
    for i in 0..metal_idx.len() {
        let bi = &bodies[metal_idx[i]];
        for j in (i + 1)..metal_idx.len() {
            let bj = &bodies[metal_idx[j]];
            if (bi.pos - bj.pos).mag_sq() < cutoff_sq {
                uf.union(i, j);
            }
        }
    }

    // Component roots that touch foil are "alive". Use a boolean mask sized to
    // metal_idx.len() — any root index can be looked up directly.
    let mut root_is_alive: Vec<bool> = vec![false; metal_idx.len()];
    for (i, &body_i) in metal_idx.iter().enumerate() {
        if bodies[body_i].species == Species::FoilMetal {
            let r = uf.find(i);
            root_is_alive[r] = true;
        }
    }

    for (i, &body_i) in metal_idx.iter().enumerate() {
        if bodies[body_i].species == Species::LithiumMetal {
            let r = uf.find(i);
            result[body_i] = Some(!root_is_alive[r]);
        }
    }
    result
}

/// Fraction of LithiumMetal particles disconnected from any FoilMetal
/// percolating cluster. 0.0 = all Li connected to a foil; 1.0 = no Li
/// connected to any foil.
///
/// Edge cases:
/// - No LithiumMetal in `bodies` → 0.0 (no Li to be dead).
/// - LithiumMetal exists but no FoilMetal anywhere → 1.0 (everything dead).
fn dead_li_fraction(bodies: &[Body]) -> f32 {
    let classification = classify_li_metal_dead(bodies);
    let li_flags: Vec<bool> = classification.into_iter().flatten().collect();
    if li_flags.is_empty() {
        return 0.0;
    }
    let n_dead = li_flags.iter().filter(|&&dead| dead).count();
    n_dead as f32 / li_flags.len() as f32
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

    // -------------------------------------------------------------------
    // Tests for interface_arc_length
    // -------------------------------------------------------------------

    #[test]
    fn arc_length_one_for_flat_foils() {
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        bodies.extend(flat_foil_column(150.0, 50, Species::FoilMetal));
        let m = compute_morphology_metrics(&bodies);
        // Frontier is exactly vertical → segments are pure Δy → ratio = 1.0.
        assert!(
            (m.interface_arc_length_per_unit_lateral - 1.0).abs() < 1e-3,
            "flat 2-foil should give 1.0, got {}",
            m.interface_arc_length_per_unit_lateral
        );
    }

    #[test]
    fn arc_length_one_for_single_flat_foil() {
        let bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        let m = compute_morphology_metrics(&bodies);
        assert!(
            (m.interface_arc_length_per_unit_lateral - 1.0).abs() < 1e-3,
            "single flat foil should give 1.0, got {}",
            m.interface_arc_length_per_unit_lateral
        );
    }

    #[test]
    fn arc_length_zero_for_empty_bodies() {
        let bodies: Vec<Body> = Vec::new();
        let m = compute_morphology_metrics(&bodies);
        assert_eq!(m.interface_arc_length_per_unit_lateral, 0.0);
    }

    #[test]
    fn arc_length_grows_with_bump() {
        // Baseline: flat 2-foil → 1.0.
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        bodies.extend(flat_foil_column(150.0, 50, Species::FoilMetal));
        let baseline = compute_morphology_metrics(&bodies).interface_arc_length_per_unit_lateral;

        // Bump one body of left foil rightward by 5 Å (toward electrolyte).
        bodies[10].pos.x += 5.0;
        let perturbed = compute_morphology_metrics(&bodies).interface_arc_length_per_unit_lateral;

        assert!(
            perturbed > baseline,
            "bump should increase arc length ({} -> {})",
            baseline,
            perturbed
        );
        assert!(
            perturbed < 1.5,
            "single 5 Å bump should not blow up the metric ({})",
            perturbed
        );
    }

    #[test]
    fn arc_length_large_for_dendritic_spike() {
        // Flat foil + a long spike protruding 30 Å outward at y=0.
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        // Spike: 5 atoms marching from x=-145 down to x=-120, all at y close to 0.
        for i in 0..6 {
            let mut b = Body::new(
                Vec2::new(-145.0 + (i as f32) * 5.0, 0.0),
                Vec2::zero(),
                Species::FoilMetal.mass(),
                Species::FoilMetal.radius(),
                0.0,
                Species::FoilMetal,
            );
            b.id = (50_000 + i) as u64;
            bodies.push(b);
        }
        let m = compute_morphology_metrics(&bodies);
        assert!(
            m.interface_arc_length_per_unit_lateral > 1.3,
            "30 Å dendritic spike should drive arc length well above 1.0, got {}",
            m.interface_arc_length_per_unit_lateral
        );
    }

    #[test]
    fn arc_length_grows_with_perturbation() {
        // Same flat baseline, then perturb several bodies outward by random
        // amounts ≈ 3 Å. Should give a small but consistent increase.
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        let baseline = compute_morphology_metrics(&bodies).interface_arc_length_per_unit_lateral;

        for i in (0..50).step_by(3) {
            bodies[i].pos.x += 3.0;
        }
        let perturbed = compute_morphology_metrics(&bodies).interface_arc_length_per_unit_lateral;

        assert!(
            perturbed > baseline + 0.05,
            "multiple 3 Å perturbations should give a measurable increase ({} -> {})",
            baseline,
            perturbed
        );
        assert!(
            perturbed < 1.5,
            "moderate perturbations should not push past 1.5 ({})",
            perturbed
        );
    }

    // -------------------------------------------------------------------
    // Tests for dead_li_fraction
    // -------------------------------------------------------------------

    #[test]
    fn dead_li_fraction_zero_for_no_li_metal() {
        // Only foils, no LithiumMetal — no Li to be dead.
        let bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        let m = compute_morphology_metrics(&bodies);
        assert_eq!(m.dead_li_fraction, 0.0);
    }

    #[test]
    fn dead_li_fraction_zero_for_empty() {
        let bodies: Vec<Body> = Vec::new();
        let m = compute_morphology_metrics(&bodies);
        assert_eq!(m.dead_li_fraction, 0.0);
    }

    #[test]
    fn dead_li_fraction_zero_for_connected_li() {
        // Foil at x=-150 + adjacent Li column at x=-148 (distance 2.0 < cutoff 3.8).
        // All Li atoms connect to foil through the chain.
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        bodies.extend(flat_foil_column(-148.0, 50, Species::LithiumMetal));
        let m = compute_morphology_metrics(&bodies);
        assert_eq!(m.dead_li_fraction, 0.0);
    }

    #[test]
    fn dead_li_fraction_one_for_no_foil() {
        // Li chain only (10 atoms, step 2.0 within cutoff). No foil → 1.0 dead.
        let bodies = flat_foil_column(0.0, 10, Species::LithiumMetal);
        let m = compute_morphology_metrics(&bodies);
        assert!(
            (m.dead_li_fraction - 1.0).abs() < 1e-6,
            "no foil → all Li dead, got {}",
            m.dead_li_fraction
        );
    }

    #[test]
    fn dead_li_fraction_partial_with_isolated_cluster() {
        // Foil + connected Li (50 atoms) + isolated 10-atom cluster.
        // Expected: 10 / (50 + 10) ≈ 0.1667.
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        bodies.extend(flat_foil_column(-148.0, 50, Species::LithiumMetal));
        // Isolated cluster at x=0, step 2.0 (within Li-Li cutoff 3.8 → cluster
        // is internally connected). Far from any foil.
        bodies.extend(flat_foil_column(0.0, 10, Species::LithiumMetal));
        let m = compute_morphology_metrics(&bodies);
        let expected = 10.0_f32 / 60.0_f32;
        assert!(
            (m.dead_li_fraction - expected).abs() < 1e-3,
            "expected {}, got {}",
            expected,
            m.dead_li_fraction
        );
    }

    #[test]
    fn dead_li_fraction_classifies_per_atom() {
        // Build the same partial scenario, then ask the per-atom classifier:
        // connected Li atoms should be Some(false); isolated cluster atoms Some(true);
        // foils Should be None.
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        bodies.extend(flat_foil_column(-148.0, 50, Species::LithiumMetal));
        bodies.extend(flat_foil_column(0.0, 10, Species::LithiumMetal));

        let class = classify_li_metal_dead(&bodies);
        assert_eq!(class.len(), bodies.len());

        // First 50 are foil → None.
        for c in &class[0..50] {
            assert_eq!(*c, None);
        }
        // Next 50 are connected Li → Some(false).
        for c in &class[50..100] {
            assert_eq!(*c, Some(false));
        }
        // Last 10 are isolated cluster → Some(true).
        for c in &class[100..110] {
            assert_eq!(*c, Some(true));
        }
    }

    #[test]
    fn dead_li_fraction_isolated_single_atom() {
        // Foil + connected Li + one singleton Li 50 Å away.
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        bodies.extend(flat_foil_column(-148.0, 50, Species::LithiumMetal));
        let mut iso = Body::new(
            Vec2::new(50.0, 0.0),
            Vec2::zero(),
            Species::LithiumMetal.mass(),
            Species::LithiumMetal.radius(),
            0.0,
            Species::LithiumMetal,
        );
        iso.id = 99_999;
        bodies.push(iso);

        let m = compute_morphology_metrics(&bodies);
        let expected = 1.0_f32 / 51.0_f32;
        assert!(
            (m.dead_li_fraction - expected).abs() < 1e-3,
            "expected {}, got {}",
            expected,
            m.dead_li_fraction
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
