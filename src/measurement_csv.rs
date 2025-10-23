use std::collections::{HashMap, HashSet};
use std::io;
use std::path::PathBuf;

#[derive(Default)]
pub struct MeasurementCsv {
    resolved_paths: HashMap<String, PathBuf>,
    header_written: HashSet<String>,
}

impl MeasurementCsv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.resolved_paths.clear();
        self.header_written.clear();
    }

    pub fn resolve<F>(&mut self, key: &str, generator: F) -> io::Result<PathBuf>
    where
        F: FnOnce() -> io::Result<PathBuf>,
    {
        if let Some(path) = self.resolved_paths.get(key) {
            return Ok(path.clone());
        }
        let path = generator()?;
        self.resolved_paths.insert(key.to_string(), path.clone());
        Ok(path)
    }

    pub fn mark_header_written(&mut self, key: &str) {
        self.header_written.insert(key.to_string());
    }

    pub fn header_written(&self, key: &str) -> bool {
        self.header_written.contains(key)
    }
}
