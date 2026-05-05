// simulation/morphology_log.rs
//
// Phase 4.2 of docs/EIS_AMPLITUDE_STUDY_PLAN.md.
//
// Per-frame morphology-metrics CSV writer. Hooks into the simulation step
// loop guarded by `frame % log_every == 0`. Pure plumbing — the metrics
// themselves live in `morphology.rs`.
//
// CSV schema:
//   frame,time_fs,arc_length_norm,roughness_rms,dead_li_frac,accessible_atoms

use crate::body::Body;
use crate::simulation::morphology::{compute_morphology_metrics, MorphologyMetrics};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// Per-run morphology CSV writer state.
pub struct MorphologyLogger {
    pub path: PathBuf,
    pub log_every_frames: usize,
    file: BufWriter<File>,
    /// Last `frame` value at which a row was written. Initialised to None so
    /// the first eligible frame is logged.
    last_logged_frame: Option<usize>,
}

impl MorphologyLogger {
    /// Open the log file at `path`, creating directories as needed and
    /// writing the CSV header.
    pub fn open(path: PathBuf, log_every_frames: usize) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let f = File::create(&path)?;
        let mut buf = BufWriter::new(f);
        writeln!(
            buf,
            "frame,time_fs,arc_length_norm,roughness_rms,dead_li_frac,accessible_atoms"
        )?;
        Ok(Self {
            path,
            log_every_frames: log_every_frames.max(1),
            file: buf,
            last_logged_frame: None,
        })
    }

    /// Compute and write a row if the frame is due. Returns the metrics that
    /// were written, so callers can publish them to a live snapshot for GUI
    /// display, or `None` if no row was written this call.
    pub fn write_if_due(
        &mut self,
        frame: usize,
        time_fs: f32,
        bodies: &[Body],
    ) -> Option<MorphologyMetrics> {
        // Always log frame 0 (first call) so a fresh-equilibrate baseline
        // appears in the CSV without waiting for one full stride.
        let due = match self.last_logged_frame {
            None => true,
            Some(last) => frame.saturating_sub(last) >= self.log_every_frames,
        };
        if !due {
            return None;
        }
        let metrics = compute_morphology_metrics(bodies);
        if let Err(e) = writeln!(
            self.file,
            "{},{:.3},{:.6},{:.6},{:.6},{}",
            frame,
            time_fs,
            metrics.interface_arc_length_per_unit_lateral,
            metrics.interface_roughness_rms_angstroms,
            metrics.dead_li_fraction,
            metrics.accessible_surface_atoms,
        ) {
            eprintln!("morphology_log: write failed: {e}");
            return None;
        }
        let _ = self.file.flush();
        self.last_logged_frame = Some(frame);
        Some(metrics)
    }
}

#[cfg(all(test, feature = "unit_tests"))]
mod tests {
    use super::*;
    use crate::body::Species;
    use ultraviolet::Vec2;

    fn flat_foil_column(x: f32, n: usize, species: Species) -> Vec<Body> {
        (0..n)
            .map(|i| {
                let y = -50.0 + (i as f32) * 2.0;
                let mut b = Body::new(
                    Vec2::new(x, y),
                    Vec2::zero(),
                    species.mass(),
                    species.radius(),
                    0.0,
                    species,
                );
                b.id = (i + 1) as u64;
                b
            })
            .collect()
    }

    #[test]
    fn morphology_logger_writes_first_frame() {
        let dir = std::env::temp_dir().join("particlesim_morphology_log_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("morphology.csv");
        let _ = std::fs::remove_file(&path);

        let mut logger = MorphologyLogger::open(path.clone(), 1000).unwrap();
        let bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);

        let res = logger.write_if_due(0, 0.0, &bodies);
        assert!(res.is_some(), "first call (frame 0) should always log");

        let res2 = logger.write_if_due(1, 5.0, &bodies);
        assert!(res2.is_none(), "frame 1 (< stride 1000) should not log");

        let res3 = logger.write_if_due(1000, 5000.0, &bodies);
        assert!(res3.is_some(), "frame 1000 (= stride) should log");

        // Drop logger to flush, then read.
        drop(logger);
        let contents = std::fs::read_to_string(&path).unwrap();
        // Header + 2 data rows.
        assert_eq!(contents.lines().count(), 3);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn morphology_logger_csv_matches_flat_baseline() {
        // Phase 4.2 acceptance: on a flat validation scenario, the first
        // logged row should have arc_length_norm ≈ 1.0 and roughness_rms < 5.
        let dir = std::env::temp_dir().join("particlesim_morphology_log_baseline");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("morphology_baseline.csv");
        let _ = std::fs::remove_file(&path);

        let mut logger = MorphologyLogger::open(path.clone(), 100).unwrap();

        // Flat 2-foil baseline: same bodies as the morphology integration
        // tests use for arc_length_one_for_flat_foils.
        let mut bodies = flat_foil_column(-150.0, 50, Species::FoilMetal);
        bodies.extend(flat_foil_column(150.0, 50, Species::FoilMetal));

        let metrics = logger.write_if_due(0, 0.0, &bodies).unwrap();
        drop(logger);

        // Acceptance criteria from EIS_AMPLITUDE_STUDY_PLAN.md Phase 4.2:
        assert!(
            (metrics.interface_arc_length_per_unit_lateral - 1.0).abs() < 0.05,
            "flat baseline arc_length should be ≈ 1.0, got {}",
            metrics.interface_arc_length_per_unit_lateral
        );
        assert!(
            metrics.interface_roughness_rms_angstroms < 5.0,
            "flat baseline roughness should be < 5 Å, got {}",
            metrics.interface_roughness_rms_angstroms
        );

        // Also verify the CSV row parsed back has the same numbers (catches
        // formatting bugs in writeln! and the header/row alignment).
        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "expected header + 1 data row");
        let cols: Vec<&str> = lines[1].split(',').collect();
        assert_eq!(cols[0], "0", "frame");
        let arc_csv: f32 = cols[2].parse().unwrap();
        let roughness_csv: f32 = cols[3].parse().unwrap();
        assert!((arc_csv - 1.0).abs() < 0.05);
        assert!(roughness_csv < 5.0);

        let _ = std::fs::remove_file(&path);
    }
}
