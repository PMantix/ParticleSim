use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Simple scoped profiler recording cumulative time per section.
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

    pub fn print_and_clear(&mut self) {
        for (name, dur) in self.report_sorted() {
            println!("{:<20} {:?}", name, dur);
        }
        self.clear();
    }
}

pub struct ProfilerGuard {
    name: &'static str,
    start: Instant,
}

/// Start a profiling section. Returns a guard that will update the global
/// profiler when dropped.
pub fn start(name: &'static str) -> ProfilerGuard {
    ProfilerGuard { name, start: Instant::now() }
}

#[cfg(feature = "profiling")]
impl Drop for ProfilerGuard {
    fn drop(&mut self) {
        crate::PROFILER.lock().finish(self);
    }
}

/// Macro helper to profile a scope only when the `profiling` feature is enabled.
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "profiling")]
        let _guard = $crate::profiler::start($name);
    };
}
