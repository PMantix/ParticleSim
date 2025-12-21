// body/mod.rs
// Re-exports for the body module

mod electron;
pub mod redox;
mod types;
// Module declarations removed dead test files
pub mod foil;

pub use electron::*;
pub use types::*;
pub use redox::local_potential_from_charge;

#[cfg(test)]
#[path = "tests/foil_electron_limits.rs"]
mod foil_electron_limits;
#[cfg(test)]
#[path = "tests/foil_mass_and_inertia.rs"]
mod foil_mass_and_inertia;

#[cfg(test)]
#[path = "tests/foil_cohesion_and_overlap.rs"]
mod foil_cohesion_and_overlap;

#[cfg(test)]
#[path = "tests/ion_vs_anion.rs"]
mod ion_vs_anion;

#[cfg(test)]
#[path = "tests/anion.rs"]
mod anion;
