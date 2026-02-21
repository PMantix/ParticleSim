// simulation/eis.rs
// Electrochemical Impedance Spectroscopy (EIS) state machine with lock-in detection

use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// Shared EIS results accessible from the renderer thread
pub static EIS_RESULTS: Lazy<Mutex<EisSharedState>> =
    Lazy::new(|| Mutex::new(EisSharedState::default()));

#[derive(Clone, Debug, Default)]
pub struct EisSharedState {
    pub points: Vec<EisPoint>,
    pub current_freq_idx: usize,
    pub total_frequencies: usize,
    pub is_running: bool,
    /// Detailed progress info for UI
    pub phase: String,
    pub elapsed_fs: f32,
    pub needed_fs: f32,
    pub current_freq: f32,
    pub sample_count: usize,
    /// Foil group assignments (for UI display)
    pub group_a_ids: Vec<u64>,
    pub group_b_ids: Vec<u64>,
}

#[derive(Clone, Debug)]
pub struct EisConfig {
    pub amplitude: f32,
    pub frequencies: Vec<f32>,
    pub periods_per_freq: usize,
    pub settle_periods: usize,
}

impl EisConfig {
    /// Generate log-spaced frequencies from f_min to f_max.
    pub fn log_spaced_frequencies(
        f_min: f32,
        f_max: f32,
        points_per_decade: f32,
    ) -> Vec<f32> {
        let log_min = f_min.log10();
        let log_max = f_max.log10();
        let n = ((log_max - log_min) * points_per_decade).ceil() as usize;
        if n == 0 {
            return vec![f_min];
        }
        (0..=n)
            .map(|i| 10f32.powf(log_min + i as f32 * (log_max - log_min) / n as f32))
            .collect()
    }

    /// Compute total estimated sweep time in fs.
    pub fn estimated_total_fs(&self) -> f32 {
        let total_periods = (self.settle_periods + self.periods_per_freq) as f32;
        self.frequencies.iter().map(|f| total_periods / f).sum()
    }
}

pub struct EisState {
    pub config: EisConfig,
    pub current_freq_idx: usize,
    pub phase: EisPhase,
    pub t_start: f32,
    pub t_start_recording: f32,
    pub t_eis_start: f32, // absolute time when sweep began

    // Lock-in accumulators (running DFT at excitation frequency)
    pub v_sin_acc: f64,
    pub v_cos_acc: f64,
    pub i_sin_acc: f64,
    pub i_cos_acc: f64,
    pub sample_count: usize,

    // Results
    pub results: Vec<EisPoint>,

    // Foil group assignments and saved DC biases (foil_id -> saved dc_current)
    pub group_a_ids: Vec<u64>,
    pub group_b_ids: Vec<u64>,
    pub saved_dc_currents: std::collections::HashMap<u64, f32>,

    // Whether EIS is finished
    pub finished: bool,

