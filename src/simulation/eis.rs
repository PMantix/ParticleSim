// simulation/eis.rs
// Electrochemical Impedance Spectroscopy (EIS) state machine with lock-in detection

use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// Shared EIS results accessible from the renderer thread
pub static EIS_RESULTS: Lazy<Mutex<EisSharedState>> =
    Lazy::new(|| Mutex::new(EisSharedState::default()));

/// Maximum time-series samples stored per frequency for live visualization.
const TS_MAX_SAMPLES: usize = 2000;

#[derive(Clone, Debug, Default)]
pub struct EisSharedState {
    pub points: Vec<EisPoint>,
    pub current_freq_idx: usize,
    pub total_frequencies: usize,
    pub is_running: bool,
    pub mode: EisMode,
    /// Detailed progress info for UI
    pub phase: String,
    pub elapsed_fs: f32,
    pub needed_fs: f32,
    pub current_freq: f32,
    pub sample_count: usize,
    /// Foil group assignments (for UI display)
    pub group_a_ids: Vec<u64>,
    pub group_b_ids: Vec<u64>,
    /// Live time-series for V and I (reset each frequency).
    /// `ts_t_rel` is time relative to the current frequency's t_start (fs).
    /// `ts_is_recording` marks whether the sample fell in the recording phase.
    pub ts_t_rel: Vec<f32>,
    pub ts_v: Vec<f32>,
    pub ts_i: Vec<f32>,
    pub ts_is_recording: Vec<bool>,
    /// Running best-fit sinusoid for the *response* signal.
    /// Galvanostatic: fitted to V.  Potentiostatic: fitted to I.
    /// fit(t) = fit_response_dc + fit_response_re·cos(ω·t) + fit_response_im·sin(ω·t)
    /// where t is relative to the start of the recording phase.
    pub fit_response_re: f64,
    pub fit_response_im: f64,
    pub fit_response_dc: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum EisMode {
    #[default]
    /// Galvanostatic: apply sinusoidal current, measure voltage.
    Galvanostatic,
    /// Potentiostatic: apply sinusoidal overpotential target, measure current.
    Potentiostatic,
}

#[derive(Clone, Debug)]
pub struct EisConfig {
    pub amplitude: f32,
    pub frequencies: Vec<f32>,
    pub periods_per_freq: usize,
    pub settle_periods: usize,
    pub mode: EisMode,
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
    pub v_sq_acc: f64,   // sum of V²  — for AC variance
    pub v_sum_acc: f64,  // sum of V   — for DC offset
    pub i_sum_acc: f64,  // sum of I   — for DC offset (used by 3-param LS fit)
    pub sample_count: usize,

    // Basis-function accumulators for 3-parameter LS fit.
    // The simple 2/N formula assumes Σcos²=N/2, Σsin²=N/2, Σcos·sin=0 — true only for
    // integer periods.  Accumulating these sums lets us solve the exact normal equations
    // and get correct amplitudes for any (fractional) number of recorded periods.
    pub cos_sq_acc: f64,   // Σ cos²(ωt)
    pub sin_sq_acc: f64,   // Σ sin²(ωt)
    pub cos_sin_acc: f64,  // Σ cos(ωt)·sin(ωt)
    pub cos_sum_acc: f64,  // Σ cos(ωt)
    pub sin_sum_acc: f64,  // Σ sin(ωt)

    // Results
    pub results: Vec<EisPoint>,

    pub mode: EisMode,

    // Foil group assignments
    pub group_a_ids: Vec<u64>,
    pub group_b_ids: Vec<u64>,
    /// Galvanostatic: saved dc_current per foil (restored after sweep).
    pub saved_dc_currents: std::collections::HashMap<u64, f32>,
    /// Potentiostatic: saved target_ratio per foil (restored after sweep).
    pub saved_target_ratios: std::collections::HashMap<u64, f32>,

    // Whether EIS is finished
    pub finished: bool,

    // Logging throttle
    log_counter: usize,

