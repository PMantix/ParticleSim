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
            Self {
                timings: HashMap::new(),
            }
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
        pub fn print_and_clear(
            &mut self,
            sim: Option<&crate::simulation::Simulation>,
            fps: Option<f32>,
        ) {
            use std::collections::HashSet;
            let sorted = self.report_sorted();
            if sorted.is_empty() {
                return;
            }

            // Clear screen and move cursor to top-left
            print!("\x1B[2J\x1B[H");
            use std::io::{self, Write};
            io::stdout().flush().ok();

            // Organize profiling sections by folder structure
            let expected_sections = [
                // Simulation core
                ("simulation_step", "simulation/"),
                ("iterate", "simulation/"),
                ("apply_redox", "simulation/"),
                // Forces
                ("forces_attract", "simulation/forces"),
                ("forces_polar", "simulation/forces"),
                ("forces_lj", "simulation/forces"),
                ("forces_repulsion", "simulation/forces"),
                // Collision detection
                ("collision", "simulation/"),
                // Spatial partitioning
                ("quadtree_build", "quadtree/"),
                ("quadtree_build_domain", "quadtree/"),
                ("quadtree_field", "quadtree/"),
                ("quadtree_neighbors", "quadtree/"),
                // App management
                ("command_handling", "app/commands"),
                ("particle_spawn", "app/spawn"),
                ("simulation_loop", "app/"),
                // Renderer
                ("gui_update", "renderer/gui"),
                ("draw_particles", "renderer/draw"),
                ("input_handling", "renderer/"),
                // File I/O
                ("save_state", "io/"),
                ("load_state", "io/"),
                // Configuration
                ("config_update", "config/"),
                // Diagnostics
                ("diagnostics_transference", "diagnostics/"),
                ("diagnostics_solvation", "diagnostics/"),
                ("diagnostics_foil_electron", "diagnostics/"),
                ("transference_calculation_internal", "diagnostics/"),
                ("solvation_calculation_internal", "diagnostics/"),
                ("foil_electron_calculation_internal", "diagnostics/"),
                // Electron updates
                ("electron_updates", "body/"),
            ];
            let mut seen = HashSet::new();
            let mut total: f64 = 0.0;
            let mut section_map = std::collections::HashMap::new();
            for (name, dur) in &sorted {
                section_map.insert(*name, *dur);
                total += dur.as_secs_f64() * 1000.0;
            }

            // Group sections by folder for organized display
            let mut folder_groups = std::collections::HashMap::new();
            for &(section, folder) in &expected_sections {
                let ms = section_map
                    .get(section)
                    .map(|d| d.as_secs_f64() * 1000.0)
                    .unwrap_or(0.0);
                if ms > 0.0 {
                    folder_groups
                        .entry(folder)
                        .or_insert_with(Vec::new)
                        .push((section, ms));
                    seen.insert(section);
                }
            }

            println!("=== PARTICLE SIMULATION PROFILER ===");

            // Display by folder structure
            let folder_order = [
                ("simulation/", "Simulation Core"),
                ("simulation/forces", "Force Calculations"),
                ("quadtree/", "Spatial Partitioning"),
                ("body/", "Body Updates"),
                ("app/", "App Management"),
                ("app/commands", "Command Processing"),
                ("app/spawn", "Particle Spawning"),
                ("renderer/", "Rendering"),
                ("renderer/gui", "GUI Updates"),
                ("renderer/draw", "Draw Operations"),
                ("diagnostics/", "Diagnostics"),
                ("io/", "File I/O"),
                ("config/", "Configuration"),
            ];

            let mut any_sections = false;
            for (folder, display_name) in &folder_order {
                if let Some(sections) = folder_groups.get(folder) {
                    if !any_sections {
                        println!("\n{:<20} {:>10}   {:>8}", "Section", "Time", "% Step");
                        println!("{}", "=".repeat(42));
                        any_sections = true;
                    }
                    println!("\n{}", display_name);
                    for &(section, ms) in sections {
                        let percent = if total > 0.0 { 100.0 * ms / total } else { 0.0 };
                        println!("  {:<18} {:>8.2}ms   {:>6.1}%", section, ms, percent);
                    }
                }
            }

            // Print any extra sections not in expected list
            let mut has_other = false;
            for (name, dur) in &sorted {
                if !seen.contains(name) {
                    if !has_other {
                        println!("\nOther Sections");
                        has_other = true;
                    }
                    let ms = dur.as_secs_f64() * 1000.0;
                    let percent = if total > 0.0 { 100.0 * ms / total } else { 0.0 };
                    println!("  {:<18} {:>8.2}ms   {:>6.1}%", name, ms, percent);
                }
            }

            // Summary line
            println!("\n{}", "=".repeat(42));
            println!("{:<20} {:>8.2}ms   100.0%", "TOTAL", total);

            // Print simulation stats if provided
            if let Some(sim) = sim {
                let total_particles = sim.bodies.len();
                let num_ions = sim
                    .bodies
                    .iter()
                    .filter(|b| matches!(b.species, crate::body::Species::LithiumIon))
                    .count();
                let num_metals = sim
                    .bodies
                    .iter()
                    .filter(|b| {
                        matches!(
                            b.species,
                            crate::body::Species::LithiumMetal | crate::body::Species::FoilMetal
                        )
                    })
                    .count();
                let num_electrons: usize = sim.bodies.iter().map(|b| b.electrons.len()).sum();
                let num_foils = sim.foils.len();
                println!(
                    "\nSimulation: {} particles | {} ions | {} metals | {} electrons | {} foils",
                    total_particles, num_ions, num_metals, num_electrons, num_foils
                );
            }

            // Print FPS if provided
            if let Some(fps) = fps {
                println!("Performance: {:.1} FPS", fps);
            }

            println!(); // Add a blank line at the end
            self.clear();
        }
        pub fn print_and_clear_if_running(
            &mut self,
            running: bool,
            sim: Option<&crate::simulation::Simulation>,
            fps: Option<f32>,
        ) {
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
        ProfilerGuard {
            name,
            start: Instant::now(),
        }
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
pub fn start(_: &'static str) -> ProfilerGuard {
    ProfilerGuard
}

#[cfg(not(feature = "profiling"))]
#[allow(dead_code, unused_imports, unused_variables)]
impl Profiler {
    pub fn new() -> Self {
        Profiler
    }
    pub fn finish(&mut self, _: &ProfilerGuard) {}
    pub fn report_sorted(&self) -> Vec<(&'static str, std::time::Duration)> {
        vec![]
    }
    pub fn clear(&mut self) {}
    pub fn print_and_clear(&mut self, _: Option<&crate::simulation::Simulation>, _: Option<f32>) {}
    pub fn print_and_clear_if_running(
        &mut self,
        _: bool,
        _: Option<&crate::simulation::Simulation>,
        _: Option<f32>,
    ) {
    }
}

#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "profiling")]
        let _guard = $crate::profiler::start($name);
    };
}