    // Logging throttle
    log_counter: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EisPhase {
    Settling,
    Recording,
}

#[derive(Clone, Debug)]
pub struct EisPoint {
    pub frequency: f32,
    pub z_real: f64,
    pub z_imag: f64,
    pub magnitude: f64,
    pub phase_deg: f64,
}

impl EisState {
    pub fn new(
        config: EisConfig,
        group_a_ids: Vec<u64>,
        group_b_ids: Vec<u64>,
        saved_dc_currents: std::collections::HashMap<u64, f32>,
        start_time: f32,
    ) -> Self {
        let total = config.frequencies.len();
        let est_time = config.estimated_total_fs();
        eprintln!(
            "[EIS] Started sweep: {} frequencies from {:.2e} to {:.2e} (1/fs)",
            total,
            config.frequencies.first().unwrap_or(&0.0),
            config.frequencies.last().unwrap_or(&0.0),
        );
        eprintln!(
            "[EIS] Estimated total time: {:.0} fs ({:.1} ps), amplitude={:.4e}",
            est_time,
            est_time / 1000.0,
            config.amplitude,
        );
        eprintln!(
            "[EIS] Group A (+ perturbation): {:?}, Group B (- perturbation): {:?}",
            group_a_ids, group_b_ids,
        );
        eprintln!(
            "[EIS] Saved DC biases: {:?}",
            saved_dc_currents,
        );
        let state = Self {
            config,
            current_freq_idx: 0,
            phase: EisPhase::Settling,
            t_start: start_time,
            t_start_recording: start_time,
            t_eis_start: start_time,
            v_sin_acc: 0.0,
            v_cos_acc: 0.0,
            i_sin_acc: 0.0,
            i_cos_acc: 0.0,
            sample_count: 0,
            results: Vec::new(),
            group_a_ids,
            group_b_ids,
            saved_dc_currents,
            finished: false,
            log_counter: 0,
        };
        // Initialize shared state
        let mut shared = EIS_RESULTS.lock();
        shared.points.clear();
        shared.current_freq_idx = 0;
        shared.total_frequencies = total;
        shared.is_running = true;
        shared.phase = "Settling".to_string();
        shared.elapsed_fs = 0.0;
        shared.needed_fs = est_time;
        shared.current_freq = *state.config.frequencies.first().unwrap_or(&0.0);
        shared.sample_count = 0;
        shared.group_a_ids = state.group_a_ids.clone();
        shared.group_b_ids = state.group_b_ids.clone();
        state
    }

    /// Get the sinusoidal perturbation current for the current time.
    pub fn get_perturbation(&self, time: f32) -> f32 {
        if self.finished {
            return 0.0;
        }
        let freq = self.config.frequencies[self.current_freq_idx];
        let omega = 2.0 * std::f32::consts::PI * freq;
        self.config.amplitude * (omega * (time - self.t_start)).sin()
    }

    /// Record one simulation step's voltage and current data.
    /// Returns true if the entire sweep is complete.
    pub fn record_step(
        &mut self,
        cell_voltage: f32,
        applied_current: f32,
        time: f32,
        _dt: f32,
    ) -> bool {
        if self.finished {
            return true;
        }

        let freq = self.config.frequencies[self.current_freq_idx];
        let period = 1.0 / freq;
        let elapsed = (time - self.t_start) as f64;
        let cycles_elapsed = elapsed / period as f64;
        let total_needed_this_freq =
            (self.config.settle_periods + self.config.periods_per_freq) as f32 * period;

        // Periodic logging (every 10000 steps)
        self.log_counter += 1;
        if self.log_counter % 10000 == 1 {
            eprintln!(
                "[EIS] freq {}/{} ({:.2e} 1/fs) phase={:?} elapsed={:.0}fs/{:.0}fs cycles={:.2} samples={} V={:.4e} I={:.4e}",
                self.current_freq_idx + 1,
                self.config.frequencies.len(),
                freq,
                self.phase,
                time - self.t_start,
                total_needed_this_freq,
                cycles_elapsed,
                self.sample_count,
                cell_voltage,
                applied_current,
            );
        }

        // Update shared state for GUI progress
        {
            let mut shared = EIS_RESULTS.lock();
            shared.elapsed_fs = time - self.t_eis_start;
            shared.current_freq = freq;
            shared.sample_count = self.sample_count;
            shared.phase = match self.phase {
                EisPhase::Settling => format!(
                    "Settling ({:.0}/{:.0} fs)",
                    time - self.t_start,
                    self.config.settle_periods as f32 * period
                ),
                EisPhase::Recording => format!(
                    "Recording ({:.0}/{:.0} fs, {} samples)",
                    time - self.t_start_recording,
                    self.config.periods_per_freq as f32 * period,
                    self.sample_count
                ),
            };
        }

        match self.phase {
            EisPhase::Settling => {
                if cycles_elapsed >= self.config.settle_periods as f64 {
                    // Transition to recording
                    self.phase = EisPhase::Recording;
                    self.t_start_recording = time;
                    self.v_sin_acc = 0.0;
                    self.v_cos_acc = 0.0;
                    self.i_sin_acc = 0.0;
                    self.i_cos_acc = 0.0;
                    self.sample_count = 0;
                    eprintln!(
                        "[EIS] Freq {}/{} ({:.2e}): settling complete, now recording",
                        self.current_freq_idx + 1,
                        self.config.frequencies.len(),
                        freq,
                    );
                }
            }
            EisPhase::Recording => {
                // Lock-in accumulation
                let omega = 2.0 * std::f64::consts::PI * freq as f64;
                let t_rel = (time - self.t_start_recording) as f64;
                let sin_val = (omega * t_rel).sin();
                let cos_val = (omega * t_rel).cos();

                let v = cell_voltage as f64;
                let i = applied_current as f64;

                self.v_sin_acc += v * sin_val;
                self.v_cos_acc += v * cos_val;
                self.i_sin_acc += i * sin_val;
                self.i_cos_acc += i * cos_val;
                self.sample_count += 1;

                // Check if we've recorded enough cycles
                let recording_cycles =
                    ((time - self.t_start_recording) as f64) / period as f64;
                if recording_cycles >= self.config.periods_per_freq as f64 {
                    self.finish_frequency_point();
                    return self.advance_to_next_frequency(time);
                }
            }
        }
        false
    }

