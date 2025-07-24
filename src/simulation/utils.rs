use crate::body::Body;
use crate::body::Species;

/// Returns true if an electron should be allowed to hop from src to dst
pub fn can_transfer_electron(src: &Body, dst: &Body) -> bool {
    // Check if destination would exceed maximum electron limit
    let dst_max_electrons = match dst.species {
        Species::FoilMetal => crate::config::FOIL_MAX_ELECTRONS,
        Species::LithiumMetal => crate::config::LITHIUM_METAL_MAX_ELECTRONS,
        _ => usize::MAX, // No limit for other species
    };
    
    if dst.electrons.len() >= dst_max_electrons {
        return false; // Destination already at maximum capacity
    }
    
    // Calculate the difference between the current and the neutral count.
    // A positive value means surplus; zero means neutral; negative means deficiency.
    let src_diff = src.electrons.len() as i32 - src.neutral_electron_count() as i32;
    let dst_diff = dst.electrons.len() as i32 - dst.neutral_electron_count() as i32;
    
    // If source has neutral or surplus electrons, transfer is allowed when donor is "richer" than acceptor.
    if src_diff >= 0 {
        src_diff > dst_diff
    } else {
        // If source is deficient, only allow transfer if it is very deficient (for example, less than -1)
        // and the target is also deficient.
        src_diff < -1 && dst_diff < 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::Electron;
    use ultraviolet::Vec2;

    #[test]
    fn test_lithium_metal_max_electrons() {
        // Create a lithium metal particle with max electrons
        let mut lithium_metal = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        for _ in 0..crate::config::LITHIUM_METAL_MAX_ELECTRONS {
            lithium_metal.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        }
        
        // Create a deficient lithium metal particle
        let deficient_lithium = Body::new(Vec2::new(1.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        // This one has no electrons (deficient)
        
        // Try to transfer electron from max to deficient - should work
        assert!(can_transfer_electron(&lithium_metal, &deficient_lithium));
        
        // Try to transfer electron to a lithium metal that's already at max - should fail
        let mut another_max_lithium = Body::new(Vec2::new(2.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        for _ in 0..crate::config::LITHIUM_METAL_MAX_ELECTRONS {
            another_max_lithium.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        }
        
        assert!(!can_transfer_electron(&lithium_metal, &another_max_lithium));
    }

    #[test]
    fn test_foil_metal_max_electrons_still_works() {
        let mut dst = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        let src = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        
        // Fill destination to maximum capacity
        for _ in 0..crate::config::FOIL_MAX_ELECTRONS {
            dst.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        }
        
        // Should not allow transfer when destination is at max
        assert!(!can_transfer_electron(&src, &dst));
    }
}
