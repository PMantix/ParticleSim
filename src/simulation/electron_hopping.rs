// electron_hopping.rs
// Electron transfer and hopping logic between particles

use crate::body::Species;
use crate::profile_scope;
use crate::simulation::utils::can_transfer_electron;
use rand::prelude::*;
use rayon::prelude::*;
use ultraviolet::Vec2;
use super::Simulation;

impl Simulation {
    /// Attempts electron hopping between particles, with optional exclusions for donors
    /// (used for foil current sources). When `use_butler_volmer` is enabled
    /// in the configuration, hops between different species use the
    /// Butler-Volmer rate expression.
    pub fn perform_electron_hopping_with_exclusions(&mut self, exclude_donor: &[bool]) {
        if self.bodies.is_empty() {
            return;
        }
        let n = self.bodies.len();
        let mut hops: Vec<(usize, usize)> = vec![];
        let mut received_electron = vec![false; n];
        let mut donated_electron = vec![false; n];
        let mut src_indices: Vec<usize> = (0..n).collect();
        let mut rng = rand::rng();
        src_indices.shuffle(&mut rng);
        
        for &src_idx in &src_indices {
            if donated_electron[src_idx] || exclude_donor[src_idx] {
                continue;
            }
            let src_body = &self.bodies[src_idx];
            let src_diff =
                src_body.electrons.len() as i32 - src_body.neutral_electron_count() as i32;
            if !(src_body.species == Species::LithiumMetal
                || src_body.species == Species::FoilMetal)
                || src_diff < 0
            {
                continue;
            }
            let hop_radius = self.config.hop_radius_factor * src_body.radius;

            // Use quadtree for neighbor search!
            let mut candidate_neighbors = self
                .quadtree
                .find_neighbors_within(&self.bodies, src_idx, hop_radius)
                .into_iter()
                .filter(|&dst_idx| dst_idx != src_idx && !received_electron[dst_idx])
                .filter(|&dst_idx| {
                    let dst_body = &self.bodies[dst_idx];
                    let dst_diff =
                        dst_body.electrons.len() as i32 - dst_body.neutral_electron_count() as i32;
                    // Allow hop if donor is more excess than recipient
                    if src_diff >= dst_diff {
                        match dst_body.species {
                            Species::LithiumMetal | Species::FoilMetal | Species::LithiumIon => {
                                can_transfer_electron(src_body, dst_body)
                            }
                            _ => false,
                        }
                    } else {
                        false
                    }
                })
                .collect::<Vec<_>>();

            candidate_neighbors.shuffle(&mut rng);

            // Only check until the first successful hop
            if let Some(&dst_idx) = candidate_neighbors.iter().find(|&&dst_idx| {
                let dst_body = &self.bodies[dst_idx];
                let d_phi = dst_body.charge - src_body.charge;
                let hop_vec = dst_body.pos - src_body.pos;
                let hop_dir = if hop_vec.mag() > 1e-6 {
                    hop_vec.normalized()
                } else {
                    Vec2::zero()
                };
                let local_field = self.background_e_field
                    + self.quadtree.field_at_point(
                        &self.bodies,
                        src_body.pos,
                        self.config.coulomb_constant,
                    );
                let field_dir = if local_field.mag() > 1e-6 {
                    local_field.normalized()
                } else {
                    Vec2::zero()
                };
                let mut alignment = (-hop_dir.dot(field_dir)).max(0.0);
                if field_dir == Vec2::zero() {
                    alignment = 1.0;
                }
                let bias = self.config.hop_alignment_bias.max(0.0);
                // Scale the alignment by the bias factor (no clamping to allow amplification > 1.0)
                alignment = alignment * bias;
                if alignment < 1e-3 {
                    return false;
                }

                // Vacancy polarization bias: favor hops that reduce local electron offset direction
                let mut polarization_factor = 1.0f32;
                let pol_gain = self.config.hop_vacancy_polarization_gain.max(0.0);
                if pol_gain > 0.0 {
                    // Estimate local electron offset vectors at src and dst
                    // Use average of electron relative positions as a proxy for polarization direction
                    let src_pol = if !src_body.electrons.is_empty() {
                        let mut v = Vec2::zero();
                        for e in &src_body.electrons { v += e.rel_pos; }
                        v / (src_body.electrons.len() as f32)
                    } else { Vec2::zero() };
                    let dst_pol = if !dst_body.electrons.is_empty() {
                        let mut v = Vec2::zero();
                        for e in &dst_body.electrons { v += e.rel_pos; }
                        v / (dst_body.electrons.len() as f32)
                    } else { Vec2::zero() };
                    // For a vacancy moving from src to dst, we want the hop direction to align with
                    // the local electron offset direction (electrons displaced roughly opposite external field).
                    let pol_dir = if (src_pol + dst_pol).mag() > 1e-6 { (src_pol + dst_pol).normalized() } else { Vec2::zero() };
                    if pol_dir != Vec2::zero() && hop_dir != Vec2::zero() {
                        let align = hop_dir.dot(pol_dir).max(0.0); // [0,1]
                        // Map to multiplier 1 + gain*align (kept modest to avoid dominance)
                        polarization_factor = 1.0 + pol_gain * align;
                    }
                }

                let rate = if self.config.use_butler_volmer && src_body.species != dst_body.species
                {
                    // Butler-Volmer kinetics for inter-species electron transfer
                    let alpha = self.config.bv_transfer_coeff;
                    let scale = self.config.bv_overpotential_scale;
                    let i0 = self.config.bv_exchange_current;
                    let forward = (alpha * d_phi / scale).exp();
                    let backward = (-(1.0 - alpha) * d_phi / scale).exp();
                    i0 * (forward - backward)
                } else {
                    if d_phi <= 0.0 {
                        return false;
                    }
                    self.config.hop_rate_k0
                        * (self.config.hop_transfer_coeff * d_phi
                            / self.config.hop_activation_energy)
                            .exp()
                };

                if rate <= 0.0 {
                    return false;
                }
                let p_hop = alignment * polarization_factor * (1.0 - (-rate * self.dt).exp());
                rand::random::<f32>() < p_hop
            }) {
                hops.push((src_idx, dst_idx));
                received_electron[dst_idx] = true;
                donated_electron[src_idx] = true;
            }
        }
        
        for (src_idx, dst_idx) in hops {
            if let Some(electron) = self.bodies[src_idx].electrons.pop() {
                self.bodies[dst_idx].electrons.push(electron);
                self.bodies[src_idx].update_charge_from_electrons();
                self.bodies[dst_idx].update_charge_from_electrons();
            }
        }
        
        // Split immutable borrows for rayon safety
        profile_scope!("apply_redox");
        self.bodies.par_iter_mut().for_each(|body| {
            body.apply_redox();
        });
    }
}