    fn finish_frequency_point(&mut self) {
        if self.sample_count == 0 {
            return;
        }
        let n = self.sample_count as f64;
        // Extract complex amplitudes via lock-in (factor of 2/N for single-sided)
        let v_re = self.v_cos_acc * 2.0 / n;
        let v_im = self.v_sin_acc * 2.0 / n;
        let i_re = self.i_cos_acc * 2.0 / n;
        let i_im = self.i_sin_acc * 2.0 / n;

        // Z = V / I  (complex division)
        let denom = i_re * i_re + i_im * i_im;
        let (z_real, z_imag) = if denom > 1e-30 {
            (
                (v_re * i_re + v_im * i_im) / denom,
                (v_im * i_re - v_re * i_im) / denom,
            )
        } else {
            (0.0, 0.0)
        };

        let magnitude = (z_real * z_real + z_imag * z_imag).sqrt();
        let phase_deg = z_imag.atan2(z_real).to_degrees();

        let freq = self.config.frequencies[self.current_freq_idx];
        eprintln!(
            "[EIS] Completed freq {}/{}: {:.2e} 1/fs -> Z = ({:.4e}, {:.4e}), |Z|={:.4e}, phase={:.1} deg, {} samples",
            self.current_freq_idx + 1,
            self.config.frequencies.len(),
            freq,
            z_real,
            z_imag,
            magnitude,
            phase_deg,
            self.sample_count,
        );

        let point = EisPoint {
            frequency: freq,
            z_real,
            z_imag,
            magnitude,
            phase_deg,
        };
        self.results.push(point.clone());

        // Update shared state
        let mut shared = EIS_RESULTS.lock();
        shared.points.push(point);
        shared.current_freq_idx = self.current_freq_idx + 1;
    }

    /// Advance to the next frequency. Returns true if sweep is complete.
    fn advance_to_next_frequency(&mut self, time: f32) -> bool {
        self.current_freq_idx += 1;
        if self.current_freq_idx >= self.config.frequencies.len() {
            self.finished = true;
            eprintln!("[EIS] Sweep complete! {} points collected", self.results.len());
            let mut shared = EIS_RESULTS.lock();
            shared.is_running = false;
            return true;
        }
        // Reset for next frequency
        self.phase = EisPhase::Settling;
        self.t_start = time;
        self.v_sin_acc = 0.0;
        self.v_cos_acc = 0.0;
        self.i_sin_acc = 0.0;
        self.i_cos_acc = 0.0;
        self.sample_count = 0;
        false
    }
}
