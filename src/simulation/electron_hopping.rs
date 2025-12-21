// electron_hopping.rs
// Electron transfer and hopping logic between particles

use super::Simulation;
use crate::body::Species;
use crate::profile_scope;
use crate::simulation::utils::can_transfer_electron;
use rand::prelude::*;
use rayon::prelude::*;
use ultraviolet::Vec2;
use std::sync::atomic::{AtomicU64, Ordering};

// Debug counters for electrode hopping diagnostics
static DEBUG_FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);

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

        // Debug: track electrode hopping attempts
        let frame = DEBUG_FRAME_COUNTER.fetch_add(1, Ordering::Relaxed);
        let debug_this_frame = frame % 500 == 0; // Print every 500 frames
        
        let mut foil_with_excess = 0usize;
        let mut foil_with_deficit = 0usize;
        let mut electrode_with_electrons = 0usize;
        let mut electrode_neighbors_found = 0usize;
        let mut electrode_hops_attempted = 0usize;
        let mut electrode_hops_failed_alignment = 0usize;
        let mut electrode_hops_failed_dphi = 0usize;
        let mut electrode_hops_failed_rate = 0usize;
        let mut electrode_hops_failed_prob = 0usize;
        let mut electrode_hops_succeeded = 0usize;
        let mut electrode_to_foil_attempted = 0usize;
        let mut electrode_to_foil_succeeded = 0usize;

        for &src_idx in &src_indices {
            if donated_electron[src_idx] || exclude_donor[src_idx] {
                continue;
            }
            let src_body = &self.bodies[src_idx];
            let src_diff =
                src_body.electrons.len() as i32 - src_body.neutral_electron_count() as i32;
            // Allow electron donation from metals and intercalation electrode materials
            let is_conductor = matches!(
                src_body.species,
                Species::LithiumMetal
                    | Species::FoilMetal
                    | Species::Graphite
                    | Species::HardCarbon
                    | Species::SiliconOxide
                    | Species::LTO
                    | Species::LFP
                    | Species::LMFP
                    | Species::NMC
                    | Species::NCA
            );
            if !is_conductor || src_diff < 0 {
                continue;
            }
            
            // Debug: track foils with excess/deficit electrons and electrodes with electrons
            let src_is_foil = src_body.species == Species::FoilMetal;
            let src_is_electrode = matches!(
                src_body.species,
                Species::Graphite | Species::HardCarbon | Species::SiliconOxide | Species::LTO |
                Species::LFP | Species::LMFP | Species::NMC | Species::NCA
            );
            if src_is_foil && src_diff > 0 {
                foil_with_excess += 1;
            }
            if src_is_electrode && src_body.electrons.len() > 0 {
                electrode_with_electrons += 1;
            }
            
            let hop_radius = self.config.hop_radius_factor * src_body.radius;

            // Use quadtree for neighbor search!
            let all_neighbors = self
                .quadtree
                .find_neighbors_within(&self.bodies, src_idx, hop_radius);
            
            // Debug: check if foils have electrode neighbors
            if src_is_foil && src_diff > 0 && debug_this_frame && foil_with_excess <= 3 {
                let electrode_neighbor_count = all_neighbors.iter()
                    .filter(|&&idx| matches!(
                        self.bodies[idx].species,
                        Species::Graphite | Species::HardCarbon | Species::SiliconOxide | Species::LTO |
                        Species::LFP | Species::LMFP | Species::NMC | Species::NCA
                    ))
                    .count();
                if electrode_neighbor_count == 0 {
                    eprintln!("[HOPPING] Foil idx={} has {} electrons (excess {}) but NO electrode neighbors within hop_radius={:.2}",
                        src_idx, src_body.electrons.len(), src_diff, hop_radius);
                    // Check what neighbors it does have
                    let neighbor_species: Vec<_> = all_neighbors.iter()
                        .take(5)
                        .map(|&idx| format!("{:?}", self.bodies[idx].species))
                        .collect();
                    eprintln!("[HOPPING]   Found {} neighbors, first few: {:?}", all_neighbors.len(), neighbor_species);
                }
            }
            
            let mut candidate_neighbors = all_neighbors
                .into_iter()
                .filter(|&dst_idx| dst_idx != src_idx && !received_electron[dst_idx])
                .filter(|&dst_idx| {
                    let dst_body = &self.bodies[dst_idx];
                    let dst_diff =
                        dst_body.electrons.len() as i32 - dst_body.neutral_electron_count() as i32;
                    // Allow hop if donor is more excess than recipient
                    if src_diff >= dst_diff {
                        match dst_body.species {
                            // Standard species that can receive electrons
                            Species::LithiumMetal | Species::FoilMetal | Species::LithiumIon => {
                                can_transfer_electron(src_body, dst_body)
                            }
                            // Intercalation electrode materials can receive electrons
                            Species::Graphite
                            | Species::HardCarbon
                            | Species::SiliconOxide
                            | Species::LTO
                            | Species::LFP
                            | Species::LMFP
                            | Species::NMC
                            | Species::NCA => can_transfer_electron(src_body, dst_body),
                            _ => false,
                        }
                    } else {
                        false
                    }
                })
                .collect::<Vec<_>>();

            candidate_neighbors.shuffle(&mut rng);
            
            // Debug: track electrode neighbor candidates
            let has_electrode_candidate = candidate_neighbors.iter().any(|&idx| {
                matches!(
                    self.bodies[idx].species,
                    Species::Graphite | Species::HardCarbon | Species::SiliconOxide | Species::LTO |
                    Species::LFP | Species::LMFP | Species::NMC | Species::NCA
                )
            });
            if (src_is_foil || src_is_electrode) && has_electrode_candidate {
                electrode_neighbors_found += 1;
            }

            // Only check until the first successful hop
            if let Some(&dst_idx) = candidate_neighbors.iter().find(|&&dst_idx| {
                let dst_body = &self.bodies[dst_idx];
                let dst_is_foil = dst_body.species == Species::FoilMetal;
                let dst_is_electrode = matches!(
                    dst_body.species,
                    Species::Graphite | Species::HardCarbon | Species::SiliconOxide | Species::LTO |
                    Species::LFP | Species::LMFP | Species::NMC | Species::NCA
                );
                let is_electrode_hop = (src_is_foil || src_is_electrode) && dst_is_electrode;
                let is_electrode_to_foil = src_is_electrode && dst_is_foil;
                
                if is_electrode_hop {
                    electrode_hops_attempted += 1;
                }
                if is_electrode_to_foil {
                    electrode_to_foil_attempted += 1;
                }
                
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
                
                // For electrode hops (foil<->electrode or electrode<->electrode), 
                // use relaxed alignment - electrode materials conduct electrons freely
                let is_electrode_material = |s: Species| matches!(s,
                    Species::Graphite | Species::HardCarbon | Species::SiliconOxide | Species::LTO |
                    Species::LFP | Species::LMFP | Species::NMC | Species::NCA
                );
                let is_metal_or_electrode = |s: Species| matches!(s,
                    Species::FoilMetal | Species::LithiumMetal |
                    Species::Graphite | Species::HardCarbon | Species::SiliconOxide | Species::LTO |
                    Species::LFP | Species::LMFP | Species::NMC | Species::NCA
                );
                
                // Relax alignment for conductive pathways (foil-electrode or electrode-electrode)
                let both_conductive = is_metal_or_electrode(src_body.species) && is_metal_or_electrode(dst_body.species);
                let involves_electrode = is_electrode_material(src_body.species) || is_electrode_material(dst_body.species);
                
                if both_conductive && involves_electrode {
                    // Use minimum alignment of 0.5 for electrode conduction paths
                    alignment = alignment.max(0.5);
                }
                
                if alignment < 1e-3 {
                    if is_electrode_hop {
                        electrode_hops_failed_alignment += 1;
                    }
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
                        for e in &src_body.electrons {
                            v += e.rel_pos;
                        }
                        v / (src_body.electrons.len() as f32)
                    } else {
                        Vec2::zero()
                    };
                    let dst_pol = if !dst_body.electrons.is_empty() {
                        let mut v = Vec2::zero();
                        for e in &dst_body.electrons {
                            v += e.rel_pos;
                        }
                        v / (dst_body.electrons.len() as f32)
                    } else {
                        Vec2::zero()
                    };
                    // For a vacancy moving from src to dst, we want the hop direction to align with
                    // the local electron offset direction (electrons displaced roughly opposite external field).
                    let pol_dir = if (src_pol + dst_pol).mag() > 1e-6 {
                        (src_pol + dst_pol).normalized()
                    } else {
                        Vec2::zero()
                    };
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
                        if is_electrode_hop {
                            electrode_hops_failed_dphi += 1;
                        }
                        return false;
                    }
                    self.config.hop_rate_k0
                        * (self.config.hop_transfer_coeff * d_phi
                            / self.config.hop_activation_energy)
                            .exp()
                };

                if rate <= 0.0 {
                    if is_electrode_hop {
                        electrode_hops_failed_rate += 1;
                    }
                    return false;
                }
                let p_hop = alignment * polarization_factor * (1.0 - (-rate * self.dt).exp());
                let succeeded = rand::random::<f32>() < p_hop;
                if is_electrode_hop {
                    if succeeded {
                        electrode_hops_succeeded += 1;
                    } else {
                        electrode_hops_failed_prob += 1;
                    }
                }
                if is_electrode_to_foil && succeeded {
                    electrode_to_foil_succeeded += 1;
                }
                succeeded
            }) {
                hops.push((src_idx, dst_idx));
                received_electron[dst_idx] = true;
                donated_electron[src_idx] = true;
            }
        }
        
        // Count foils with deficit (for discharge tracking)
        for body in &self.bodies {
            if body.species == Species::FoilMetal {
                let diff = body.electrons.len() as i32 - body.neutral_electron_count() as i32;
                if diff < 0 {
                    foil_with_deficit += 1;
                }
            }
        }
        
        // Debug summary
        if debug_this_frame {
            eprintln!("[HOPPING] Frame {} Summary:", frame);
            eprintln!("  Foils with excess electrons: {}", foil_with_excess);
            eprintln!("  Foils with deficit (need electrons): {}", foil_with_deficit);
            eprintln!("  Electrode particles with electrons: {}", electrode_with_electrons);
            eprintln!("  Electrode neighbors found: {}", electrode_neighbors_found);
            eprintln!("  Electrode hops attempted: {}", electrode_hops_attempted);
            eprintln!("  Electrode->Foil attempted: {}", electrode_to_foil_attempted);
            eprintln!("  Electrode->Foil succeeded: {}", electrode_to_foil_succeeded);
            eprintln!("  Failed - alignment: {}", electrode_hops_failed_alignment);
            eprintln!("  Failed - d_phi <= 0: {}", electrode_hops_failed_dphi);
            eprintln!("  Failed - rate <= 0: {}", electrode_hops_failed_rate);
            eprintln!("  Failed - probability: {}", electrode_hops_failed_prob);
            eprintln!("  Succeeded: {}", electrode_hops_succeeded);
            eprintln!("  Total hops this frame: {}", hops.len());
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