    // Time-series subsampling: record one sample every ts_subsample steps
    ts_subsample: usize,
    ts_step_counter: usize,
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
    /// R² of the sinusoidal fit: fraction of V variance explained by the fundamental.
    /// 1.0 = perfect fit; < 1.0 indicates harmonics, noise, or nonlinear response.
    pub fit_r2: f64,
}

impl EisState {
    pub fn new(
        config: EisConfig,
        group_a_ids: Vec<u64>,
        group_b_ids: Vec<u64>,
        saved_dc_currents: std::collections::HashMap<u64, f32>,
        saved_target_ratios: std::collections::HashMap<u64, f32>,
        start_time: f32,
    ) -> Self {
        let total = config.frequencies.len();
        let est_time = config.estimated_total_fs();
        let mode = config.mode;
        eprintln!(
            "[EIS] Started sweep ({:?}): {} frequencies from {:.2e} to {:.2e} (1/fs)",
            mode,
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
        let state = Self {
            mode,
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
            v_sq_acc: 0.0,
            v_sum_acc: 0.0,
            i_sum_acc: 0.0,
            sample_count: 0,
            cos_sq_acc: 0.0,
            sin_sq_acc: 0.0,
            cos_sin_acc: 0.0,
            cos_sum_acc: 0.0,
            sin_sum_acc: 0.0,
            results: Vec::new(),
            group_a_ids,
            group_b_ids,
            saved_dc_currents,
            saved_target_ratios,
            finished: false,
            log_counter: 0,
            ts_subsample: 1,
            ts_step_counter: 0,
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
        shared.mode = state.mode;
        shared.group_a_ids = state.group_a_ids.clone();
        shared.group_b_ids = state.group_b_ids.clone();
        // Pre-allocate to full capacity to avoid incremental Vec reallocations
        shared.ts_t_rel = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.ts_v = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.ts_i = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.ts_is_recording = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.fit_response_re = 0.0;
        shared.fit_response_im = 0.0;
        shared.fit_response_dc = 0.0;
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
        dt: f32,
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

        // Compute subsample ratio once per frequency (targeting ~TS_MAX_SAMPLES display pts)
        if self.ts_step_counter == 0 && dt > 0.0 {
            let total_periods =
                (self.config.settle_periods + self.config.periods_per_freq) as f32;
            let total_steps_est = total_periods / (freq * dt);
            self.ts_subsample =
                ((total_steps_est / TS_MAX_SAMPLES as f32).ceil() as usize).max(1);
        }

        // Increment counter outside the lock so the lock is only acquired every
        // ts_subsample steps rather than every single simulation step.  This
        // eliminates the per-step mutex contention and format!/String allocation
        // that caused heap fragmentation and renderer choppiness at low frequencies.
        self.ts_step_counter += 1;
        if self.ts_step_counter % self.ts_subsample == 0 {
            // Build phase string outside the lock to minimise time holding it.
            let phase_str = match self.phase {
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

            let mut shared = EIS_RESULTS.lock();
            shared.elapsed_fs = time - self.t_eis_start;
            shared.current_freq = freq;
            shared.sample_count = self.sample_count;
            shared.phase = phase_str;

            if shared.ts_t_rel.len() < TS_MAX_SAMPLES {
                shared.ts_t_rel.push(time - self.t_start);
                shared.ts_v.push(cell_voltage);
                shared.ts_i.push(applied_current);
                shared.ts_is_recording.push(self.phase == EisPhase::Recording);
            }

            // Update running best-fit for the *response* signal.
            // Galvanostatic: response is V (Coulomb potential).
            // Potentiostatic: response is I (PID current output).
            if self.phase == EisPhase::Recording && self.sample_count > 0 {
                let n = self.sample_count as f64;
                let (sig_cos, sig_sin, sig_sum) = match self.mode {
                    EisMode::Galvanostatic => (self.v_cos_acc, self.v_sin_acc, self.v_sum_acc),
                    EisMode::Potentiostatic => (self.i_cos_acc, self.i_sin_acc, self.i_sum_acc),
                };
                let (fit_re, fit_im) = self.ls_fit_re_im(sig_cos, sig_sin, sig_sum);
                // C_ls = (Σ signal − Σcos·A − Σsin·B) / N
                let fit_dc = (sig_sum - self.cos_sum_acc * fit_re - self.sin_sum_acc * fit_im) / n;
                shared.fit_response_re = fit_re;
                shared.fit_response_im = fit_im;
                shared.fit_response_dc = fit_dc as f32;
            }
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
                    self.v_sq_acc = 0.0;
                    self.v_sum_acc = 0.0;
                    self.i_sum_acc = 0.0;
                    self.sample_count = 0;
                    self.cos_sq_acc = 0.0;
                    self.sin_sq_acc = 0.0;
                    self.cos_sin_acc = 0.0;
                    self.cos_sum_acc = 0.0;
                    self.sin_sum_acc = 0.0;
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
                self.v_sq_acc += v * v;
                self.v_sum_acc += v;
                self.i_sum_acc += i;
                self.cos_sq_acc += cos_val * cos_val;
                self.sin_sq_acc += sin_val * sin_val;
                self.cos_sin_acc += cos_val * sin_val;
                self.cos_sum_acc += cos_val;
                self.sin_sum_acc += sin_val;
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

    /// Solve the 3-parameter least-squares sinusoidal fit:
    ///   signal(t) = A·cos(ωt) + B·sin(ωt) + C
    ///
    /// Returns (A, B) — the cosine and sine amplitudes — via Cramer's rule on the
    /// 3×3 normal-equation matrix built from the accumulated basis sums.
    ///
    /// The simple formula `2/N · Σ signal·cos(ωt)` assumes Σcos²=N/2 and Σcos·sin=0,
    /// which only holds for integer numbers of periods.  This method is exact for any
    /// window length: for integer periods the extra terms cancel and it reduces to 2/N.
    fn ls_fit_re_im(&self, sig_cos: f64, sig_sin: f64, sig_sum: f64) -> (f64, f64) {
        let n = self.sample_count as f64;
        if n < 3.0 {
            let s = if n > 0.0 { 2.0 / n } else { 0.0 };
            return (sig_cos * s, sig_sin * s);
        }

        // Build the 3×3 normal-equation matrix M:
        //   [[scc, scs, sc ],
        //    [scs, sss, ss ],
        //    [sc,  ss,  n  ]]
        // where scc=Σcos², sss=Σsin², scs=Σcos·sin, sc=Σcos, ss=Σsin.
        let scc = self.cos_sq_acc;
        let sss = self.sin_sq_acc;
        let scs = self.cos_sin_acc;
        let sc  = self.cos_sum_acc;
        let ss  = self.sin_sum_acc;

        // det(M) expanding along the first row
        let det = scc * (sss * n - ss * ss)
                - scs * (scs * n - ss * sc)
                + sc  * (scs * ss - sss * sc);

        // Guard against degenerate matrices (all-zero or near-singular).
        // det scales as O(N³); use a relative threshold.
        if det.abs() < 1e-10 * n * n * n {
            return (sig_cos * 2.0 / n, sig_sin * 2.0 / n);
        }

        // Cramer's rule — replace first column with RHS to get A (cosine component)
        let a = (sig_cos * (sss * n  - ss  * ss)
               - scs    * (sig_sin * n - ss  * sig_sum)
               + sc     * (sig_sin * ss  - sss * sig_sum)) / det;

        // Cramer's rule — replace second column with RHS to get B (sine component)
        let b = (scc     * (sig_sin * n - ss  * sig_sum)
               - sig_cos * (scs * n     - ss  * sc)
               + sc      * (scs * sig_sum - sig_sin * sc)) / det;

        (a, b)
    }

    fn finish_frequency_point(&mut self) {
        if self.sample_count == 0 {
            return;
        }
        let n = self.sample_count as f64;
        // Extract complex amplitudes via 3-parameter least-squares fit.
        // This is correct for any (fractional) number of recorded periods, unlike the
        // simple 2/N formula which assumes exact orthogonality (integer periods only).
        let (v_re, v_im) = self.ls_fit_re_im(self.v_cos_acc, self.v_sin_acc, self.v_sum_acc);
        let (i_re, i_im) = self.ls_fit_re_im(self.i_cos_acc, self.i_sin_acc, self.i_sum_acc);

        // R² of the V fit: fundamental power as a fraction of AC (DC-subtracted) variance.
        //   P_fit    = (v_re² + v_im²) / 2   — RMS² of the best-fit sinusoid
        //   V_dc     = v_sum_acc / N          — mean (DC offset)
        //   P_ac     = mean(V²) - V_dc²       — AC variance (total power about the mean)
        //   R²       = P_fit / P_ac
        // Using AC variance instead of raw total power prevents the DC offset from
        // artificially deflating R² when the signal rides on a large bias.
        let p_fit = (v_re * v_re + v_im * v_im) * 0.5;
        let v_dc = self.v_sum_acc / n;
        let p_ac = (self.v_sq_acc / n - v_dc * v_dc).max(0.0);
        let fit_r2 = if p_ac > 1e-60 { (p_fit / p_ac).min(1.0) } else { 1.0 };

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
            fit_r2,
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
        self.v_sq_acc = 0.0;
        self.v_sum_acc = 0.0;
        self.i_sum_acc = 0.0;
        self.sample_count = 0;
        self.cos_sq_acc = 0.0;
        self.sin_sq_acc = 0.0;
        self.cos_sin_acc = 0.0;
        self.cos_sum_acc = 0.0;
        self.sin_sum_acc = 0.0;
        self.ts_step_counter = 0;
        self.ts_subsample = 1;
        // Clear time-series buffer and fit state for the new frequency
        let mut shared = EIS_RESULTS.lock();
        shared.ts_t_rel.clear();
        shared.ts_v.clear();
        shared.ts_i.clear();
        shared.ts_is_recording.clear();
        shared.fit_response_re = 0.0;
        shared.fit_response_im = 0.0;
        shared.fit_response_dc = 0.0;
        false
    }
}
