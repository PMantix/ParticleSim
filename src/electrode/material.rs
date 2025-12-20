// electrode/material.rs
// Defines electrode material types and their electrochemical properties

use serde::{Deserialize, Serialize};

/// Role of the electrode in the cell
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ElectrodeRole {
    /// Negative electrode (oxidized during discharge)
    Anode,
    /// Positive electrode (reduced during discharge)
    Cathode,
}

/// Mechanism by which lithium is stored
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum StorageMechanism {
    /// Li⁺ + e⁻ → Li⁰ (metallic plating/stripping)
    Plating,
    /// Li⁺ + e⁻ + Host ↔ Li-Host (reversible insertion)
    Intercalation,
    /// Li + Host → LiₓHost (alloying with volume change)
    Alloying,
}

/// Known electrode material types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialType {
    /// Lithium metal (plating/stripping) - existing behavior
    LithiumMetal,
    /// Synthetic graphite anode
    Graphite,
    /// Hard carbon (disordered carbon) anode
    HardCarbon,
    /// Silicon oxide composite anode
    SiliconOxide,
    /// Lithium titanate (Li₄Ti₅O₁₂) anode
    LTO,
    /// Lithium iron phosphate (LiFePO₄) cathode
    LFP,
    /// Lithium manganese iron phosphate cathode
    LMFP,
    /// Nickel manganese cobalt oxide (various stoichiometries)
    NMC,
    /// Nickel cobalt aluminum oxide cathode
    NCA,
}

impl MaterialType {
    /// Get the electrode role for this material
    pub fn role(&self) -> ElectrodeRole {
        match self {
            MaterialType::LithiumMetal => ElectrodeRole::Anode, // Typically used as anode
            MaterialType::Graphite => ElectrodeRole::Anode,
            MaterialType::HardCarbon => ElectrodeRole::Anode,
            MaterialType::SiliconOxide => ElectrodeRole::Anode,
            MaterialType::LTO => ElectrodeRole::Anode,
            MaterialType::LFP => ElectrodeRole::Cathode,
            MaterialType::LMFP => ElectrodeRole::Cathode,
            MaterialType::NMC => ElectrodeRole::Cathode,
            MaterialType::NCA => ElectrodeRole::Cathode,
        }
    }

    /// Get the storage mechanism
    pub fn mechanism(&self) -> StorageMechanism {
        match self {
            MaterialType::LithiumMetal => StorageMechanism::Plating,
            MaterialType::SiliconOxide => StorageMechanism::Alloying,
            _ => StorageMechanism::Intercalation,
        }
    }

    /// Maximum lithium stoichiometry (Li per formula unit)
    /// For graphite: LiC₆ → 1.0
    /// For LFP: LiFePO₄ → 1.0
    /// For NMC: Li(Ni,Mn,Co)O₂ → ~1.0 practical
    pub fn max_stoichiometry(&self) -> f32 {
        match self {
            MaterialType::LithiumMetal => f32::INFINITY, // No limit
            MaterialType::Graphite => 1.0,      // LiC₆
            MaterialType::HardCarbon => 1.0,    // Approximate
            MaterialType::SiliconOxide => 2.5,  // Li₂.₅SiO (practical)
            MaterialType::LTO => 3.0,           // Li₄Ti₅O₁₂ → Li₇Ti₅O₁₂
            MaterialType::LFP => 1.0,           // LiFePO₄
            MaterialType::LMFP => 1.0,          // LiMn₀.₆Fe₀.₄PO₄
            MaterialType::NMC => 1.0,           // LiNiₓMnyCo_zO₂
            MaterialType::NCA => 1.0,           // LiNi₀.₈Co₀.₁₅Al₀.₀₅O₂
        }
    }

