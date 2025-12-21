// test_electrode_hopping.rs
// Debug test for electron hopping between foils and intercalation electrode materials

use particle_sim::body::{Body, Species, Electron};
use particle_sim::simulation::utils::can_transfer_electron;
use ultraviolet::Vec2;

fn main() {
    println!("=== ELECTRODE ELECTRON HOPPING DEBUG TEST ===\n");

    // Test 1: Check neutral electron counts for all relevant species
    println!("--- TEST 1: Neutral Electron Counts ---");
    let species_list = [
        Species::FoilMetal,
        Species::LithiumMetal,
        Species::LithiumIon,
        Species::Graphite,
        Species::HardCarbon,
        Species::SiliconOxide,
        Species::LTO,
        Species::LFP,
        Species::LMFP,
        Species::NMC,
        Species::NCA,
    ];

    for species in &species_list {
        let body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, *species);
        println!(
            "  {:?}: neutral_electron_count = {}",
            species,
            body.neutral_electron_count()
        );
    }

    // Test 2: Create bodies with different electron states and test can_transfer_electron
    println!("\n--- TEST 2: can_transfer_electron() Results ---");
    
    // Create a foil with excess electrons (simulating current injection)
    let mut foil_excess = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
    // Add 2 electrons (foil neutral is typically 0, so this is excess)
    foil_excess.electrons.push(Electron { rel_pos: Vec2::new(0.1, 0.0), vel: Vec2::zero() });
    foil_excess.electrons.push(Electron { rel_pos: Vec2::new(-0.1, 0.0), vel: Vec2::zero() });
    foil_excess.update_charge_from_electrons();
    
    println!("\nFoil with {} electrons (neutral={}):", 
        foil_excess.electrons.len(), 
        foil_excess.neutral_electron_count());
    println!("  Foil charge: {}", foil_excess.charge);
    println!("  Foil electron diff: {} - {} = {}", 
        foil_excess.electrons.len(),
        foil_excess.neutral_electron_count(),
        foil_excess.electrons.len() as i32 - foil_excess.neutral_electron_count() as i32);

    // Test transfers to each electrode material
    for species in &[Species::Graphite, Species::LFP, Species::NMC, Species::HardCarbon] {
        let electrode = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, *species);
        let can_transfer = can_transfer_electron(&foil_excess, &electrode);
        let electrode_diff = electrode.electrons.len() as i32 - electrode.neutral_electron_count() as i32;
        
        println!("\n  Foil -> {:?}:", species);
        println!("    Electrode electrons: {}, neutral: {}, diff: {}", 
            electrode.electrons.len(),
            electrode.neutral_electron_count(),
            electrode_diff);
        println!("    can_transfer_electron: {}", can_transfer);
        
        if !can_transfer {
            // Debug why
            let src_diff = foil_excess.electrons.len() as i32 - foil_excess.neutral_electron_count() as i32;
            let dst_diff = electrode.electrons.len() as i32 - electrode.neutral_electron_count() as i32;
            println!("    DEBUG: src_diff={} >= dst_diff={} ? {}", src_diff, dst_diff, src_diff >= dst_diff);
            println!("    DEBUG: src_diff > dst_diff ? {}", src_diff > dst_diff);
        }
    }

    // Test 3: Test the opposite - electrode with excess electrons -> foil deficit
    println!("\n--- TEST 3: Electrode -> Foil (reverse direction) ---");
    
    let mut graphite_excess = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::Graphite);
    graphite_excess.electrons.push(Electron { rel_pos: Vec2::new(0.1, 0.0), vel: Vec2::zero() });
    graphite_excess.update_charge_from_electrons();
    
    let foil_neutral = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
    
    println!("Graphite with {} electrons -> Foil with {} electrons:", 
        graphite_excess.electrons.len(),
        foil_neutral.electrons.len());
    println!("  can_transfer_electron: {}", can_transfer_electron(&graphite_excess, &foil_neutral));

    // Test 4: Check the electron hopping source filter
    println!("\n--- TEST 4: Electron Hopping Source Filter Check ---");
    println!("Checking if electrode species pass the 'is_conductor' filter in electron_hopping.rs:");
    
    for species in &species_list {
        let is_conductor = matches!(
            species,
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
        println!("  {:?}: is_conductor = {}", species, is_conductor);
    }

    // Test 5: Simulate actual hopping scenario
    println!("\n--- TEST 5: Simulated Hopping Scenario ---");
    
    // Foil has 2 excess electrons
    let mut src = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
    src.electrons.push(Electron { rel_pos: Vec2::new(0.1, 0.0), vel: Vec2::zero() });
    src.electrons.push(Electron { rel_pos: Vec2::new(-0.1, 0.0), vel: Vec2::zero() });
    src.update_charge_from_electrons();
    
    // Graphite has 0 electrons
    let dst = Body::new(Vec2::new(5.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::Graphite);
    
    let src_diff = src.electrons.len() as i32 - src.neutral_electron_count() as i32;
    let dst_diff = dst.electrons.len() as i32 - dst.neutral_electron_count() as i32;
    
    println!("Source (FoilMetal):");
    println!("  electrons: {}, neutral: {}, diff: {}", src.electrons.len(), src.neutral_electron_count(), src_diff);
    println!("  species check: {}", src.species == Species::LithiumMetal || src.species == Species::FoilMetal);
    println!("  src_diff >= 0: {}", src_diff >= 0);
    
    println!("\nDestination (Graphite):");
    println!("  electrons: {}, neutral: {}, diff: {}", dst.electrons.len(), dst.neutral_electron_count(), dst_diff);
    
    println!("\nHopping conditions:");
    println!("  src_diff >= dst_diff: {} >= {} = {}", src_diff, dst_diff, src_diff >= dst_diff);
    println!("  can_transfer_electron: {}", can_transfer_electron(&src, &dst));
    
    // The key insight: if neutral_electron_count is 0 for both, and foil has 2 electrons:
    // src_diff = 2 - 0 = 2
    // dst_diff = 0 - 0 = 0
    // src_diff > dst_diff: 2 > 0 = true
    // So transfer SHOULD be allowed
    
    println!("\n--- TEST 6: Check max electron limits ---");
    println!("FOIL_MAX_ELECTRONS: {}", particle_sim::config::FOIL_MAX_ELECTRONS);
    println!("LITHIUM_METAL_MAX_ELECTRONS: {}", particle_sim::config::LITHIUM_METAL_MAX_ELECTRONS);
    println!("ELECTRODE_ANODE_MAX_ELECTRONS: {}", particle_sim::config::ELECTRODE_ANODE_MAX_ELECTRONS);
    println!("ELECTRODE_CATHODE_MAX_ELECTRONS: {}", particle_sim::config::ELECTRODE_CATHODE_MAX_ELECTRONS);
    
    // Check if dst would exceed max
    let dst_max = particle_sim::config::ELECTRODE_ANODE_MAX_ELECTRONS;
    println!("\nWould Graphite exceed max? {} >= {} = {}", dst.electrons.len(), dst_max, dst.electrons.len() >= dst_max);

    // Test 7: Check electrode-to-electrode hopping (the critical fix)
    println!("\n--- TEST 7: Electrode-to-Electrode Hopping (charge-based) ---");
    
    // Graphite with 1 electron
    let mut graphite1 = Body::new(Vec2::new(0.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::Graphite);
    graphite1.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
    graphite1.update_charge_from_electrons();
    
    // Graphite with 0 electrons
    let graphite2 = Body::new(Vec2::new(3.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::Graphite);
    
    println!("Graphite1 (1 electron): charge = {}", graphite1.charge);
    println!("Graphite2 (0 electrons): charge = {}", graphite2.charge);
    
    let d_phi = graphite2.charge - graphite1.charge;
    println!("d_phi = dst.charge - src.charge = {} - {} = {}", graphite2.charge, graphite1.charge, d_phi);
    println!("d_phi > 0? {} (required for non-BV hopping)", d_phi > 0.0);
    println!("can_transfer_electron: {}", can_transfer_electron(&graphite1, &graphite2));
    
    // Also test foil -> electrode charge difference
    println!("\n--- TEST 8: Foil -> Electrode Charge Difference ---");
    let mut foil_charged = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
    foil_charged.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
    foil_charged.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
    foil_charged.update_charge_from_electrons();
    
    let electrode_empty = Body::new(Vec2::new(3.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::Graphite);
    
    println!("Foil (2 electrons, neutral=1): charge = {}", foil_charged.charge);
    println!("Graphite (0 electrons, neutral=0): charge = {}", electrode_empty.charge);
    
    let d_phi_foil = electrode_empty.charge - foil_charged.charge;
    println!("d_phi = {} - {} = {}", electrode_empty.charge, foil_charged.charge, d_phi_foil);
    println!("d_phi > 0? {} (required for non-BV hopping)", d_phi_foil > 0.0);

    println!("\n=== END OF TESTS ===");
}
