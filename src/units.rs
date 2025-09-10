//! Physical unit definitions and conversions.
//!
//! Base units:
//! - Length: angstrom (Å)
//! - Time: femtosecond (fs)
//! - Charge: elementary charge (e)
//! - Mass: atomic mass unit (amu)

/// Angstrom in meters.
pub const ANGSTROM: f64 = 1.0e-10;
/// Femtosecond in seconds.
pub const FEMTOSECOND: f64 = 1.0e-15;
/// Elementary charge in coulombs.
pub const ELEMENTARY_CHARGE: f64 = 1.602_176_634e-19;
/// Atomic mass unit in kilograms.
pub const AMU: f64 = 1.660_539_066_60e-27;

/// Energy of one simulation unit expressed in joules.
pub const ENERGY_JOULE: f64 = AMU * ANGSTROM * ANGSTROM / (FEMTOSECOND * FEMTOSECOND);
/// Convert electronvolts to simulation energy units.
pub const EV_TO_SIM: f64 = ELEMENTARY_CHARGE / ENERGY_JOULE;

/// Coulomb's constant in simulation units.
/// k = 8.987e9 N⋅m²/C² converted to [AMU⋅Å³/fs²⋅e²] 
/// Dimensional analysis: kg⋅m³/(s²⋅C²) → AMU⋅Å³/(fs²⋅e²)
pub const COULOMB_CONSTANT: f32 = (
    8.987_551_792_3e9 * ELEMENTARY_CHARGE * ELEMENTARY_CHARGE * FEMTOSECOND * FEMTOSECOND
        / (AMU * ANGSTROM * ANGSTROM * ANGSTROM)
) as f32;
