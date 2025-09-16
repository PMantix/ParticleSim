use super::Quadtree;
use crate::body::Body;
use std::ops::Range;
use ultraviolet::Vec2;

/// Result of sampling the electrostatic field solver at a single point.
#[derive(Clone, Copy, Debug, Default)]
pub struct FieldSample {
    pub potential: f32,
    pub field: Vec2,
}

/// Check whether a value lies inside a range while handling potentially invalid ranges.
fn range_contains(range: &Range<usize>, value: usize) -> bool {
    range.start < range.end && value >= range.start && value < range.end
}

/// Evaluate the electric potential and field at `pos`, excluding bodies listed in `excluded`.
///
/// This performs a Barnes-Hut traversal similar to [`Quadtree::acc_pos`] but gathers both the
/// scalar potential and the vector field. Nodes that contain an excluded body are always
/// descended so that the excluded bodies can be skipped explicitly.
pub fn evaluate_field_at_point_excluding(
    quadtree: &Quadtree,
    bodies: &[Body],
    pos: Vec2,
    radius: f32,
    k_e: f32,
    excluded: &[usize],
) -> FieldSample {
    if bodies.is_empty() {
        return FieldSample::default();
    }

    let mut potential = 0.0f32;
    let mut field = Vec2::zero();
    let mut stack = vec![Quadtree::ROOT];

    while let Some(node_idx) = stack.pop() {
        if node_idx >= quadtree.nodes.len() {
            continue;
        }

        let node = &quadtree.nodes[node_idx];

        if node.bodies.is_empty() {
            continue;
        }

        let contains_excluded = excluded
            .iter()
            .copied()
            .any(|idx| range_contains(&node.bodies, idx));

        let d = pos - node.pos;
        let dist = d.mag();
        let node_radius = node.quad.size * 0.5;
        let min_sep = (radius + node_radius).max(1e-4);
        let dist_adj = (dist - radius).max(0.0);
        let size_sq = node.quad.size * node.quad.size;
        let should_approximate = size_sq < dist_adj * dist_adj * quadtree.t_sq;

        if should_approximate && !contains_excluded && !node.is_leaf() {
            let r_eff = dist.max(min_sep);
            let r_sq = r_eff * r_eff + quadtree.e_sq;
            if r_sq > 0.0 && r_sq.is_finite() {
                let inv_r = r_sq.sqrt().recip();
                potential += k_e * node.charge * inv_r;

                let denom = r_sq * r_eff.max(1e-6);
                if denom > 0.0 && denom.is_finite() {
                    field += d * (k_e * node.charge / denom);
                }
            }
            continue;
        }

        if node.is_leaf() || contains_excluded {
            for body_idx in node.bodies.clone() {
                if body_idx >= bodies.len() {
                    continue;
                }
                if excluded.contains(&body_idx) {
                    continue;
                }

                let body = &bodies[body_idx];
                if !body.pos.x.is_finite()
                    || !body.pos.y.is_finite()
                    || !body.charge.is_finite()
                {
                    continue;
                }

                let d = pos - body.pos;
                let dist = d.mag();
                let min_sep = (radius + body.radius).max(1e-4);
                let r_eff = dist.max(min_sep);
                let r_sq = r_eff * r_eff + quadtree.e_sq;
                if r_sq <= 0.0 || !r_sq.is_finite() {
                    continue;
                }

                let inv_r = r_sq.sqrt().recip();
                potential += k_e * body.charge * inv_r;

                let denom = r_sq * r_eff.max(1e-6);
                if denom > 0.0 && denom.is_finite() {
                    field += d * (k_e * body.charge / denom);
                }
            }
        } else if node.children != 0 {
            for offset in 0..4 {
                let child_idx = node.children + offset;
                if child_idx < quadtree.nodes.len() {
                    stack.push(child_idx);
                }
            }
        }
    }

    FieldSample { potential, field }
}

/// Convenience wrapper that returns only the potential component.
#[inline]
pub fn potential_at_point_excluding(
    quadtree: &Quadtree,
    bodies: &[Body],
    pos: Vec2,
    radius: f32,
    k_e: f32,
    excluded: &[usize],
) -> f32 {
    evaluate_field_at_point_excluding(quadtree, bodies, pos, radius, k_e, excluded).potential
}

/// Convenience wrapper that returns only the electric field component.
#[inline]
pub fn field_at_point_excluding(
    quadtree: &Quadtree,
    bodies: &[Body],
    pos: Vec2,
    radius: f32,
    k_e: f32,
    excluded: &[usize],
) -> Vec2 {
    evaluate_field_at_point_excluding(quadtree, bodies, pos, radius, k_e, excluded).field
}
