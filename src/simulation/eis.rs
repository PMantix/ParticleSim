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
    /// Repeat progress for UI display
    pub current_repeat: usize,
    pub total_repeats: usize,
    /// Foil group assignments (for UI display)
    pub group_a_ids: Vec<u64>,
    pub group_b_ids: Vec<u64>,
    /// Selected voltage probe body IDs (for visualization)
    pub probe_a_ids: Vec<u64>,
    pub probe_b_ids: Vec<u64>,
    /// Live time-series for V and I (reset each frequency).
    /// `ts_t_rel` is time relative to the current frequency's t_start (fs).
    /// `ts_is_recording` marks whether the sample fell in the recording phase.
    pub ts_t_rel: Vec<f32>,
    pub ts_v: Vec<f32>,
    pub ts_i: Vec<f32>,
    pub ts_is_recording: Vec<bool>,
    /// Actual electron current (discrete hopping rate) time series.
    pub ts_actual_i: Vec<f32>,
    /// Virtual capacitor voltage time series.
    pub ts_v_cap: Vec<f32>,
    /// Running best-fit sinusoid for the V (Coulomb potential) signal — both modes.
    /// fit_V(t) = fit_v_dc + fit_v_re·cos(ω·t) + fit_v_im·sin(ω·t)
    pub fit_v_re: f64,
    pub fit_v_im: f64,
    pub fit_v_dc: f32,
    /// Running best-fit sinusoid for the I signal — potentiostatic mode only.
    /// In galvanostatic mode I is the applied (known) sinusoid so this is zeroed.
    /// fit_I(t) = fit_i_dc + fit_i_re·cos(ω·t) + fit_i_im·sin(ω·t)
    pub fit_i_re: f64,
    pub fit_i_im: f64,
    pub fit_i_dc: f32,
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
    pub repeats_per_freq: usize,
    /// Number of spatial probe points per electrode for voltage averaging.
    /// 0 = use all foil bodies (original behaviour).
    pub voltage_probes: usize,
    /// Virtual capacitance for diagnostic integrator (default 1e-3).
    pub c_virtual: f64,
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
        let total_periods = (self.settle_periods + self.periods_per_freq) as f32
            * self.repeats_per_freq as f32;
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
    pub v_sq_acc: f64,   // sum of V²  — for AC variance (galvanostatic R²)
    pub i_sq_acc: f64,   // sum of I²  — for AC variance (potentiostatic R²)
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

    // Actual electron current lock-in accumulators
    pub actual_i_sin_acc: f64,
    pub actual_i_cos_acc: f64,
    pub actual_i_sq_acc: f64,
    pub actual_i_sum_acc: f64,

    // Virtual capacitor diagnostic
    pub v_cap: f64,        // running integral V_cap = ∫(I·dt / C_virtual)
    pub c_virtual: f64,    // virtual capacitance value
    pub vcap_sin_acc: f64,
    pub vcap_cos_acc: f64,
    pub vcap_sq_acc: f64,
    pub vcap_sum_acc: f64,

    // Results
    pub results: Vec<EisPoint>,

    // Repeat averaging state
    pub current_repeat: usize,
    pub repeat_buffer: Vec<EisPoint>,

    pub mode: EisMode,

    // Foil group assignments
    pub group_a_ids: Vec<u64>,
    pub group_b_ids: Vec<u64>,
    /// Fixed body IDs selected as voltage probe points for each group.
    /// Empty = use centroid (original single-probe behaviour).
    pub probe_a_ids: Vec<u64>,
    pub probe_b_ids: Vec<u64>,
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
    /// R² of the V (Coulomb potential) sinusoidal fit.
    pub fit_r2_v: f64,
    /// R² of the I sinusoidal fit.
    /// Galvanostatic: I is the applied signal so this is trivially ~1.0.
    /// Potentiostatic: I is a measured response so this reflects true fit quality.
    pub fit_r2_i: f64,
    /// Peak amplitude of the best-fit V sinusoid: sqrt(v_re² + v_im²).
    pub fit_v_amp: f64,
    /// Phase of the best-fit V sinusoid in degrees (cosine convention: V = A·cos(ωt + φ)).
    pub fit_v_phase_deg: f64,
    /// Peak amplitude of the best-fit I sinusoid: sqrt(i_re² + i_im²).
    pub fit_i_amp: f64,
    /// Phase of the best-fit I sinusoid in degrees (cosine convention: I = A·cos(ωt + φ)).
    pub fit_i_phase_deg: f64,
    /// Impedance from actual discrete electron current (Z_actual = V̂ / Î_actual).
    pub z_actual_real: f64,
    pub z_actual_imag: f64,
    /// Impedance from virtual capacitor (Z_cap = V̂_cap / Î_pid).
    pub z_cap_real: f64,
    pub z_cap_imag: f64,
}