    /// Open circuit voltage (V vs Li/Li⁺) as function of state of charge
    /// SOC = 0.0 means fully delithiated, SOC = 1.0 means fully lithiated
    pub fn open_circuit_voltage(&self, soc: f32) -> f32 {
        let soc = soc.clamp(0.0, 1.0);
        match self {
            MaterialType::LithiumMetal => 0.0, // Reference electrode

            MaterialType::Graphite => {
                // Graphite has staging plateaus
                // Approximated as smooth curve
                if soc < 0.1 {
                    0.15 - 0.5 * soc
                } else if soc < 0.5 {
                    0.12 - 0.05 * (soc - 0.1)
                } else if soc < 0.9 {
                    0.10 - 0.07 * (soc - 0.5)
                } else {
                    0.07 - 0.06 * (soc - 0.9)
                }
            }

            MaterialType::HardCarbon => {
                // Sloping profile with low-voltage plateau
                0.8 * (1.0 - soc) + 0.02
            }

            MaterialType::SiliconOxide => {
                // Silicon has sloping profile, higher voltage than graphite
                0.4 * (1.0 - soc) + 0.1
            }

            MaterialType::LTO => {
                // Very flat at 1.55V (safe, no SEI formation)
                1.55 + 0.05 * (0.5 - soc)
            }

            MaterialType::LFP => {
                // Very flat plateau at 3.4V
                3.4 + 0.02 * (soc - 0.5)
            }

            MaterialType::LMFP => {
                // Two plateaus: Fe²⁺/Fe³⁺ at 3.5V, Mn²⁺/Mn³⁺ at 4.1V
                if soc < 0.6 {
                    3.5 + 0.02 * (soc / 0.6 - 0.5)
                } else {
                    4.1 + 0.02 * ((soc - 0.6) / 0.4 - 0.5)
                }
            }

            MaterialType::NMC => {
                // Sloping profile from ~4.3V to ~3.6V
                4.3 - 0.7 * soc
            }

            MaterialType::NCA => {
                // Similar to NMC but slightly different shape
                4.2 - 0.6 * soc
            }
        }
    }

    /// Desolvation energy barrier in eV
    /// This is the activation energy for Li⁺ to shed its solvent shell
    pub fn desolvation_barrier(&self) -> f32 {
        match self {
            MaterialType::LithiumMetal => 0.25, // Lower barrier for metal
            MaterialType::Graphite => 0.35,
            MaterialType::HardCarbon => 0.30,
            MaterialType::SiliconOxide => 0.30,
            MaterialType::LTO => 0.28,
            MaterialType::LFP => 0.40,
            MaterialType::LMFP => 0.42,
            MaterialType::NMC => 0.45,
            MaterialType::NCA => 0.45,
        }
    }

    /// Exchange current density (A/m²) for Butler-Volmer kinetics
    /// Higher = faster kinetics
    pub fn exchange_current(&self) -> f32 {
        match self {
            MaterialType::LithiumMetal => 10.0,  // Very fast
            MaterialType::Graphite => 2.0,
            MaterialType::HardCarbon => 3.0,
            MaterialType::SiliconOxide => 1.0,   // Slower due to diffusion
            MaterialType::LTO => 5.0,            // Fast kinetics
            MaterialType::LFP => 0.5,            // Slower 1D diffusion
            MaterialType::LMFP => 0.4,
            MaterialType::NMC => 1.5,
            MaterialType::NCA => 1.5,
        }
    }

    /// Color when fully lithiated [R, G, B, A]
    pub fn lithiated_color(&self) -> [u8; 4] {
        match self {
            MaterialType::LithiumMetal => [192, 192, 192, 255], // Silver
            MaterialType::Graphite => [218, 165, 32, 255],       // Gold (LiC₆)
            MaterialType::HardCarbon => [180, 140, 60, 255],     // Bronze
            MaterialType::SiliconOxide => [220, 180, 100, 255],  // Light gold
            MaterialType::LTO => [100, 140, 200, 255],           // Blue-gray
            MaterialType::LFP => [80, 180, 80, 255],             // Green (LiFePO₄)
            MaterialType::LMFP => [120, 160, 80, 255],           // Yellow-green
            MaterialType::NMC => [60, 60, 180, 255],             // Blue (reduced Ni)
            MaterialType::NCA => [80, 80, 200, 255],             // Blue
        }
    }

