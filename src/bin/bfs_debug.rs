//! Debug: inspect actual body spacing and BFS reach for an 80x80 electrode.

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use particle_sim::simulation::Simulation;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::channel;
use ultraviolet::Vec2;

fn template_body(species: Species) -> Body {
    Body::new(
        Vec2::zero(), Vec2::zero(),
        species.mass(), species.radius(), 0.0, species,
    )
}

fn main() {
    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let mut sim = Simulation::new();
    let esize = 80.0f32;
    let foil_w = 20.0f32;
    let domain_w = 400.0f32;
    let domain_h = 240.0f32;
    sim.domain_width = domain_w / 2.0;
    sim.domain_height = domain_h / 2.0;
    sim.cell_list.update_domain_size(sim.domain_width, sim.domain_height);

    let half_sep = domain_w / 4.0;

    let half_e = esize / 2.0;
    // Left electrode — center at -half_sep, convert to bottom-left origin
    handle_command(SimCommand::AddRectangle {
        body: template_body(Species::LithiumMetal),
        x: -half_sep - half_e, y: -half_e, width: esize, height: esize,
    }, &mut sim);
    handle_command(SimCommand::AddFoil {
        width: foil_w, height: esize,
        x: -half_sep - half_e, y: -half_e,
        particle_radius: Species::FoilMetal.radius(), current: 0.0001,
    }, &mut sim);

    // Count species
    let n_metal = sim.bodies.iter().filter(|b| b.species == Species::LithiumMetal).count();
    let n_foil = sim.bodies.iter().filter(|b| b.species == Species::FoilMetal).count();
    println!("Total bodies: {}", sim.bodies.len());
    println!("Li metal: {}, Foil: {}", n_metal, n_foil);
    println!("Expected electrode total: {}", n_metal + n_foil);

    // Find distances between foil bodies and their nearest metal neighbor
    let foil_ids: HashSet<u64> = sim.foils[0].body_ids.iter().copied().collect();
    let mut min_dists: Vec<f32> = Vec::new();
    for b in &sim.bodies {
        if !foil_ids.contains(&b.id) { continue; }
        let mut closest = f32::MAX;
        for other in &sim.bodies {
            if other.id == b.id { continue; }
            if other.species != Species::LithiumMetal { continue; }
            let d = (b.pos - other.pos).mag();
            if d < closest { closest = d; }
        }
        if closest < f32::MAX {
            min_dists.push(closest);
        }
    }
    min_dists.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("\nFoil→Metal nearest neighbor distances:");
    println!("  Min:    {:.4} Å", min_dists.first().unwrap_or(&0.0));
    println!("  Max:    {:.4} Å", min_dists.last().unwrap_or(&0.0));
    println!("  Median: {:.4} Å", min_dists.get(min_dists.len()/2).unwrap_or(&0.0));
    println!("  Mean:   {:.4} Å", min_dists.iter().sum::<f32>() / min_dists.len() as f32);

    // Also check metal-metal nearest neighbor distances
    let metals: Vec<usize> = sim.bodies.iter().enumerate()
        .filter(|(_, b)| b.species == Species::LithiumMetal)
        .map(|(i, _)| i).collect();
    let mut mm_dists: Vec<f32> = Vec::new();
    for &i in &metals {
        let mut closest = f32::MAX;
        for &j in &metals {
            if i == j { continue; }
            let d = (sim.bodies[i].pos - sim.bodies[j].pos).mag();
            if d < closest { closest = d; }
        }
        if closest < f32::MAX { mm_dists.push(closest); }
    }
    mm_dists.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("\nMetal→Metal nearest neighbor distances:");
    println!("  Min:    {:.4} Å", mm_dists.first().unwrap_or(&0.0));
    println!("  Max:    {:.4} Å", mm_dists.last().unwrap_or(&0.0));
    println!("  Median: {:.4} Å", mm_dists.get(mm_dists.len()/2).unwrap_or(&0.0));
    println!("  Mean:   {:.4} Å", mm_dists.iter().sum::<f32>() / mm_dists.len() as f32);

    // BFS threshold used by calculate_foil_electron_ratio
    let r_foil = Species::FoilMetal.radius();
    let r_metal = Species::LithiumMetal.radius();
    let bfs_threshold = (r_foil + r_metal) * 1.1;
    println!("\nBFS connection threshold: ({:.2} + {:.2}) × 1.1 = {:.4} Å", r_foil, r_metal, bfs_threshold);

    // How many foil bodies have a metal neighbor within BFS threshold?
    let foil_connected = min_dists.iter().filter(|&&d| d <= bfs_threshold).count();
    println!("Foil bodies with metal within threshold: {}/{}", foil_connected, min_dists.len());
    let foil_connected_loose = min_dists.iter().filter(|&&d| d <= 5.0).count();
    println!("Foil bodies with metal within 5.0 Å: {}/{}", foil_connected_loose, min_dists.len());

    // Try BFS at different thresholds
    let id_to_idx: HashMap<u64, usize> = sim.bodies.iter().enumerate().map(|(i, b)| (b.id, i)).collect();
    for threshold_factor in [1.1f32, 1.5, 2.0, 2.5, 3.0] {
        let mut visited: HashSet<usize> = HashSet::new();
        let mut queue: VecDeque<usize> = VecDeque::new();
        for &id in &sim.foils[0].body_ids {
            if let Some(&idx) = id_to_idx.get(&id) {
                if visited.insert(idx) { queue.push_back(idx); }
            }
        }
        while let Some(idx) = queue.pop_front() {
            let body = &sim.bodies[idx];
            for (j, other) in sim.bodies.iter().enumerate() {
                if visited.contains(&j) { continue; }
                if !matches!(other.species, Species::LithiumMetal | Species::FoilMetal) { continue; }
                let threshold = (body.radius + other.radius) * threshold_factor;
                if (body.pos - other.pos).mag() <= threshold {
                    if visited.insert(j) { queue.push_back(j); }
                }
            }
        }
        println!("BFS factor={:.1}: found {} bodies (expected {})", threshold_factor, visited.len(), n_metal + n_foil);
    }
}
