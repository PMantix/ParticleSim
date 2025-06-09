#![cfg_attr(feature = "profiling", allow(dead_code, unused_imports))]

#[cfg(feature = "profiling")]
mod profiling_impl {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    pub struct Profiler {
        pub timings: HashMap<&'static str, Duration>,
    }

    impl Profiler {
        pub fn new() -> Self {
            Self { timings: HashMap::new() }
        }
        pub fn finish(&mut self, guard: &ProfilerGuard) {
            let elapsed = guard.start.elapsed();
            *self.timings.entry(guard.name).or_default() += elapsed;
        }
        pub fn report_sorted(&self) -> Vec<(&'static str, Duration)> {
            let mut v: Vec<_> = self.timings.iter().map(|(n, d)| (*n, *d)).collect();
            v.sort_by(|a, b| b.1.cmp(&a.1));
            v
        }
        pub fn clear(&mut self) {
            self.timings.clear();
        }
        pub fn print_and_clear(&mut self, sim: Option<&crate::simulation::Simulation>, fps: Option<f32>) {
            use std::collections::HashSet;
            let sorted = self.report_sorted();
            if sorted.is_empty() {
                return;
            }
            // Move cursor to top-left (no full clear)
            print!("\x1B[H");
            use std::io::{self, Write};
            io::stdout().flush().ok();

            // List of expected section names (add/remove as needed)
            let expected_sections = [
                "simulation_step",
                "forces_lj",
                "quadtree_neighbors",
                "quadtree_build",
                "forces_attract",
                "collision",
                "quadtree_field",
                "iterate",
            ];
            let mut seen = HashSet::new();
            let mut total: f64 = 0.0;
            let mut section_map = std::collections::HashMap::new();
            for (name, dur) in &sorted {
                section_map.insert(*name, *dur);
                total += dur.as_secs_f64() * 1000.0;
            }
            println!("\nEnd of step profile summary:");
            println!("{:<20} {:>10}   {:>8}", "Section", "Time", "% of step");
            for &section in &expected_sections {
                let ms = section_map.get(section).map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0);
                let percent = if total > 0.0 { 100.0 * ms / total } else { 0.0 };
                println!("{:<20} {:>8.4}ms   {:>6.2}%", section, ms, percent);
                seen.insert(section);
            }
            // Print any extra sections not in expected list
            for (name, dur) in &sorted {
                if !seen.contains(name) {
                    let ms = dur.as_secs_f64() * 1000.0;
                    let percent = if total > 0.0 { 100.0 * ms / total } else { 0.0 };
                    println!("{:<20} {:>8.4}ms   {:>6.2}%", name, ms, percent);
                }
            }
            println!("{:<20} {:>8.4}ms   100.00%", "TOTAL", total);

            // Print simulation stats if provided
            if let Some(sim) = sim {
                let total_particles = sim.bodies.len();
                let num_ions = sim.bodies.iter().filter(|b| matches!(b.species, crate::body::Species::LithiumIon)).count();
                let num_metals = sim.bodies.iter().filter(|b| matches!(b.species, crate::body::Species::LithiumMetal | crate::body::Species::FoilMetal)).count();
                let num_electrons: usize = sim.bodies.iter().map(|b| b.electrons.len()).sum();
                let num_foils = sim.foils.len();
                println!("\nParticles: {}   Ions: {}   Metals: {}   Electrons: {}   Foils: {}", total_particles, num_ions, num_metals, num_electrons, num_foils);
            } else {
                println!("\nParticles: 0   Ions: 0   Metals: 0   Electrons: 0   Foils: 0");
            }
            // Print FPS if provided
            if let Some(fps) = fps {
                println!("FPS: {:.1}", fps);
            }
            self.clear();
        }
        pub fn print_and_clear_if_running(&mut self, running: bool, sim: Option<&crate::simulation::Simulation>, fps: Option<f32>) {
            if running {
                self.print_and_clear(sim, fps);
            }
            // If not running, do not print or clear; keep timings for inspection
        }
    }

    pub struct ProfilerGuard {
        pub(crate) name: &'static str,
        pub(crate) start: Instant,
    }

    pub fn start(name: &'static str) -> ProfilerGuard {
        ProfilerGuard { name, start: Instant::now() }
    }

    impl Drop for ProfilerGuard {
        fn drop(&mut self) {
            crate::PROFILER.lock().finish(self);
        }
    }
}

#[cfg(feature = "profiling")]
pub use profiling_impl::*;

#[cfg(not(feature = "profiling"))]
#[allow(dead_code, unused_imports, unused_variables)]
pub struct Profiler;

#[cfg(not(feature = "profiling"))]
#[allow(dead_code, unused_imports, unused_variables)]
pub struct ProfilerGuard;

#[cfg(not(feature = "profiling"))]
#[allow(dead_code, unused_imports, unused_variables)]
pub fn start(_: &'static str) -> ProfilerGuard { ProfilerGuard }

#[cfg(not(feature = "profiling"))]
#[allow(dead_code, unused_imports, unused_variables)]
impl Profiler {
    pub fn new() -> Self { Profiler }
    pub fn finish(&mut self, _: &ProfilerGuard) {}
    pub fn report_sorted(&self) -> Vec<(&'static str, std::time::Duration)> { vec![] }
    pub fn clear(&mut self) {}
    pub fn print_and_clear(&mut self, _: Option<&crate::simulation::Simulation>, _: Option<f32>) {}
    pub fn print_and_clear_if_running(&mut self, _: bool, _: Option<&crate::simulation::Simulation>, _: Option<f32>) {}
}

#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "profiling")]
        let _guard = $crate::profiler::start($name);
    };
}
