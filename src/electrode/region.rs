// electrode/region.rs
// Defines ActiveMaterialRegion - a group of particles representing an intercalation electrode

use super::material::MaterialType;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Unique identifier for an active material region
pub type RegionId = u64;

/// An active material region represents a contiguous electrode area
/// that can store lithium via intercalation/alloying.
/// 
/// Unlike foils (which are current collectors), active material regions
/// track lithium content and state of charge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveMaterialRegion {
    /// Unique identifier for this region
    pub id: RegionId,
    
    /// The electrode material type
    pub material: MaterialType,
    
    /// IDs of surface particles (bodies) that belong to this region
    /// These particles represent the electrode surface where reactions occur
    pub surface_body_ids: HashSet<u64>,
    
    /// Associated foil ID (current collector), if any
    pub foil_id: Option<u64>,
    
    // === Lithium Content Tracking ===
    
    /// Maximum number of Li atoms this region can hold (based on stoichiometry and size)
    pub lithium_capacity: usize,
    
    /// Current number of Li atoms stored in this region
    pub lithium_count: usize,
    
    /// Current state of charge (0.0 = empty, 1.0 = fully lithiated)
    /// Calculated as lithium_count / lithium_capacity
    pub state_of_charge: f32,
    
    // === Geometry ===
    
    /// Total surface area (Å²) for rate calculations
    pub surface_area: f32,
    
    /// Center position of the region (for display purposes)
    pub center_x: f32,
    pub center_y: f32,
    
    // === Statistics ===
    
    /// Cumulative Li intercalated since creation
    pub total_intercalated: usize,
    
    /// Cumulative Li deintercalated since creation  
    pub total_deintercalated: usize,
    
    /// Current rate of change (Li/fs) - for display
    pub current_rate: f32,
}

use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_REGION_ID: AtomicU64 = AtomicU64::new(1);

impl ActiveMaterialRegion {
    /// Create a new active material region
    pub fn new(material: MaterialType, surface_area: f32) -> Self {
        // Estimate capacity from surface area and material properties
        // Rough estimate: 1 Li site per ~10 Ų for graphite
        let site_density = match material {
            MaterialType::Graphite => 0.1,      // sites/Å²
            MaterialType::HardCarbon => 0.08,
            MaterialType::LFP => 0.05,
            MaterialType::NMC => 0.06,
            _ => 0.1,
        };
        
        let lithium_capacity = ((surface_area * site_density) as usize).max(1);
        
        Self {
            id: NEXT_REGION_ID.fetch_add(1, Ordering::Relaxed),
            material,
            surface_body_ids: HashSet::new(),
            foil_id: None,
            lithium_capacity,
            lithium_count: 0,
            state_of_charge: 0.0,
            surface_area,
            center_x: 0.0,
            center_y: 0.0,
            total_intercalated: 0,
            total_deintercalated: 0,
            current_rate: 0.0,
        }
    }
    
    /// Create a region with specified initial state of charge
    pub fn with_initial_soc(mut self, soc: f32) -> Self {
        let soc = soc.clamp(0.0, 1.0);
        self.lithium_count = (self.lithium_capacity as f32 * soc) as usize;
        self.state_of_charge = soc;
        self
    }
    
    /// Update state of charge from lithium count
    fn update_soc(&mut self) {
        self.state_of_charge = if self.lithium_capacity > 0 {
            self.lithium_count as f32 / self.lithium_capacity as f32
        } else {
            0.0
        };
    }
    
    /// Attempt to intercalate one Li atom
    /// Returns true if successful, false if at capacity
    pub fn intercalate(&mut self) -> bool {
        if self.lithium_count >= self.lithium_capacity {
            return false;
        }
        
        self.lithium_count += 1;
        self.total_intercalated += 1;
        self.update_soc();
        true
    }
    
    /// Attempt to deintercalate one Li atom
    /// Returns true if successful, false if empty
    pub fn deintercalate(&mut self) -> bool {
        if self.lithium_count == 0 {
            return false;
        }
        
        self.lithium_count -= 1;
        self.total_deintercalated += 1;
        self.update_soc();
        true
    }
    
    /// Get color for visualization based on current SOC
    pub fn current_color(&self) -> [u8; 4] {
        self.material.color_at_soc(self.state_of_charge)
    }

    /// Check if fully lithiated
    pub fn is_full(&self) -> bool {
        self.lithium_count >= self.lithium_capacity
    }

    /// Check if fully delithiated
    pub fn is_empty(&self) -> bool {
        self.lithium_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intercalation_limits() {
        // Graphite site_density = 0.1 → capacity = 100 * 0.1 = 10
        let mut region = ActiveMaterialRegion::new(MaterialType::Graphite, 100.0);
        let capacity = region.lithium_capacity;
        assert_eq!(capacity, 10);
        assert!(region.is_empty());

        for i in 0..capacity {
            assert!(region.intercalate(), "Failed at i={}", i);
        }
        assert!(!region.intercalate());
        assert!(region.is_full());
    }

    #[test]
    fn test_deintercalation_limits() {
        // LFP site_density = 0.05 → capacity = 100 * 0.05 = 5
        let mut region = ActiveMaterialRegion::new(MaterialType::LFP, 100.0)
            .with_initial_soc(1.0);
        let capacity = region.lithium_capacity;
        assert_eq!(capacity, 5);
        assert!(region.is_full());

        for _ in 0..capacity {
            assert!(region.deintercalate());
        }
        assert!(!region.deintercalate());
        assert!(region.is_empty());
    }

    #[test]
    fn test_soc_tracking() {
        // Graphite site_density = 0.1 → capacity = 1000 * 0.1 = 100
        let mut region = ActiveMaterialRegion::new(MaterialType::Graphite, 1000.0);
        assert_eq!(region.lithium_capacity, 100);
        assert_eq!(region.state_of_charge, 0.0);

        for _ in 0..50 {
            assert!(region.intercalate());
        }
        assert!((region.state_of_charge - 0.5).abs() < 0.01);

        for _ in 0..50 {
            assert!(region.intercalate());
        }
        assert!((region.state_of_charge - 1.0).abs() < 0.01);
    }
}