    /// Color when fully delithiated [R, G, B, A]
    pub fn delithiated_color(&self) -> [u8; 4] {
        match self {
            MaterialType::LithiumMetal => [255, 255, 0, 255],    // Ion color
            MaterialType::Graphite => [40, 40, 40, 255],          // Black (graphite)
            MaterialType::HardCarbon => [60, 60, 60, 255],        // Dark gray
            MaterialType::SiliconOxide => [120, 100, 80, 255],    // Brown
            MaterialType::LTO => [200, 200, 220, 255],            // Light gray
            MaterialType::LFP => [140, 140, 140, 255],            // Gray (FePO₄)
            MaterialType::LMFP => [160, 140, 120, 255],           // Tan
            MaterialType::NMC => [40, 40, 40, 255],               // Black (oxidized)
            MaterialType::NCA => [50, 50, 50, 255],               // Black
        }
    }

    /// Interpolate color based on state of charge
    pub fn color_at_soc(&self, soc: f32) -> [u8; 4] {
        let soc = soc.clamp(0.0, 1.0);
        let lith = self.lithiated_color();
        let delith = self.delithiated_color();
        
        [
            ((lith[0] as f32 * soc) + (delith[0] as f32 * (1.0 - soc))) as u8,
            ((lith[1] as f32 * soc) + (delith[1] as f32 * (1.0 - soc))) as u8,
            ((lith[2] as f32 * soc) + (delith[2] as f32 * (1.0 - soc))) as u8,
            lith[3], // Keep alpha constant
        ]
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            MaterialType::LithiumMetal => "Lithium Metal",
            MaterialType::Graphite => "Graphite",
            MaterialType::HardCarbon => "Hard Carbon",
            MaterialType::SiliconOxide => "Silicon Oxide (SiOₓ)",
            MaterialType::LTO => "LTO (Li₄Ti₅O₁₂)",
            MaterialType::LFP => "LFP (LiFePO₄)",
            MaterialType::LMFP => "LMFP (LiMn₀.₆Fe₀.₄PO₄)",
            MaterialType::NMC => "NMC (LiNiMnCoO₂)",
            MaterialType::NCA => "NCA (LiNiCoAlO₂)",
        }
    }

    /// Short chemical formula
    pub fn formula(&self) -> &'static str {
        match self {
            MaterialType::LithiumMetal => "Li",
            MaterialType::Graphite => "LiC₆",
            MaterialType::HardCarbon => "LiCₓ",
            MaterialType::SiliconOxide => "Li₂SiO",
            MaterialType::LTO => "Li₇Ti₅O₁₂",
            MaterialType::LFP => "LiFePO₄",
            MaterialType::LMFP => "LiMnFePO₄",
            MaterialType::NMC => "LiNMC",
            MaterialType::NCA => "LiNCA",
        }
    }
}

/// All available anode materials
pub const ANODE_MATERIALS: &[MaterialType] = &[
    MaterialType::LithiumMetal,
    MaterialType::Graphite,
    MaterialType::HardCarbon,
    MaterialType::SiliconOxide,
    MaterialType::LTO,
];

/// All available cathode materials  
pub const CATHODE_MATERIALS: &[MaterialType] = &[
    MaterialType::LFP,
    MaterialType::LMFP,
    MaterialType::NMC,
    MaterialType::NCA,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocv_in_valid_range() {
        for material in ANODE_MATERIALS.iter().chain(CATHODE_MATERIALS.iter()) {
            for soc in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let v = material.open_circuit_voltage(soc);
                assert!(v >= -0.1 && v <= 5.0, 
                    "{:?} at SOC={}: OCV={} out of range", material, soc, v);
            }
        }
    }

    #[test]
    fn test_color_interpolation() {
        let material = MaterialType::Graphite;
        let empty = material.color_at_soc(0.0);
        let full = material.color_at_soc(1.0);
        let half = material.color_at_soc(0.5);
        
        // Half should be between empty and full
        for i in 0..3 {
            let expected = (empty[i] as f32 + full[i] as f32) / 2.0;
            assert!((half[i] as f32 - expected).abs() < 2.0);
        }
    }
}