impl EisState {
    pub fn new(
        config: EisConfig,
        group_a_ids: Vec<u64>,
        group_b_ids: Vec<u64>,
        probe_a_ids: Vec<u64>,
        probe_b_ids: Vec<u64>,
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
        eprintln!(
            "[EIS] Voltage probes: {} per group (A={}, B={})",
            config.voltage_probes,
            probe_a_ids.len(),
            probe_b_ids.len(),
        );
        let c_virtual = config.c_virtual;
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
            i_sq_acc: 0.0,
            v_sum_acc: 0.0,
            i_sum_acc: 0.0,
            sample_count: 0,
            cos_sq_acc: 0.0,
            sin_sq_acc: 0.0,
            cos_sin_acc: 0.0,
            cos_sum_acc: 0.0,
            sin_sum_acc: 0.0,
            actual_i_sin_acc: 0.0,
            actual_i_cos_acc: 0.0,
            actual_i_sq_acc: 0.0,
            actual_i_sum_acc: 0.0,
            v_cap: 0.0,
            c_virtual,
            vcap_sin_acc: 0.0,
            vcap_cos_acc: 0.0,
            vcap_sq_acc: 0.0,
            vcap_sum_acc: 0.0,
            results: Vec::new(),
            current_repeat: 0,
            repeat_buffer: Vec::new(),
            group_a_ids,
            group_b_ids,
            probe_a_ids,
            probe_b_ids,
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
        shared.current_repeat = 0;
        shared.total_repeats = state.config.repeats_per_freq;
        shared.mode = state.mode;
        shared.group_a_ids = state.group_a_ids.clone();
        shared.group_b_ids = state.group_b_ids.clone();
        shared.probe_a_ids = state.probe_a_ids.clone();
        shared.probe_b_ids = state.probe_b_ids.clone();
        // Pre-allocate to full capacity to avoid incremental Vec reallocations
        shared.ts_t_rel = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.ts_v = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.ts_i = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.ts_is_recording = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.ts_actual_i = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.ts_v_cap = Vec::with_capacity(TS_MAX_SAMPLES);
        shared.fit_v_re = 0.0;
        shared.fit_v_im = 0.0;
        shared.fit_v_dc = 0.0;
        shared.fit_i_re = 0.0;
        shared.fit_i_im = 0.0;
        shared.fit_i_dc = 0.0;
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
    /// `actual_electron_delta` is the net electron count change on group-A foils this step.
    /// Returns true if the entire sweep is complete.
    pub fn record_step(
        &mut self,
        cell_voltage: f32,
        applied_current: f32,
        actual_electron_delta: i32,
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
            shared.current_repeat = self.current_repeat;
            shared.phase = phase_str;

            if shared.ts_t_rel.len() < TS_MAX_SAMPLES {
                shared.ts_t_rel.push(time - self.t_start);
                shared.ts_v.push(cell_voltage);
                shared.ts_i.push(applied_current);
                shared.ts_is_recording.push(self.phase == EisPhase::Recording);
                let actual_i_rate = if dt > 0.0 { actual_electron_delta as f32 / dt } else { 0.0 };
                shared.ts_actual_i.push(actual_i_rate);
                shared.ts_v_cap.push(self.v_cap as f32);
            }

            // Update running best-fit for V (Coulomb potential) — both modes.
            // Potentiostatic also fits I since both are measured signals needed for Z = V̂/Î.
            if self.phase == EisPhase::Recording && self.sample_count > 0 {
                let n = self.sample_count as f64;

                let (v_re, v_im) = self.ls_fit_re_im(self.v_cos_acc, self.v_sin_acc, self.v_sum_acc);
                let v_dc = (self.v_sum_acc - self.cos_sum_acc * v_re - self.sin_sum_acc * v_im) / n;
                shared.fit_v_re = v_re;
                shared.fit_v_im = v_im;
                shared.fit_v_dc = v_dc as f32;

                if self.mode == EisMode::Potentiostatic {
                    let (i_re, i_im) = self.ls_fit_re_im(self.i_cos_acc, self.i_sin_acc, self.i_sum_acc);
                    let i_dc = (self.i_sum_acc - self.cos_sum_acc * i_re - self.sin_sum_acc * i_im) / n;
                    shared.fit_i_re = i_re;
                    shared.fit_i_im = i_im;
                    shared.fit_i_dc = i_dc as f32;
                } else {
                    shared.fit_i_re = 0.0;
                    shared.fit_i_im = 0.0;
                    shared.fit_i_dc = 0.0;
                }
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
                    self.i_sq_acc = 0.0;
                    self.v_sum_acc = 0.0;
                    self.i_sum_acc = 0.0;
                    self.sample_count = 0;
                    self.cos_sq_acc = 0.0;
                    self.sin_sq_acc = 0.0;
                    self.cos_sin_acc = 0.0;
                    self.cos_sum_acc = 0.0;
                    self.sin_sum_acc = 0.0;
                    self.actual_i_sin_acc = 0.0;
                    self.actual_i_cos_acc = 0.0;
                    self.actual_i_sq_acc = 0.0;
                    self.actual_i_sum_acc = 0.0;
                    self.vcap_sin_acc = 0.0;
                    self.vcap_cos_acc = 0.0;
                    self.vcap_sq_acc = 0.0;
                    self.vcap_sum_acc = 0.0;
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
                self.i_sq_acc += i * i;
                self.v_sum_acc += v;
                self.i_sum_acc += i;
                self.cos_sq_acc += cos_val * cos_val;
                self.sin_sq_acc += sin_val * sin_val;
                self.cos_sin_acc += cos_val * sin_val;
                self.cos_sum_acc += cos_val;
                self.sin_sum_acc += sin_val;

                // Actual discrete electron current lock-in
                let actual_i = if dt > 0.0 { actual_electron_delta as f64 / dt as f64 } else { 0.0 };
                self.actual_i_sin_acc += actual_i * sin_val;
                self.actual_i_cos_acc += actual_i * cos_val;
                self.actual_i_sq_acc += actual_i * actual_i;
                self.actual_i_sum_acc += actual_i;

                // Virtual capacitor: V_cap += I·dt / C
                if self.c_virtual > 0.0 {
                    self.v_cap += i * dt as f64 / self.c_virtual;
                }
                let vc = self.v_cap;
                self.vcap_sin_acc += vc * sin_val;
                self.vcap_cos_acc += vc * cos_val;
                self.vcap_sq_acc += vc * vc;
                self.vcap_sum_acc += vc;

                self.sample_count += 1;

                // Check if we've recorded enough cycles
                let recording_cycles =
                    ((time - self.t_start_recording) as f64) / period as f64;
                if recording_cycles >= self.config.periods_per_freq as f64 {
                    self.finish_frequency_point();
                    self.current_repeat += 1;
                    if self.current_repeat < self.config.repeats_per_freq {
                        // More repeats needed — reset accumulators, go back to Settling
                        self.reset_accumulators(time);
                        return false;
                    } else {
                        // All repeats done — average and advance
                        self.push_averaged_point();
                        self.current_repeat = 0;
                        self.repeat_buffer.clear();
                        return self.advance_to_next_frequency(time);
                    }
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

        // R² for V and I independently.
        //   P_fit = (re² + im²) / 2   — RMS² of the best-fit sinusoid
        //   DC    = Σsignal / N
        //   P_ac  = mean(signal²) - DC²   — AC variance about the mean
        //   R²    = P_fit / P_ac
        let r2 = |re: f64, im: f64, sum: f64, sq: f64| -> f64 {
            let p_fit = (re * re + im * im) * 0.5;
            let dc = sum / n;
            let p_ac = (sq / n - dc * dc).max(0.0);
            if p_ac > 1e-60 { (p_fit / p_ac).min(1.0) } else { 1.0 }
        };
        let fit_r2_v = r2(v_re, v_im, self.v_sum_acc, self.v_sq_acc);
        let fit_r2_i = r2(i_re, i_im, self.i_sum_acc, self.i_sq_acc);

        // Per-signal amplitude and phase for the fitted sinusoid.
        // Using the cosine convention: signal(t) = amp · cos(ωt + φ) + dc
        // where amp = sqrt(re² + im²)  and  φ = atan2(-im, re).
        let fit_v_amp = (v_re * v_re + v_im * v_im).sqrt();
        let fit_v_phase_deg = (-v_im).atan2(v_re).to_degrees();
        let fit_i_amp = (i_re * i_re + i_im * i_im).sqrt();
        let fit_i_phase_deg = (-i_im).atan2(i_re).to_degrees();

        // Z = V / I  (complex division)
        // The fit convention  signal = A·cos(ωt) + B·sin(ωt)  maps a pure-sine I to
        // i_re=0, i_im=A.  A capacitive V lags I by 90° → v_re=-A|Z|, v_im=0.
        // Without correction: z_imag = (v_im·i_re − v_re·i_im)/denom = +|Z| > 0.
        // Standard EIS requires Im(Z) < 0 for capacitive, so negate z_imag here.
        // The Nyquist y-axis then plots −z_imag > 0 (arc in upper half).
        let denom = i_re * i_re + i_im * i_im;
        let (z_real, z_imag) = if denom > 1e-30 {
            (
                 (v_re * i_re + v_im * i_im) / denom,   //  Re(V·I*) / |I|²
                -((v_im * i_re - v_re * i_im) / denom), // −Im(V·I*) / |I|²  → Im(Z) < 0 for capacitive
            )
        } else {
            (0.0, 0.0)
        };

        let magnitude = (z_real * z_real + z_imag * z_imag).sqrt();
        let phase_deg = z_imag.atan2(z_real).to_degrees();

        // Z_actual = V̂ / Î_actual (impedance from discrete electron current)
        let (ai_re, ai_im) = self.ls_fit_re_im(self.actual_i_cos_acc, self.actual_i_sin_acc, self.actual_i_sum_acc);
        let ai_denom = ai_re * ai_re + ai_im * ai_im;
        let (z_actual_real, z_actual_imag) = if ai_denom > 1e-30 {
            (
                 (v_re * ai_re + v_im * ai_im) / ai_denom,
                -((v_im * ai_re - v_re * ai_im) / ai_denom),
            )
        } else {
            (0.0, 0.0)
        };

        // Z_cap = V̂_cap / Î_pid (virtual capacitor diagnostic)
        let (vc_re, vc_im) = self.ls_fit_re_im(self.vcap_cos_acc, self.vcap_sin_acc, self.vcap_sum_acc);
        let (z_cap_real, z_cap_imag) = if denom > 1e-30 {
            (
                 (vc_re * i_re + vc_im * i_im) / denom,
                -((vc_im * i_re - vc_re * i_im) / denom),
            )
        } else {
            (0.0, 0.0)
        };

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
        eprintln!(
            "[EIS]   Z_actual = ({:.4e}, {:.4e}), Z_cap = ({:.4e}, {:.4e})",
            z_actual_real, z_actual_imag, z_cap_real, z_cap_imag,
        );

        let point = EisPoint {
            frequency: freq,
            z_real,
            z_imag,
            magnitude,
            phase_deg,
            fit_r2_v,
            fit_r2_i,
            fit_v_amp,
            fit_v_phase_deg,
            fit_i_amp,
            fit_i_phase_deg,
            z_actual_real,
            z_actual_imag,
            z_cap_real,
            z_cap_imag,
        };
        self.repeat_buffer.push(point);
    }

    /// Reset lock-in accumulators and go back to Settling phase.
    /// Used both when advancing to the next frequency and when starting a new repeat.
    fn reset_accumulators(&mut self, time: f32) {
        self.phase = EisPhase::Settling;
        self.t_start = time;
        self.v_sin_acc = 0.0;
        self.v_cos_acc = 0.0;
        self.i_sin_acc = 0.0;
        self.i_cos_acc = 0.0;
        self.v_sq_acc = 0.0;
        self.i_sq_acc = 0.0;
        self.v_sum_acc = 0.0;
        self.i_sum_acc = 0.0;
        self.sample_count = 0;
        self.cos_sq_acc = 0.0;
        self.sin_sq_acc = 0.0;
        self.cos_sin_acc = 0.0;
        self.cos_sum_acc = 0.0;
        self.sin_sum_acc = 0.0;
        self.actual_i_sin_acc = 0.0;
        self.actual_i_cos_acc = 0.0;
        self.actual_i_sq_acc = 0.0;
        self.actual_i_sum_acc = 0.0;
        // Note: v_cap is NOT reset — it's a running integral across frequencies
        self.vcap_sin_acc = 0.0;
        self.vcap_cos_acc = 0.0;
        self.vcap_sq_acc = 0.0;
        self.vcap_sum_acc = 0.0;
        self.ts_step_counter = 0;
        self.ts_subsample = 1;
        // Clear time-series buffer and fit state
        let mut shared = EIS_RESULTS.lock();
        shared.ts_t_rel.clear();
        shared.ts_v.clear();
        shared.ts_i.clear();
        shared.ts_is_recording.clear();
        shared.ts_actual_i.clear();
        shared.ts_v_cap.clear();
        shared.fit_v_re = 0.0;
        shared.fit_v_im = 0.0;
        shared.fit_v_dc = 0.0;
        shared.fit_i_re = 0.0;
        shared.fit_i_im = 0.0;
        shared.fit_i_dc = 0.0;
        shared.current_repeat = self.current_repeat;
    }

    /// Average all EisPoints in repeat_buffer and push the averaged point to results/shared.
    fn push_averaged_point(&mut self) {
        let n = self.repeat_buffer.len();
        if n == 0 {
            return;
        }
        let nf = n as f64;
        let z_real: f64 = self.repeat_buffer.iter().map(|p| p.z_real).sum::<f64>() / nf;
        let z_imag: f64 = self.repeat_buffer.iter().map(|p| p.z_imag).sum::<f64>() / nf;
        let magnitude = (z_real * z_real + z_imag * z_imag).sqrt();
        let phase_deg = z_imag.atan2(z_real).to_degrees();
        let fit_r2_v: f64 = self.repeat_buffer.iter().map(|p| p.fit_r2_v).sum::<f64>() / nf;
        let fit_r2_i: f64 = self.repeat_buffer.iter().map(|p| p.fit_r2_i).sum::<f64>() / nf;
        let fit_v_amp: f64 = self.repeat_buffer.iter().map(|p| p.fit_v_amp).sum::<f64>() / nf;
        let fit_v_phase_deg: f64 = self.repeat_buffer.iter().map(|p| p.fit_v_phase_deg).sum::<f64>() / nf;
        let fit_i_amp: f64 = self.repeat_buffer.iter().map(|p| p.fit_i_amp).sum::<f64>() / nf;
        let fit_i_phase_deg: f64 = self.repeat_buffer.iter().map(|p| p.fit_i_phase_deg).sum::<f64>() / nf;
        let z_actual_real: f64 = self.repeat_buffer.iter().map(|p| p.z_actual_real).sum::<f64>() / nf;
        let z_actual_imag: f64 = self.repeat_buffer.iter().map(|p| p.z_actual_imag).sum::<f64>() / nf;
        let z_cap_real: f64 = self.repeat_buffer.iter().map(|p| p.z_cap_real).sum::<f64>() / nf;
        let z_cap_imag: f64 = self.repeat_buffer.iter().map(|p| p.z_cap_imag).sum::<f64>() / nf;

        let freq = self.repeat_buffer[0].frequency;

        if n > 1 {
            eprintln!(
                "[EIS] Averaged {} repeats for freq {:.2e}: Z = ({:.4e}, {:.4e})",
                n, freq, z_real, z_imag,
            );
        }

        let point = EisPoint {
            frequency: freq,
            z_real,
            z_imag,
            magnitude,
            phase_deg,
            fit_r2_v,
            fit_r2_i,
            fit_v_amp,
            fit_v_phase_deg,
            fit_i_amp,
            fit_i_phase_deg,
            z_actual_real,
            z_actual_imag,
            z_cap_real,
            z_cap_imag,
        };
        self.results.push(point.clone());

        let mut shared = EIS_RESULTS.lock();
        shared.points.push(point);
        shared.current_freq_idx = self.current_freq_idx + 1;
    }

    /// Advance to the next frequency. Returns true if sweep is complete.
    fn advance_to_next_frequency(&mut self, time: f32) -> bool {
        // Save time-series for the frequency that just completed, before clearing the buffer.
        let completed_idx = self.current_freq_idx;
        let completed_freq = self.config.frequencies[completed_idx];
        {
            let shared = EIS_RESULTS.lock();
            Self::save_timeseries_csv(completed_idx, completed_freq, &shared);
        }

        self.current_freq_idx += 1;
        if self.current_freq_idx >= self.config.frequencies.len() {
            self.finished = true;
            eprintln!("[EIS] Sweep complete! {} points collected", self.results.len());
            let mut shared = EIS_RESULTS.lock();
            shared.is_running = false;
            return true;
        }
        self.reset_accumulators(time);
        false
    }

    /// Write the current time-series buffer to `eis_timeseries/eis_ts_NNN_F.csv`.
    ///
    /// Columns:
    ///   t_rel_fs      — time relative to this frequency's window start (fs)
    ///   v             — raw Coulomb potential (large positive DC + small AC)
    ///   i             — raw current signal (DC + AC)
    ///   v_ac          — DC-removed V: v − mean(v) over recording-phase samples
    ///   i_ac          — DC-removed I: i − mean(i) over recording-phase samples
    ///   is_recording  — 1 during the recording phase, 0 during settling
    ///
    /// The raw signals often have a large positive DC offset that swamps the AC
    /// perturbation, making them look one-sided.  Use v_ac / i_ac to see the
    /// symmetric AC oscillation directly.
    fn save_timeseries_csv(freq_idx: usize, freq: f32, shared: &EisSharedState) {
        let dir = std::path::Path::new("eis_timeseries");
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("[EIS] Could not create eis_timeseries/: {}", e);
            return;
        }
        let filename = format!("eis_ts_{:03}_{:.3e}.csv", freq_idx + 1, freq);
        let path = dir.join(&filename);

        let n = shared.ts_t_rel.len();

        // Compute mean of V and I over recording-phase samples only, so the
        // DC offset is representative of the actual operating point rather than
        // being diluted by the settling transient.
        let (v_mean, i_mean) = {
            let (mut sv, mut si, mut cnt) = (0.0f64, 0.0f64, 0usize);
            for k in 0..n {
                if shared.ts_is_recording[k] {
                    sv += shared.ts_v[k] as f64;
                    si += shared.ts_i[k] as f64;
                    cnt += 1;
                }
            }
            if cnt > 0 {
                (sv / cnt as f64, si / cnt as f64)
            } else {
                // No recording-phase samples yet — fall back to full-window mean
                let sv: f64 = shared.ts_v.iter().map(|&x| x as f64).sum();
                let si: f64 = shared.ts_i.iter().map(|&x| x as f64).sum();
                let nn = n.max(1) as f64;
                (sv / nn, si / nn)
            }
        };

        let mut csv = String::with_capacity(n * 60);
        // Header comment with DC values for reference
        csv.push_str(&format!(
            "# v_dc={:.6e}  i_dc={:.6e}  freq={:.3e}\n",
            v_mean, i_mean, freq
        ));
        // Compute mean of actual_i over recording-phase samples
        let ai_mean = {
            let (mut s, mut cnt) = (0.0f64, 0usize);
            for k in 0..n {
                if shared.ts_is_recording[k] {
                    if let Some(&ai) = shared.ts_actual_i.get(k) {
                        s += ai as f64;
                        cnt += 1;
                    }
                }
            }
            if cnt > 0 { s / cnt as f64 } else { 0.0 }
        };

        csv.push_str("t_rel_fs,v,i,v_ac,i_ac,actual_i,actual_i_ac,v_cap,is_recording\n");
        for k in 0..n {
            let v_ac = shared.ts_v[k] as f64 - v_mean;
            let i_ac = shared.ts_i[k] as f64 - i_mean;
            let ai = shared.ts_actual_i.get(k).copied().unwrap_or(0.0);
            let ai_ac = ai as f64 - ai_mean;
            let vc = shared.ts_v_cap.get(k).copied().unwrap_or(0.0);
            csv.push_str(&format!(
                "{:.4e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{}\n",
                shared.ts_t_rel[k],
                shared.ts_v[k],
                shared.ts_i[k],
                v_ac,
                i_ac,
                ai,
                ai_ac,
                vc,
                if shared.ts_is_recording[k] { 1 } else { 0 },
            ));
        }
        match std::fs::write(&path, &csv) {
            Ok(_) => eprintln!(
                "[EIS] Saved time-series: {} ({} pts, v_dc={:.3e}, i_dc={:.3e})",
                path.display(), n, v_mean, i_mean
            ),
            Err(e) => eprintln!("[EIS] Failed to save {}: {}", path.display(), e),
        }
    }
}
