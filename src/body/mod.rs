// body/mod.rs
// Re-exports for the body module

mod types;
mod electron;
mod redox;
// Module declarations removed dead test files
pub mod foil;

pub use types::*;
pub use electron::*;
//pub use redox::*;

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

