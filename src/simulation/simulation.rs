// simulation/simulation.rs
// Contains the Simulation struct and main methods (new, step, iterate, perform_electron_hopping)

use super::collision;
use super::compressed_history::CompressedHistorySystem;
use super::forces;
use super::history::PlaybackController;
use crate::body::foil::LinkMode;
use crate::config;
use crate::manual_measurement::{ManualMeasurementConfig, ManualMeasurementRecorder};
use crate::profile_scope;
use crate::renderer::state::{
    COLLISION_PASSES, FIELD_DIRECTION, FIELD_MAGNITUDE, SIM_TIME, TIMESTEP,
};
use crate::{
    body::{Body, Electron, Species},
    cell_list::CellList,
    quadtree::Quadtree,
    switch_charging::{
        self, FoilStateSnapshot, RunState, StatusSender, SwitchControl, SwitchScheduler,
        SwitchStatus,
    },
};
use rand::prelude::*; // Import all prelude traits for rand 0.9+
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use ultraviolet::Vec2; // for writeln! and flush on File

/// The main simulation state and logic for the particle system.
pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub time: f32,
    pub bodies: Vec<Body>,
    pub quadtree: Quadtree,
    pub cell_list: CellList,
    pub domain_width: f32,  // Half-width of the domain (from center to edge)
    pub domain_height: f32, // Half-height of the domain (from center to edge)
    pub domain_depth: f32,  // Half-depth of the domain (for z-direction)
    pub rewound_flags: Vec<bool>,
    pub background_e_field: Vec2,
    pub prev_induced_e_field: Vec2,
    pub foils: Vec<crate::body::foil::Foil>,
    pub body_to_foil: HashMap<u64, u64>,
    pub config: config::SimConfig, //
    /// Track when thermostat was last applied (in simulation time)
    pub last_thermostat_time: f32,
    pub compressed_history: CompressedHistorySystem, // Keep for compatibility but unused
    pub simple_history: std::collections::VecDeque<crate::io::SimulationState>,
    pub history_cursor: usize,
    pub history_dirty: bool,
    pub history_capacity: usize,
    pub playback: PlaybackController,
    pub switch_config: switch_charging::SwitchChargingConfig,
    pub switch_scheduler: SwitchScheduler,
    pub switch_run_state: RunState,
    pub switch_saved_states: HashMap<u64, FoilStateSnapshot>,
    pub switch_active_pair: Option<(u64, u64)>,
    pub switch_status_tx: Option<StatusSender>,
    pub thermostat_bootstrapped: bool,
    // Foil group linking (parallel within group, opposite between groups)
    pub group_a: std::collections::HashSet<u64>,
    pub group_b: std::collections::HashSet<u64>,
    // Pre-allocated temporary set for switch-charging inactive foil tracking
    temp_inactive_set: std::collections::HashSet<u64>,
    // Manual measurement recorder for auto-recording measurements to CSV
    pub manual_measurement_recorder: Option<ManualMeasurementRecorder>,
    // Foil metrics CSV writer state (written when manual measurements occur)
    foil_metrics_csv: Option<File>,
    foil_metrics_current_base: Option<String>,
}

impl Simulation {
    pub fn new() -> Self {
        let dt = config::DEFAULT_DT_FS;
        let theta = config::QUADTREE_THETA;
        let epsilon = config::QUADTREE_EPSILON;
        let leaf_capacity = config::QUADTREE_LEAF_CAPACITY;
        let thread_capacity = config::QUADTREE_THREAD_CAPACITY;
        let bounds = config::DOMAIN_BOUNDS;
        // Start with no bodies; scenario setup is now done via SimCommand AddCircle/AddBody
        let bodies = Vec::new();
        let quadtree = Quadtree::new(theta, epsilon, leaf_capacity, thread_capacity);
        let cell_size = crate::species::max_lj_cutoff();
        let cell_list = CellList::new(bounds, bounds, cell_size);
        let rewound_flags = vec![];
        // Initialize compressed history system
        let compressed_history = CompressedHistorySystem::new_default();
        let history_capacity = std::cmp::max(1, config::PLAYBACK_HISTORY_FRAMES);
        let mut sim = Self {
            dt,
            frame: 0,
            time: 0.0,
            bodies,
            quadtree,
            cell_list,
            domain_width: bounds, // Initialize with square domain, will be updated by SetDomainSize command
            domain_height: bounds, // Initialize with square domain, will be updated by SetDomainSize command
            domain_depth: bounds,  // Initialize with square domain depth
            rewound_flags,
            background_e_field: Vec2::zero(),
            prev_induced_e_field: Vec2::zero(),
            foils: Vec::new(),
            body_to_foil: HashMap::new(),
            config: config::SimConfig::default(),
            last_thermostat_time: 0.0,
            compressed_history,
            simple_history: std::collections::VecDeque::new(),
            history_cursor: 0,
            history_dirty: false,
            history_capacity,
            playback: PlaybackController::new(),
            switch_config: switch_charging::SwitchChargingConfig::default(),
            switch_scheduler: SwitchScheduler::default(),
            switch_run_state: RunState::Idle,
            switch_saved_states: HashMap::new(),
            switch_active_pair: None,
            switch_status_tx: None,
            thermostat_bootstrapped: false,
            group_a: std::collections::HashSet::new(),
            group_b: std::collections::HashSet::new(),
            temp_inactive_set: std::collections::HashSet::new(),
            manual_measurement_recorder: None,
            foil_metrics_csv: None,
            foil_metrics_current_base: None,
        };
        sim.initialize_history();
        sim
    }

    pub fn set_switch_status_sender(&mut self, sender: StatusSender) {
        self.switch_status_tx = Some(sender);
    }

    pub fn handle_switch_control(&mut self, control: SwitchControl) {
        match control {
            SwitchControl::Start => self.start_switch_charging(),
            SwitchControl::Pause => self.pause_switch_charging(),
            SwitchControl::Stop => self.stop_switch_charging(),
            SwitchControl::UpdateConfig(cfg) => self.update_switch_config(cfg),
        }
    }

    fn start_switch_charging(&mut self) {
        match self.switch_config.validate() {
            Ok(_) => {
                self.refresh_switch_snapshots();
                self.switch_scheduler.start(&self.switch_config);
                self.switch_run_state = RunState::Running;
                self.send_switch_status(SwitchStatus::RunState(RunState::Running));
                self.mark_history_dirty();
            }
            Err(err) => {
                self.send_switch_status(SwitchStatus::ValidationFailed(err));
            }
        }
    }

    fn pause_switch_charging(&mut self) {
        if self.switch_run_state == RunState::Running {
            self.switch_scheduler.pause();
            self.switch_run_state = RunState::Paused;
            self.send_switch_status(SwitchStatus::RunState(RunState::Paused));
        }
    }

    fn stop_switch_charging(&mut self) {
        if self.switch_run_state != RunState::Idle {
            self.switch_scheduler.stop();
            self.switch_run_state = RunState::Idle;
            self.switch_active_pair = None;
            self.restore_all_switch_snapshots();
            self.switch_saved_states.clear();
            self.send_switch_status(SwitchStatus::RunState(RunState::Idle));
            self.mark_history_dirty();
        }
    }

    fn update_switch_config(&mut self, mut cfg: switch_charging::SwitchChargingConfig) {
        cfg.ensure_all_steps();
        self.switch_config = cfg;
        self.switch_config.sim_dt_s = (self.dt as f64) * 1e-15;
        self.switch_active_pair = None;
        match self.switch_config.validate() {
            Ok(_) => {
                self.switch_scheduler.sync_with_config(&self.switch_config);
                self.refresh_switch_snapshots();
                self.send_switch_status(SwitchStatus::ConfigApplied(self.switch_config.clone()));
                self.mark_history_dirty();
            }
            Err(err) => {
                self.send_switch_status(SwitchStatus::ValidationFailed(err));
            }
        }
    }

    fn refresh_switch_snapshots(&mut self) {
        let assigned: HashSet<u64> = self
            .switch_config
            .role_to_foil
            .values()
            .flatten()
            .copied()
            .collect();
        self.switch_saved_states
            .retain(|id, _| assigned.contains(id));
        for foil_id in assigned {
            if let Some(foil) = self.foils.iter().find(|f| f.id == foil_id) {
                self.switch_saved_states
                    .insert(foil_id, FoilStateSnapshot::from_foil(foil));
            }
        }
    }

    fn restore_snapshot_for(&mut self, foil_id: u64) {
        if let Some(snapshot) = self.switch_saved_states.get(&foil_id).cloned() {
            if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                snapshot.apply(foil);
            }
        }
    }

    fn restore_all_switch_snapshots(&mut self) {
        let ids: Vec<u64> = self.switch_saved_states.keys().copied().collect();
        for foil_id in ids {
            self.restore_snapshot_for(foil_id);
        }
    }

    // Manual measurement control methods
    pub fn start_manual_measurement(&mut self, config: ManualMeasurementConfig) {
        let mut recorder = ManualMeasurementRecorder::new(config);
        let simulation_time_fs = self.time;
        match recorder.start_recording(simulation_time_fs) {
            Ok(_) => {
                println!("✓ Started manual measurement recording");
                self.manual_measurement_recorder = Some(recorder);
            }
            Err(e) => {
                eprintln!("✗ Failed to start manual measurement recording: {}", e);
            }
        }
    }

    pub fn stop_manual_measurement(&mut self) {
        if let Some(recorder) = &mut self.manual_measurement_recorder {
            recorder.stop_recording();
        }
        self.manual_measurement_recorder = None;
    }

    fn apply_switch_step_active_inactive(&mut self, foil_pairs: (Vec<u64>, Vec<u64>)) {
        let (pos_ids, neg_ids) = &foil_pairs;
        self.switch_active_pair = Some((pos_ids[0], neg_ids[0])); // For compatibility, store first pair

        // Collect all foil IDs to avoid borrow conflicts
        let all_foil_ids: Vec<u64> = self
            .switch_config
            .role_to_foil
            .values()
            .flatten()
            .copied()
            .collect();

        // First, restore all snapshots
        for &foil_id in &all_foil_ids {
            self.restore_snapshot_for(foil_id);
        }

        // Resolve active/inactive setpoints (global vs per-step)
        let current_step = self.switch_scheduler.current_step();
        let (active_sp, inactive_sp): (
            &crate::switch_charging::StepSetpoint,
            &crate::switch_charging::StepSetpoint,
        ) = if self.switch_config.use_global_active_inactive {
            (
                &self.switch_config.global_active,
                &self.switch_config.global_inactive,
            )
        } else {
            match self.switch_config.step_active_inactive.get(&current_step) {
                Some(sai) => (&sai.active, &sai.inactive),
                None => {
                    eprintln!(
                        "Warning: No active/inactive setpoints found for step {}",
                        current_step
                    );
                    return;
                }
            }
        };

        // Helper to apply a setpoint to a specific foil
        let apply_setpoint_to_foil =
            |foil: &mut crate::body::foil::Foil, sp: &crate::switch_charging::StepSetpoint| {
                match sp.mode {
                    crate::switch_charging::Mode::Current => {
                        if foil.charging_mode != crate::body::foil::ChargingMode::Current {
                            foil.disable_overpotential_mode();
                        }
                        foil.charging_mode = crate::body::foil::ChargingMode::Current;
                        foil.ac_current = 0.0;
                        foil.switch_hz = 0.0;
                        // Clear overpotential-related state to avoid residual effects when switching modes
                        foil.slave_overpotential_current = 0.0;
                        // dc_current will be set by caller
                    }
                    crate::switch_charging::Mode::Overpotential => {
                        if foil.charging_mode != crate::body::foil::ChargingMode::Overpotential {
                            foil.enable_overpotential_mode(1.0);
                        }
                        if let Some(controller) = foil.overpotential_controller.as_mut() {
                            controller.target_ratio = sp.value as f32;
                        }
                        // Ensure no stale current drive remains when in overpotential control
                        foil.dc_current = 0.0;
                        foil.ac_current = 0.0;
                        foil.switch_hz = 0.0;
                    }
                }
            };

        // Apply active setpoint to active foils (those participating in this step)
        let active_foil_ids: std::collections::HashSet<u64> =
            pos_ids.iter().chain(neg_ids.iter()).copied().collect();

        // Active foils (current step participants)
        if matches!(active_sp.mode, crate::switch_charging::Mode::Current) {
            // Split current between positive and negative groups, guarding against empty groups
            if !pos_ids.is_empty() {
                let pos_current_per_foil = active_sp.value as f32 / pos_ids.len() as f32;
                for &foil_id in pos_ids {
                    if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                        apply_setpoint_to_foil(foil, &active_sp);
                        foil.dc_current = pos_current_per_foil; // Positive group gets positive current
                    }
                }
            }
            if !neg_ids.is_empty() {
                let neg_current_per_foil = active_sp.value as f32 / neg_ids.len() as f32;
                for &foil_id in neg_ids {
                    if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                        apply_setpoint_to_foil(foil, &active_sp);
                        foil.dc_current = -neg_current_per_foil; // Negative group gets negative current
                    }
                }
            }
        } else {
            // Overpotential mode - apply to all active foils
            // Positive group gets target V; negative group gets complementary (2.0 - V)
            let v_pos = active_sp.value as f32;
            let v_neg = (2.0 - active_sp.value) as f32;
            for &foil_id in pos_ids {
                if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                    apply_setpoint_to_foil(foil, &active_sp);
                    if let Some(ctrl) = foil.overpotential_controller.as_mut() {
                        ctrl.target_ratio = v_pos;
                    }
                }
            }
            for &foil_id in neg_ids {
                if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                    apply_setpoint_to_foil(foil, &active_sp);
                    if let Some(ctrl) = foil.overpotential_controller.as_mut() {
                        ctrl.target_ratio = v_neg;
                    }
                }
            }
        }

        // Apply per-step INACTIVE setpoints to foils belonging to steps that are NOT active
        self.temp_inactive_set.clear();
        let applied_inactive = &mut self.temp_inactive_set;
        for step in 0u8..4u8 {
            if step == current_step {
                continue;
            }
            // Fetch the inactive setpoint for this non-active step
            let (inactive_mode, inactive_value) = if self.switch_config.use_global_active_inactive {
                (inactive_sp.mode, inactive_sp.value)
            } else {
                if let Some(sai_other) = self.switch_config.step_active_inactive.get(&step) {
                    (sai_other.inactive.mode, sai_other.inactive.value)
                } else {
                    continue;
                }
            };
            let (pos_role, neg_role) = crate::switch_charging::roles_for_step(step);
            let pos_step_ids = self.switch_config.foils_for_role(pos_role);
            let neg_step_ids = self.switch_config.foils_for_role(neg_role);

            for &foil_id in pos_step_ids.iter().chain(neg_step_ids.iter()) {
                if active_foil_ids.contains(&foil_id) {
                    continue;
                }
                if applied_inactive.contains(&foil_id) {
                    continue;
                }
                if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                    let sp = crate::switch_charging::StepSetpoint {
                        mode: inactive_mode,
                        value: inactive_value,
                    };
                    apply_setpoint_to_foil(foil, &sp);
                    if matches!(inactive_mode, crate::switch_charging::Mode::Current) {
                        foil.dc_current = inactive_value as f32;
                    }
                    applied_inactive.insert(foil_id);
                }
            }
        }

        // For any remaining foils not in the current step and not assigned to any step roles (or missed),
        // apply the current step's INACTIVE as a fallback.
        for &foil_id in &all_foil_ids {
            if active_foil_ids.contains(&foil_id) {
                continue;
            }
            if applied_inactive.contains(&foil_id) {
                continue;
            }
            if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                let sp = crate::switch_charging::StepSetpoint {
                    mode: inactive_sp.mode,
                    value: inactive_sp.value,
                };
                apply_setpoint_to_foil(foil, &sp);
                if matches!(inactive_sp.mode, crate::switch_charging::Mode::Current) {
                    foil.dc_current = inactive_sp.value as f32;
                }
                applied_inactive.insert(foil_id);
            }
        }
    }

    fn apply_switch_step(
        &mut self,
        foil_pairs: (Vec<u64>, Vec<u64>),
        setpoint: &switch_charging::StepSetpoint,
    ) {
        let (pos_ids, neg_ids) = &foil_pairs;
        self.switch_active_pair = Some((pos_ids[0], neg_ids[0])); // For compatibility, store first pair

        // Collect all foil IDs to avoid borrow conflicts
        let all_foil_ids: Vec<u64> = self
            .switch_config
            .role_to_foil
            .values()
            .flatten()
            .copied()
            .collect();

        // First, restore all snapshots
        for &foil_id in &all_foil_ids {
            self.restore_snapshot_for(foil_id);
        }

        // Apply step setpoint to all foils (old behavior)
        match setpoint.mode {
            switch_charging::Mode::Current => {
                // Divide current between foils in each group
                let pos_current_per_foil = setpoint.value as f32 / pos_ids.len() as f32;
                let neg_current_per_foil = setpoint.value as f32 / neg_ids.len() as f32;

                for &foil_id in pos_ids {
                    if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                        if foil.charging_mode != crate::body::foil::ChargingMode::Current {
                            foil.disable_overpotential_mode();
                        }
                        foil.charging_mode = crate::body::foil::ChargingMode::Current;
                        foil.dc_current = pos_current_per_foil;
                        foil.ac_current = 0.0;
                        foil.switch_hz = 0.0;
                    }
                }

                for &foil_id in neg_ids {
                    if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                        if foil.charging_mode != crate::body::foil::ChargingMode::Current {
                            foil.disable_overpotential_mode();
                        }
                        foil.charging_mode = crate::body::foil::ChargingMode::Current;
                        foil.dc_current = -neg_current_per_foil;
                        foil.ac_current = 0.0;
                        foil.switch_hz = 0.0;
                    }
                }

                // For inactive foils in Current mode, set them to overpotential neutralization
                let active_ids: std::collections::HashSet<u64> =
                    pos_ids.iter().chain(neg_ids.iter()).copied().collect();
                for &foil_id in &all_foil_ids {
                    if !active_ids.contains(&foil_id) {
                        if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                            // Neutralize inactive foils with overpotential 1.0
                            if foil.charging_mode != crate::body::foil::ChargingMode::Overpotential
                            {
                                foil.enable_overpotential_mode(1.0);
                            }
                            if let Some(controller) = foil.overpotential_controller.as_mut() {
                                controller.target_ratio = 1.0;
                            }
                        }
                    }
                }
            }
            switch_charging::Mode::Overpotential => {
                // Apply same overpotential to all foils in each group
                let target_positive = setpoint.value as f32;
                let target_negative = (2.0 - setpoint.value) as f32;

                for &foil_id in pos_ids {
                    if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                        if foil.charging_mode != crate::body::foil::ChargingMode::Overpotential {
                            foil.enable_overpotential_mode(target_positive);
                        }
                        if let Some(controller) = foil.overpotential_controller.as_mut() {
                            controller.target_ratio = target_positive;
                        }
                    }
                }

                for &foil_id in neg_ids {
                    if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                        if foil.charging_mode != crate::body::foil::ChargingMode::Overpotential {
                            foil.enable_overpotential_mode(target_negative);
                        }
                        if let Some(controller) = foil.overpotential_controller.as_mut() {
                            controller.target_ratio = target_negative;
                        }
                    }
                }

                // For inactive foils, also neutralize them
                let active_ids: std::collections::HashSet<u64> =
                    pos_ids.iter().chain(neg_ids.iter()).copied().collect();
                for &foil_id in &all_foil_ids {
                    if !active_ids.contains(&foil_id) {
                        if let Some(foil) = self.foils.iter_mut().find(|f| f.id == foil_id) {
                            if foil.charging_mode != crate::body::foil::ChargingMode::Overpotential
                            {
                                foil.enable_overpotential_mode(1.0);
                            }
                            if let Some(controller) = foil.overpotential_controller.as_mut() {
                                controller.target_ratio = 1.0;
                            }
                        }
                    }
                }
            }
        }
    }

    fn tick_switch_charging(&mut self) {
        self.switch_config.sim_dt_s = (self.dt as f64) * 1e-15;
        if self.switch_run_state != RunState::Running {
            return;
        }
        if let Some(((pos_ids, neg_ids), setpoint)) =
            self.switch_scheduler.on_tick(&self.switch_config)
        {
            // When using step-based active/inactive setpoints, ignore the legacy step setpoint completely
            if self.switch_config.use_active_inactive_setpoints {
                self.apply_switch_step_active_inactive((pos_ids, neg_ids));
            } else {
                self.apply_switch_step((pos_ids, neg_ids), &setpoint);
            }
            self.send_switch_status(SwitchStatus::ActiveStep {
                step_index: self.switch_scheduler.current_step(),
                dwell_remaining: self.switch_scheduler.dwell_remaining(),
            });
            // Mirror the active step for renderer global state (used in playback and live)
            *crate::renderer::state::SWITCH_STEP.lock() =
                Some(self.switch_scheduler.current_step());
        }
    }

    fn send_switch_status(&self, status: SwitchStatus) {
        if let Some(tx) = &self.switch_status_tx {
            let _ = tx.send(status);
        }
    }

    pub fn step(&mut self) {
        profile_scope!("simulation_step");
        // Sync config from global LJ_CONFIG (updated by GUI)
        let global_config = crate::config::LJ_CONFIG.lock();
        if global_config.config_version != self.config.config_version {
            self.config = global_config.clone();
        } else {
            drop(global_config); // Release lock early
        }

        let mag = *FIELD_MAGNITUDE.lock();
        let theta = (*FIELD_DIRECTION.lock()).to_radians();
        let manual_field = Vec2::new(theta.cos(), theta.sin()) * mag;

        // Compute induced external field from foil charging (current or overpotential)
        let mut induced_field = Vec2::zero();
        if self.foils.len() >= 2 && self.config.induced_field_gain != 0.0 {
            // Determine active positive/negative foil groups for direction
            // Fallback: use first two foils as pos/neg by net current sign
            let mut pos_centroid = Vec2::zero();
            let mut neg_centroid = Vec2::zero();
            let mut pos_count = 0u32;
            let mut neg_count = 0u32;
            let mut pos_drive_sum = 0.0f32;
            let mut neg_drive_sum = 0.0f32;

            // Determine per-foil drive based on mode
            for foil in &self.foils {
                // Compute a drive magnitude: |current| for current mode, or |target-1|*scale for overpotential
                let drive = match foil.charging_mode {
                    crate::body::foil::ChargingMode::Current => foil.dc_current.abs(),
                    crate::body::foil::ChargingMode::Overpotential => {
                        if let Some(ctrl) = &foil.overpotential_controller {
                            (ctrl.target_ratio - 1.0).abs()
                                * self.config.induced_field_overpot_scale
                        } else {
                            0.0
                        }
                    }
                };

                // Compute foil centroid
                if !foil.body_ids.is_empty() {
                    let mut c = Vec2::zero();
                    let mut n = 0.0f32;
                    for id in &foil.body_ids {
                        if let Some(b) = self.bodies.iter().find(|b| b.id == *id) {
                            c += b.pos;
                            n += 1.0;
                        }
                    }
                    if n > 0.0 {
                        c /= n;
                    }

                    // Classify by sign of intended current if in current mode; else by link mode/heuristic
                    let is_pos = match foil.charging_mode {
                        crate::body::foil::ChargingMode::Current => foil.dc_current > 0.0,
                        crate::body::foil::ChargingMode::Overpotential => {
                            // Heuristic: target>1 => cathodic (acts like positive collector of electrons)
                            if let Some(ctrl) = &foil.overpotential_controller {
                                ctrl.target_ratio >= 1.0
                            } else {
                                false
                            }
                        }
                    };

                    if is_pos {
                        pos_centroid += c;
                        pos_count += 1;
                        pos_drive_sum += drive;
                    } else {
                        neg_centroid += c;
                        neg_count += 1;
                        neg_drive_sum += drive;
                    }
                }
            }

            if pos_count > 0 {
                pos_centroid /= pos_count as f32;
            }
            if neg_count > 0 {
                neg_centroid /= neg_count as f32;
            }

            // Direction from negative to positive
            let mut dir = pos_centroid - neg_centroid;
            if dir.mag() > 1e-6 {
                dir = dir.normalized();
            } else {
                dir = Vec2::new(theta.cos(), theta.sin());
            }

            // Magnitude based on average drive between groups
            let avg_drive = {
                let p = if pos_count > 0 {
                    pos_drive_sum / pos_count as f32
                } else {
                    0.0
                };
                let n = if neg_count > 0 {
                    neg_drive_sum / neg_count as f32
                } else {
                    0.0
                };
                0.5 * (p + n)
            };
            let induced_mag = avg_drive * self.config.induced_field_gain;

            // Optionally override direction with foil-based direction
            // If not using foil-based direction, fall back to manual field direction (normalize if non-zero)
            let induced_dir = if self.config.induced_field_use_direction {
                dir
            } else {
                let m = manual_field.mag();
                if m > 1e-9 {
                    manual_field / m
                } else {
                    Vec2::zero()
                }
            };
            induced_field = induced_dir * induced_mag;
        }

        // Smooth induced field across frames (simple exponential)
        let alpha = self.config.induced_field_smoothing.clamp(0.0, 0.9999);
        let smoothed_induced = self.prev_induced_e_field * alpha + induced_field * (1.0 - alpha);
        self.prev_induced_e_field = smoothed_induced;

        // Compose total external: manual + smoothed induced
        self.background_e_field = manual_field + smoothed_induced;
        self.rewound_flags
            .par_iter_mut()
            .for_each(|flag| *flag = false);
        self.dt = *TIMESTEP.lock();
        self.tick_switch_charging();
        self.time += self.dt;
        let time = self.time;

        // Update global simulation time for GUI access
        *SIM_TIME.lock() = time;

        // Check for NaN values at start of step
        let nan_count = self
            .bodies
            .iter()
            .filter(|b| !b.pos.x.is_finite() || !b.pos.y.is_finite() || !b.charge.is_finite())
            .count();
        if nan_count > 0 {
            // NaN values detected at step start
        }

        // Apply group linking constraints each frame (parallel within groups, opposite between groups)
        // Skip when switch charging is running to avoid overriding active/inactive step controls
        if self.switch_run_state != RunState::Running
            && !(self.group_a.is_empty() && self.group_b.is_empty())
        {
            // Determine representatives (masters) for A and B, pick smallest id
            let master_a = self.group_a.iter().min().copied();
            let master_b = self.group_b.iter().min().copied();

            // Helper to get index by id
            let index_of = |foils: &[crate::body::foil::Foil], id: u64| -> Option<usize> {
                foils.iter().position(|f| f.id == id)
            };

            // Sync within group A
            if let Some(ma) = master_a {
                if let Some(master_idx) = index_of(&self.foils, ma) {
                    // Snapshot master values to avoid holding borrows while updating followers
                    let master_mode = self.foils[master_idx].charging_mode;
                    let (m_dc, m_ac, m_hz) = (
                        self.foils[master_idx].dc_current,
                        self.foils[master_idx].ac_current,
                        self.foils[master_idx].switch_hz,
                    );
                    let m_ctrl = self.foils[master_idx]
                        .overpotential_controller
                        .as_ref()
                        .map(|c| (c.target_ratio, c.kp, c.ki, c.kd));

                    for id in self.group_a.iter().copied().filter(|&id| id != ma) {
                        if let Some(idx) = index_of(&self.foils, id) {
                            let f = &mut self.foils[idx];
                            match master_mode {
                                crate::body::foil::ChargingMode::Current => {
                                    // Only sync current parameters if follower is also in Current mode
                                    if f.charging_mode == crate::body::foil::ChargingMode::Current {
                                        f.dc_current = m_dc;
                                        f.ac_current = m_ac;
                                        f.switch_hz = m_hz;
                                    }
                                }
                                crate::body::foil::ChargingMode::Overpotential => {
                                    // Only sync controller params if follower is also Overpotential
                                    if f.charging_mode
                                        == crate::body::foil::ChargingMode::Overpotential
                                    {
                                        if let (Some((target, kp, ki, kd)), Some(dst)) =
                                            (m_ctrl, f.overpotential_controller.as_mut())
                                        {
                                            dst.target_ratio = target;
                                            dst.kp = kp;
                                            dst.ki = ki;
                                            dst.kd = kd;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Sync within group B
            if let Some(mb) = master_b {
                if let Some(master_idx) = index_of(&self.foils, mb) {
                    // Snapshot master values
                    let master_mode = self.foils[master_idx].charging_mode;
                    let (m_dc, m_ac, m_hz) = (
                        self.foils[master_idx].dc_current,
                        self.foils[master_idx].ac_current,
                        self.foils[master_idx].switch_hz,
                    );
                    let m_ctrl = self.foils[master_idx]
                        .overpotential_controller
                        .as_ref()
                        .map(|c| (c.target_ratio, c.kp, c.ki, c.kd));

                    for id in self.group_b.iter().copied().filter(|&id| id != mb) {
                        if let Some(idx) = index_of(&self.foils, id) {
                            let f = &mut self.foils[idx];
                            match master_mode {
                                crate::body::foil::ChargingMode::Current => {
                                    if f.charging_mode == crate::body::foil::ChargingMode::Current {
                                        f.dc_current = m_dc;
                                        f.ac_current = m_ac;
                                        f.switch_hz = m_hz;
                                    }
                                }
                                crate::body::foil::ChargingMode::Overpotential => {
                                    if f.charging_mode
                                        == crate::body::foil::ChargingMode::Overpotential
                                    {
                                        if let (Some((target, kp, ki, kd)), Some(dst)) =
                                            (m_ctrl, f.overpotential_controller.as_mut())
                                        {
                                            dst.target_ratio = target;
                                            dst.kp = kp;
                                            dst.ki = ki;
                                            dst.kd = kd;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Enforce opposite across groups when both have masters
            if let (Some(ma), Some(mb)) = (master_a, master_b) {
                let mode_a = index_of(&self.foils, ma).map(|idx| self.foils[idx].charging_mode);
                let mode_b = index_of(&self.foils, mb).map(|idx| self.foils[idx].charging_mode);
                match (mode_a, mode_b) {
                    (
                        Some(crate::body::foil::ChargingMode::Current),
                        Some(crate::body::foil::ChargingMode::Current),
                    ) => {
                        if let (Some(a_idx), Some(b_idx)) =
                            (index_of(&self.foils, ma), index_of(&self.foils, mb))
                        {
                            let a_dc = self.foils[a_idx].dc_current;
                            let a_ac = self.foils[a_idx].ac_current;
                            let a_hz = self.foils[a_idx].switch_hz;
                            self.foils[b_idx].dc_current = -a_dc;
                            self.foils[b_idx].ac_current = a_ac;
                            self.foils[b_idx].switch_hz = a_hz;
                        }
                    }
                    (
                        Some(crate::body::foil::ChargingMode::Overpotential),
                        Some(crate::body::foil::ChargingMode::Overpotential),
                    ) => {
                        if let (Some(a_idx), Some(b_idx)) =
                            (index_of(&self.foils, ma), index_of(&self.foils, mb))
                        {
                            // Snapshot A controller values first
                            let a_vals = self.foils[a_idx]
                                .overpotential_controller
                                .as_ref()
                                .map(|c| (c.target_ratio, c.kp, c.ki, c.kd));
                            if let (Some((a_target, a_kp, a_ki, a_kd)), Some(b_ctrl)) =
                                (a_vals, self.foils[b_idx].overpotential_controller.as_mut())
                            {
                                b_ctrl.target_ratio = 2.0 - a_target;
                                b_ctrl.kp = a_kp;
                                b_ctrl.ki = a_ki;
                                b_ctrl.kd = a_kd;
                            }
                        }
                    }
                    // Mixed modes: leave as-is to respect manual selections
                    (Some(crate::body::foil::ChargingMode::Current), Some(_)) => {}
                    (Some(_), Some(crate::body::foil::ChargingMode::Current)) => {}
                    _ => {}
                }
            }

            // Final sync pass so followers mirror their (possibly updated) masters
            // Group A
            if let Some(ma) = master_a {
                if let Some(master_idx) = index_of(&self.foils, ma) {
                    let master_mode = self.foils[master_idx].charging_mode;
                    let (m_dc, m_ac, m_hz) = (
                        self.foils[master_idx].dc_current,
                        self.foils[master_idx].ac_current,
                        self.foils[master_idx].switch_hz,
                    );
                    let m_ctrl = self.foils[master_idx]
                        .overpotential_controller
                        .as_ref()
                        .map(|c| (c.target_ratio, c.kp, c.ki, c.kd));

                    for id in self.group_a.iter().copied().filter(|&id| id != ma) {
                        if let Some(idx) = index_of(&self.foils, id) {
                            let f = &mut self.foils[idx];
                            match master_mode {
                                crate::body::foil::ChargingMode::Current => {
                                    if f.charging_mode == crate::body::foil::ChargingMode::Current {
                                        f.dc_current = m_dc;
                                        f.ac_current = m_ac;
                                        f.switch_hz = m_hz;
                                    }
                                }
                                crate::body::foil::ChargingMode::Overpotential => {
                                    if f.charging_mode
                                        == crate::body::foil::ChargingMode::Overpotential
                                    {
                                        if let (Some((target, kp, ki, kd)), Some(dst)) =
                                            (m_ctrl, f.overpotential_controller.as_mut())
                                        {
                                            dst.target_ratio = target;
                                            dst.kp = kp;
                                            dst.ki = ki;
                                            dst.kd = kd;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Group B
            if let Some(mb) = master_b {
                if let Some(master_idx) = index_of(&self.foils, mb) {
                    let master_mode = self.foils[master_idx].charging_mode;
                    let (m_dc, m_ac, m_hz) = (
                        self.foils[master_idx].dc_current,
                        self.foils[master_idx].ac_current,
                        self.foils[master_idx].switch_hz,
                    );
                    let m_ctrl = self.foils[master_idx]
                        .overpotential_controller
                        .as_ref()
                        .map(|c| (c.target_ratio, c.kp, c.ki, c.kd));

                    for id in self.group_b.iter().copied().filter(|&id| id != mb) {
                        if let Some(idx) = index_of(&self.foils, id) {
                            let f = &mut self.foils[idx];
                            match master_mode {
                                crate::body::foil::ChargingMode::Current => {
                                    if f.charging_mode == crate::body::foil::ChargingMode::Current {
                                        f.dc_current = m_dc;
                                        f.ac_current = m_ac;
                                        f.switch_hz = m_hz;
                                    }
                                }
                                crate::body::foil::ChargingMode::Overpotential => {
                                    if f.charging_mode
                                        == crate::body::foil::ChargingMode::Overpotential
                                    {
                                        if let (Some((target, kp, ki, kd)), Some(dst)) =
                                            (m_ctrl, f.overpotential_controller.as_mut())
                                        {
                                            dst.target_ratio = target;
                                            dst.kp = kp;
                                            dst.ki = ki;
                                            dst.kd = kd;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        self.bodies.par_iter_mut().for_each(|body| {
            body.acc = Vec2::zero();
            body.az = 0.0; // Reset z-acceleration as well
        });

        forces::attract(self);
        forces::apply_polar_forces(self);
        forces::apply_lj_forces(self);
        forces::apply_repulsive_forces(self);

        // Check for NaN values after force calculations
        let nan_count = self
            .bodies
            .iter()
            .filter(|b| !b.acc.x.is_finite() || !b.acc.y.is_finite() || !b.az.is_finite())
            .count();
        if nan_count > 0 {
            // NaN values detected after force calculations
        }

        // Apply out-of-plane forces if enabled
        if self.config.enable_out_of_plane {
            super::out_of_plane::apply_out_of_plane(self);
        }

        // Check for NaN values after out-of-plane physics
        let nan_count = self
            .bodies
            .iter()
            .filter(|b| {
                !b.pos.x.is_finite()
                    || !b.pos.y.is_finite()
                    || !b.charge.is_finite()
                    || !b.z.is_finite()
            })
            .count();
        if nan_count > 0 {
            // NaN values detected after out-of-plane physics
        }

        // Apply Li+ mobility enhancement (pressure-dependent collision softening)
        // super::li_mobility::apply_li_mobility_enhancement(self);

        // Update frustration tracking for particles that may be stuck
        // Removed: frustration system replaced with simple Li+ collision softness

        self.iterate();

        // Check for NaN values after iterate
        let nan_count = self
            .bodies
            .iter()
            .filter(|b| !b.pos.x.is_finite() || !b.pos.y.is_finite() || !b.charge.is_finite())
            .count();
        if nan_count > 0 {
            // NaN values detected after iterate
        }

        let num_passes = *COLLISION_PASSES.lock();
        for _ in 1..num_passes {
            collision::collide(self);
        }
        self.update_surrounded_flags();

        // Track which bodies receive electrons from foil current this step
        let mut foil_current_recipients = vec![false; self.bodies.len()];
        // Apply foil current sources/sinks with charge conservation
        self.process_foils_with_charge_conservation(time, &mut foil_current_recipients);
        // Ensure all body charges are up-to-date after foil current changes
        self.bodies
            .par_iter_mut()
            .for_each(|body| body.update_charge_from_electrons());

        // Rebuild the quadtree after charge/electron changes so field is correct for hopping
        // Use domain-aware build to respect the configured domain boundaries
        self.quadtree
            .build_with_domain(&mut self.bodies, self.domain_width, self.domain_height);

        let quadtree = &self.quadtree;
        let len = self.bodies.len();
        let bodies_ptr = self.bodies.as_ptr();
        for i in 0..len {
            if i % 1000 == 0 && i > 0 {
                // Processing electron updates in batches
            }
            let bodies_slice = unsafe { std::slice::from_raw_parts(bodies_ptr, len) };
            let body = &mut self.bodies[i];
            body.update_electrons(
                bodies_slice,
                quadtree,
                self.background_e_field,
                self.dt,
                self.config.coulomb_constant,
            );
            body.update_charge_from_electrons();
        }

        self.perform_electron_hopping_with_exclusions(&foil_current_recipients);
        self.perform_sei_formation();

        // One-time forced bootstrap (before periodic): if not yet bootstrapped and we have liquid species with near-zero temp
        if !self.thermostat_bootstrapped {
            let liquid_temp = crate::simulation::utils::compute_liquid_temperature(&self.bodies);
            if liquid_temp <= 1e-6 {
                // Check if we actually have liquid species present
                let has_liquid = self.bodies.iter().any(|b| {
                    matches!(
                        b.species,
                        crate::body::Species::LithiumIon
                            | crate::body::Species::ElectrolyteAnion
                            | crate::body::Species::EC
                            | crate::body::Species::DMC
                    )
                });
                if has_liquid {
                    crate::simulation::utils::initialize_liquid_velocities_to_temperature(
                        &mut self.bodies,
                        self.config.temperature,
                    );
                    #[cfg(feature = "thermostat_debug")]
                    {
                        crate::simulation::thermal::tdbg!(
                            "[thermostat-force-bootstrap] frame={} assigned initial velocities at {:.2}K",
                            self.frame,
                            self.config.temperature
                        );
                    }
                    self.thermostat_bootstrapped = true;
                }
            } else {
                self.thermostat_bootstrapped = true; // Already warm
            }
        }

        // Apply periodic thermostat if enough time has passed (after ensuring bootstrap)
        if time - self.last_thermostat_time >= self.config.thermostat_interval_fs {
            #[cfg(feature = "thermostat_debug")]
            {
                crate::simulation::thermal::tdbg!(
                    "[thermo-trigger] frame={} time={:.2} calling apply_thermostat",
                    self.frame,
                    time
                );
            }
            self.apply_thermostat();
            self.last_thermostat_time = time;
        }

        // Debug: track bodies count
        #[cfg(feature = "thermostat_debug")]
        {
            if self.frame % 100 == 0 {
                crate::simulation::thermal::tdbg!(
                    "[bodies-count] frame={} bodies={} time={:.1}fs",
                    self.frame,
                    self.bodies.len(),
                    time
                );
            }
        }

        self.frame += 1;

        // Update manual measurement recorder
        let mut wrote_measurements = false;
        let simulation_time_fs = self.time;
        if let Some(recorder) = &mut self.manual_measurement_recorder {
            let results = recorder.update(
                &self.bodies,
                &self.foils,
                &self.quadtree,
                self.frame,
                simulation_time_fs,
            );
            if !results.is_empty() {
                // Update shared state for GUI display
                *crate::renderer::state::MANUAL_MEASUREMENT_RESULTS.lock() = results;
                wrote_measurements = true;
            }

            // Check for auto-pause at target time
            if let Some(target_time_fs) = recorder.config().auto_pause_time_fs {
                if simulation_time_fs >= target_time_fs {
                    println!(
                        "✓ Auto-pause triggered at {:.0} fs (target: {:.0} fs)",
                        simulation_time_fs, target_time_fs
                    );
                    crate::renderer::state::PAUSED
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        // Also write foil metrics at the same cadence, after recorder borrow ends
        if wrote_measurements {
            self.write_foil_metrics_if_due(self.frame, simulation_time_fs);
        }

        // Capture history with lightweight ring buffer approach
        // Only capture every 10 frames and keep limited history for good performance
        if self.frame % 10 == 0 {
            self.push_history_snapshot();
        }

        #[cfg(test)]
        // After all updates, print debug info for anions
        for (i, body) in self.bodies.iter().enumerate() {
            if body.species == crate::body::Species::ElectrolyteAnion {
                println!(
                    "[DEBUG] Step {}: Anion {} charge = {}, pos = {:?}, vel = {:?}",
                    self.frame, i, body.charge, body.pos, body.vel
                );
            }
        }
    }

    /// Simple history capture using VecDeque ring buffer (like the original working system)
    pub fn push_simple_history_snapshot(&mut self) {
        // Create lightweight state snapshot
        let state = crate::io::SimulationState {
            frame: self.frame,
            sim_time: self.time,
            dt: self.dt,
            bodies: self.bodies.clone(),
            foils: self.foils.clone(),
            body_to_foil: self.body_to_foil.clone(),
            config: self.config.clone(),
            switch_config: self.switch_config.clone(),
            last_thermostat_time: self.last_thermostat_time,
            domain_width: self.domain_width,
            domain_height: self.domain_height,
            domain_depth: self.domain_depth,
            switch_step: Some(self.switch_scheduler.current_step()),
            group_a: self.group_a.iter().copied().collect(),
            group_b: self.group_b.iter().copied().collect(),
        };

        // Add to ring buffer with capacity limit
        self.simple_history.push_back(state);
        while self.simple_history.len() > self.history_capacity {
            self.simple_history.pop_front();
            if self.history_cursor > 0 {
                self.history_cursor -= 1;
            }
        }

        // Update cursor to latest frame
        self.history_cursor = self.simple_history.len().saturating_sub(1);
        self.history_dirty = false;
    }

    pub fn iterate(&mut self) {
        profile_scope!("iterate");
        // Damping factor scales with timestep and is user-configurable
        let dt = self.dt;
        let base_damping = self.config.damping_base.powf(dt / 0.01);
        let domain_width = self.domain_width;
        let domain_height = self.domain_height;
        let domain_depth = self.domain_depth;
        let enable_out_of_plane = self.config.enable_out_of_plane;
        self.bodies.par_iter_mut().for_each(|body| {
            body.vel += body.acc * dt;
            let damping = base_damping * body.species.damping();
            body.vel *= damping;
            body.pos += body.vel * dt;

            // Z-coordinate integration (if out-of-plane is enabled)
            if enable_out_of_plane {
                body.vz += body.az * dt;
                body.vz *= damping; // Apply same damping to z-velocity
                body.z += body.vz * dt;

                // Z-axis boundary enforcement
                if body.z < -domain_depth {
                    body.z = -domain_depth;
                    body.vz = -body.vz;
                } else if body.z > domain_depth {
                    body.z = domain_depth;
                    body.vz = -body.vz;
                }
            }

            // X-axis boundary enforcement
            if body.pos.x < -domain_width {
                body.pos.x = -domain_width;
                body.vel.x = -body.vel.x;
            } else if body.pos.x > domain_width {
                body.pos.x = domain_width;
                body.vel.x = -body.vel.x;
            }

            // Y-axis boundary enforcement
            if body.pos.y < -domain_height {
                body.pos.y = -domain_height;
                body.vel.y = -body.vel.y;
            } else if body.pos.y > domain_height {
                body.pos.y = domain_height;
                body.vel.y = -body.vel.y;
            }
        });
    }

    /// Build default foil metrics base filename using unified scheme reflecting current settings
    fn foil_metrics_filename_base(&self) -> String {
        // If GUI provided an override, use it as-is
        if let Some(name) = crate::renderer::state::FOIL_METRICS_FILENAME_OVERRIDE
            .lock()
            .clone()
        {
            return name;
        }
        // Determine charging mode and value
        let mode_str = if matches!(self.switch_run_state, RunState::Running) {
            "SWITCH"
        } else {
            "CONV"
        };
        let (ctrl, val, step_opt) = if matches!(self.switch_run_state, RunState::Running) {
            use crate::switch_charging::Mode;
            let cfg = &self.switch_config;
            let (m, v) = if cfg.use_active_inactive_setpoints {
                let sp = if cfg.use_global_active_inactive {
                    &cfg.global_active
                } else {
                    cfg.step_active_inactive
                        .get(&0)
                        .map(|s| &s.active)
                        .unwrap_or(&cfg.global_active)
                };
                (sp.mode, sp.value)
            } else {
                let sp = cfg.step_setpoints.get(&0).cloned().unwrap_or_default();
                (sp.mode, sp.value)
            };
            let ctrl = match m {
                Mode::Current => "CC",
                Mode::Overpotential => "OP",
            };
            (ctrl.to_string(), v as f64, Some(cfg.delta_steps))
        } else {
            // Conventional/Advanced: infer from first foil (fallback to defaults)
            if let Some(foil) = self.foils.first() {
                match foil.charging_mode {
                    crate::body::foil::ChargingMode::Current => {
                        ("CC".to_string(), foil.dc_current as f64, None)
                    }
                    crate::body::foil::ChargingMode::Overpotential => {
                        let target = foil
                            .overpotential_controller
                            .as_ref()
                            .map(|c| c.target_ratio)
                            .unwrap_or(1.0);
                        ("OP".to_string(), target as f64, None)
                    }
                }
            } else {
                ("CC".to_string(), 0.05, None)
            }
        };
        let val_str = format!("{:.2}", val.abs()).replace('.', "p");
        match step_opt {
            Some(steps) => format!("Foil-based_{}_{}_{}_{}.csv", mode_str, ctrl, val_str, steps),
            None => format!("Foil-based_{}_{}_{}.csv", mode_str, ctrl, val_str),
        }
    }

    /// Ensure foil metrics CSV file is open, creating with header if needed
    fn ensure_foil_metrics_csv_open(&mut self) {
        if self.foil_metrics_csv.is_some() {
            return;
        }
        let filename = if let Some(b) = self.foil_metrics_current_base.clone() {
            b
        } else {
            let b = self.foil_metrics_filename_base();
            self.foil_metrics_current_base = Some(b.clone());
            b
        };
        // Create doe_results if needed
        let _ = std::fs::create_dir_all("doe_results");
        let path = std::path::Path::new("doe_results").join(&filename);
        match File::create(&path) {
            Ok(mut f) => {
                // Header: single row per timestep (wide format), grouped by field across foils
                // Columns: frame,time_fs, then for each group [mode_f<ID>...], [setpoint_f<ID>...], [actual_ratio_f<ID>...], [delta_electrons_f<ID>...], [li_metal_count_f<ID>...]
                // Note: we always include all groups in the header; field toggles control values (may be blank) to keep column stability.
                let mut foil_ids: Vec<u64> = self.foils.iter().map(|f| f.id).collect();
                foil_ids.sort_unstable();
                let mut header = String::from("frame,time_fs");
                // Modes
                for id in &foil_ids {
                    header.push_str(&format!(",mode_f{}", id));
                }
                // Setpoints
                for id in &foil_ids {
                    header.push_str(&format!(",setpoint_f{}", id));
                }
                // Actual ratios
                for id in &foil_ids {
                    header.push_str(&format!(",actual_ratio_f{}", id));
                }
                // Delta electrons
                for id in &foil_ids {
                    header.push_str(&format!(",delta_electrons_f{}", id));
                }
                // Li metal counts
                for id in &foil_ids {
                    header.push_str(&format!(",li_metal_count_f{}", id));
                }
                let _ = writeln!(f, "{}", header);
                let _ = f.flush();
                self.foil_metrics_csv = Some(f);
                println!("✓ Started foil metrics recording to: {}", path.display());
            }
            Err(e) => {
                eprintln!("✗ Failed to open foil metrics CSV: {}", e);
            }
        }
    }

    /// Compute Li metal count attached to a foil via connectivity (shares logic with ratio calc)
    fn li_metal_count_for_foil(&self, foil: &crate::body::foil::Foil) -> usize {
        // Build id->index map
        let id_to_index: std::collections::HashMap<u64, usize> = self
            .bodies
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id, i))
            .collect();

        let mut visited_idx: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for &body_id in &foil.body_ids {
            if let Some(&idx) = id_to_index.get(&body_id) {
                if visited_idx.insert(idx) {
                    queue.push_back(idx);
                }
            }
        }
        if queue.is_empty() {
            return 0;
        }

        let use_cell = self.use_cell_list();
        let mut li_metal_count = 0usize;
        while let Some(body_index) = queue.pop_front() {
            if body_index >= self.bodies.len() {
                continue;
            }
            let body = &self.bodies[body_index];

            if matches!(body.species, crate::body::Species::LithiumMetal) {
                li_metal_count += 1;
            }

            let connection_radius = body.radius * 2.2;
            let nearby_indices = if use_cell {
                self.cell_list
                    .find_neighbors_within(&self.bodies, body_index, connection_radius)
            } else {
                self.quadtree
                    .find_neighbors_within(&self.bodies, body_index, connection_radius)
            };
            for &other_idx in &nearby_indices {
                if other_idx >= self.bodies.len() {
                    continue;
                }
                let other_body = &self.bodies[other_idx];
                if !matches!(
                    other_body.species,
                    crate::body::Species::LithiumMetal | crate::body::Species::FoilMetal
                ) {
                    continue;
                }
                let threshold = (body.radius + other_body.radius) * 1.1;
                if (body.pos - other_body.pos).mag() <= threshold {
                    if visited_idx.insert(other_idx) {
                        queue.push_back(other_idx);
                    }
                }
            }
        }
        li_metal_count
    }

    /// Write foil metrics in wide CSV format (single row per timestep) when manual measurement cadence fires
    fn write_foil_metrics_if_due(&mut self, frame: usize, time_fs: f32) {
        // Only write when recorder exists to align with its cadence
        if self.manual_measurement_recorder.is_none() {
            return;
        }
        if !crate::renderer::state::FOIL_METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
        {
            return; // Disabled via GUI
        }
        // Precompute electron ratios for all foils (matches charging tab display)
        let mut ratio_map: std::collections::HashMap<u64, f32> = std::collections::HashMap::new();
        for foil in &self.foils {
            let ratio = self.calculate_foil_electron_ratio(foil);
            ratio_map.insert(foil.id, ratio);
        }

        // Build snapshot first to avoid borrow conflicts
        struct Row {
            idx: usize,
            foil_id: u64,
            mode_str: &'static str,
            setpoint: f32,
            actual_ratio: f32,
            delta_e: i32,
            li_metal_count: usize,
        }
        let include_li = crate::renderer::state::FOIL_METRICS_INCLUDE_LI_METAL
            .load(std::sync::atomic::Ordering::Relaxed);
        let mut rows: Vec<Row> = Vec::with_capacity(self.foils.len());
        for (i, foil) in self.foils.iter().enumerate() {
            let (mode_str, setpoint_val) = match foil.charging_mode {
                crate::body::foil::ChargingMode::Current => ("CC", foil.dc_current),
                crate::body::foil::ChargingMode::Overpotential => {
                    let target = foil
                        .overpotential_controller
                        .as_ref()
                        .map(|c| c.target_ratio)
                        .unwrap_or(1.0);
                    ("OP", target)
                }
            };
            let actual_ratio = ratio_map.get(&foil.id).copied().unwrap_or(1.0);
            let delta_e = foil.electron_delta_since_measure;
            let li_metal_count = if include_li {
                self.li_metal_count_for_foil(foil)
            } else {
                0
            };
            rows.push(Row {
                idx: i,
                foil_id: foil.id,
                mode_str,
                setpoint: setpoint_val,
                actual_ratio,
                delta_e,
                li_metal_count,
            });
        }

        // Write a single wide row
        self.ensure_foil_metrics_csv_open();
        if let Some(file) = &mut self.foil_metrics_csv {
            // Sort by foil id to match header order
            let mut rows_sorted: Vec<&Row> = rows.iter().collect();
            rows_sorted.sort_by_key(|r| r.foil_id);

            let include_set = crate::renderer::state::FOIL_METRICS_INCLUDE_SETPOINT
                .load(std::sync::atomic::Ordering::Relaxed);
            let include_act = crate::renderer::state::FOIL_METRICS_INCLUDE_ACTUAL_RATIO
                .load(std::sync::atomic::Ordering::Relaxed);
            let include_del = crate::renderer::state::FOIL_METRICS_INCLUDE_DELTA_ELECTRONS
                .load(std::sync::atomic::Ordering::Relaxed);
            let include_li = crate::renderer::state::FOIL_METRICS_INCLUDE_LI_METAL
                .load(std::sync::atomic::Ordering::Relaxed);

            // Start with frame and time
            let mut line = format!("{},{}", frame, time_fs);
            // Modes
            for r in &rows_sorted {
                line.push_str(&format!(",{}", r.mode_str));
            }
            // Setpoints (conditional values)
            for r in &rows_sorted {
                if include_set {
                    line.push_str(&format!(",{:.6}", r.setpoint));
                } else {
                    line.push_str(",");
                }
            }
            // Actual ratios
            for r in &rows_sorted {
                if include_act {
                    line.push_str(&format!(",{:.6}", r.actual_ratio));
                } else {
                    line.push_str(",");
                }
            }
            // Delta electrons
            for r in &rows_sorted {
                if include_del {
                    line.push_str(&format!(",{}", r.delta_e));
                } else {
                    line.push_str(",");
                }
            }
            // Li metal counts
            for r in &rows_sorted {
                if include_li {
                    line.push_str(&format!(",{}", r.li_metal_count));
                } else {
                    line.push_str(",");
                }
            }

            let _ = writeln!(file, "{}", line);
            let _ = file.flush();
        }

        // Reset deltas after writing
        for r in &rows {
            if let Some(f) = self.foils.get_mut(r.idx) {
                f.electron_delta_since_measure = 0;
            }
        }
    }

    pub fn use_cell_list(&self) -> bool {
        let area = (2.0 * self.domain_width) * (2.0 * self.domain_height);
        let density = self.bodies.len() as f32 / area;
        density > self.config.cell_list_density_threshold
    }

    /// Calculate the proper foil electron ratio (same as diagnostic)
    /// This is the ratio of actual electrons to neutral electron count in the foil network
    /// OPTIMIZED: Uses spatial data structures and index-based BFS to avoid O(N) id scans
    fn calculate_foil_electron_ratio(&self, foil: &crate::body::foil::Foil) -> f32 {
        let mut total_electrons = 0usize;
        let mut total_neutral = 0usize;

        // Build a local id->index map to avoid repeated linear scans
        // Note: We keep this local to avoid mutability on self; for further speed,
        // consider caching once per step if many foils use this path.
        let id_to_index: std::collections::HashMap<u64, usize> = self
            .bodies
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id, i))
            .collect();

        // Use BFS with indices for optimization
        let mut visited_idx: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

        // Seed the queue with all bodies belonging to this foil
        for &body_id in &foil.body_ids {
            if let Some(&idx) = id_to_index.get(&body_id) {
                if visited_idx.insert(idx) {
                    queue.push_back(idx);
                }
            }
        }

        if queue.is_empty() {
            return 1.0; // No bodies -> neutral
        }

        let use_cell = self.use_cell_list();

        // BFS to find all connected metal bodies using spatial queries
        while let Some(body_index) = queue.pop_front() {
            if body_index >= self.bodies.len() {
                continue;
            }
            let body = &self.bodies[body_index];

            // Count electrons in this body
            total_electrons += body.electrons.len();
            total_neutral += body.neutral_electron_count();

            // Find connected neighbors using spatial data structures
            let connection_radius = body.radius * 2.2; // Search radius for connected bodies
            let nearby_indices = if use_cell {
                self.cell_list
                    .find_neighbors_within(&self.bodies, body_index, connection_radius)
            } else {
                self.quadtree
                    .find_neighbors_within(&self.bodies, body_index, connection_radius)
            };

            // Check each nearby body for connection
            for &other_idx in &nearby_indices {
                if other_idx >= self.bodies.len() {
                    continue;
                }
                let other_body = &self.bodies[other_idx];

                // Only consider metal bodies
                if !matches!(
                    other_body.species,
                    crate::body::Species::LithiumMetal | crate::body::Species::FoilMetal
                ) {
                    continue;
                }

                // Check if actually connected (precise distance check)
                let threshold = (body.radius + other_body.radius) * 1.1;
                if (body.pos - other_body.pos).mag() <= threshold {
                    if visited_idx.insert(other_idx) {
                        queue.push_back(other_idx);
                    }
                }
            }
        }

        if total_neutral > 0 {
            let ratio = total_electrons as f32 / total_neutral as f32;
            // Debug output occasionally
            if rand::random::<f32>() < 0.001 {
                println!(
                    "Foil {} electron ratio: {:.3} (electrons: {}, neutral: {})",
                    foil.id, ratio, total_electrons, total_neutral
                );
            }
            ratio
        } else {
            1.0 // Neutral if no reference
        }
    }

    /// Update `surrounded_by_metal` for all bodies using either the cell list or quadtree.
    pub fn update_surrounded_flags(&mut self) {
        if self.bodies.is_empty() {
            return;
        }
        let use_cell = self.use_cell_list();
        let neighbor_radius = crate::species::max_lj_cutoff();
        if use_cell {
            self.cell_list.cell_size = neighbor_radius;
            self.cell_list.rebuild(&self.bodies);
        } else {
            self.quadtree.build_with_domain(
                &mut self.bodies,
                self.domain_width,
                self.domain_height,
            );
        }
        let quadtree = &self.quadtree;
        let cell_list = &self.cell_list;
        let frame = self.frame;
        // Collect the data needed for immutable borrow
        let bodies_snapshot: Vec<_> = self.bodies.iter().map(|b| b.clone()).collect();
        for (i, body) in self.bodies.iter_mut().enumerate() {
            body.maybe_update_surrounded(i, &bodies_snapshot, quadtree, cell_list, use_cell, frame);
        }
    }

    fn effective_current(
        foil: &mut crate::body::foil::Foil,
        time: f32,
        actual_ratio: Option<f32>,
        dt: f32,
        _step: u64,
    ) -> f32 {
        match foil.charging_mode {
            crate::body::foil::ChargingMode::Current => {
                // Traditional current control mode
                let mut current = foil.dc_current;
                if foil.switch_hz > 0.0 {
                    let ac_component = if (time * foil.switch_hz) % 1.0 < 0.5 {
                        foil.ac_current
                    } else {
                        -foil.ac_current
                    };
                    current += ac_component;
                }
                current
            }
            crate::body::foil::ChargingMode::Overpotential => {
                // Check if this is a master foil (has PID controller) or slave foil (no controller)
                if let Some(_controller) = &foil.overpotential_controller {
                    // Master foil - use PID controller
                    if let Some(ratio) = actual_ratio {
                        let pid_current = foil.compute_overpotential_current(ratio, dt);

                        // Still support AC component on top of PID-controlled DC current
                        let mut current = pid_current;
                        if foil.switch_hz > 0.0 {
                            let ac_component = if (time * foil.switch_hz) % 1.0 < 0.5 {
                                foil.ac_current
                            } else {
                                -foil.ac_current
                            };
                            current += ac_component;
                        }
                        current
                    } else {
                        // Fallback to DC current if no ratio available
                        foil.dc_current
                    }
                } else {
                    // Slave foil - use stored slave current (set by master)
                    let mut current = foil.slave_overpotential_current;
                    if foil.switch_hz > 0.0 {
                        let ac_component = if (time * foil.switch_hz) % 1.0 < 0.5 {
                            foil.ac_current
                        } else {
                            -foil.ac_current
                        };
                        current += ac_component;
                    }
                    current
                }
            }
        }
    }

    /// Process foils with charge conservation - electrons can only be added if another foil removes one
    fn process_foils_with_charge_conservation(&mut self, time: f32, recipients: &mut [bool]) {
        let dt = self.dt;
        let mut rng = rand::rng();

        // Calculate proper foil electron ratios for overpotential charging foils
        // Compute for master foils (with controllers) regardless of target, so neutral can still correct
        let mut electron_ratios = std::collections::HashMap::new();
        for foil in &self.foils {
            if matches!(
                foil.charging_mode,
                crate::body::foil::ChargingMode::Overpotential
            ) && foil.overpotential_controller.is_some()
            {
                let ratio = self.calculate_foil_electron_ratio(foil);
                electron_ratios.insert(foil.id, ratio);
            }
        }

        // Handle overpotential master-slave relationships
        // First pass: compute PID outputs for master foils ONLY (including neutral targets)
        let mut master_outputs = std::collections::HashMap::new();
        for i in 0..self.foils.len() {
            if matches!(
                self.foils[i].charging_mode,
                crate::body::foil::ChargingMode::Overpotential
            ) && self.foils[i].overpotential_controller.is_some()
            {
                let foil_id = self.foils[i].id;
                if let Some(actual_ratio) = electron_ratios.get(&foil_id).copied() {
                    let master_current =
                        self.foils[i].compute_overpotential_current(actual_ratio, dt);
                    master_outputs.insert(foil_id, master_current);
                }
            }
        }

        // Second pass: set slave currents based on master currents
        for i in 0..self.foils.len() {
            if matches!(
                self.foils[i].charging_mode,
                crate::body::foil::ChargingMode::Overpotential
            ) {
                // Check if this is a slave foil (no controller but has a linked master)
                if self.foils[i].overpotential_controller.is_none()
                    && self.foils[i].link_id.is_some()
                {
                    let master_id = self.foils[i].link_id.unwrap(); // Slave's master is its linked foil
                    if let Some(&master_current) = master_outputs.get(&master_id) {
                        // Determine current sign based on link mode
                        let slave_current = match self.foils[i].mode {
                            crate::body::foil::LinkMode::Parallel => master_current,
                            crate::body::foil::LinkMode::Opposite => -master_current,
                        };

                        self.foils[i].slave_overpotential_current = slave_current;
                    }
                }
            }
        }

        // DIRECT ELECTRON MANIPULATION for overpotential mode (bypasses current-based system)
        self.process_overpotential_direct_electron_control(&electron_ratios, &mut rng, recipients);

        // Traditional current-based processing for non-overpotential foils
        self.process_current_based_foils(time, dt, &electron_ratios, recipients);

        // Handle linked foils for current mode (ensure equal/opposite currents)
        // Process linked foils that are not in overpotential mode or are not slaves
        let mut processed_links = std::collections::HashSet::new();
        for i in 0..self.foils.len() {
            if let Some(link_id) = self.foils[i].link_id {
                // Create a unique pair identifier to avoid processing the same link twice
                let pair_key = if self.foils[i].id < link_id {
                    (self.foils[i].id, link_id)
                } else {
                    (link_id, self.foils[i].id)
                };

                if processed_links.contains(&pair_key) {
                    continue; // Already processed this link pair
                }
                processed_links.insert(pair_key);

                if let Some(linked_foil_idx) = self.foils.iter().position(|f| f.id == link_id) {
                    // For current mode linked foils, synchronize their currents
                    if matches!(
                        self.foils[i].charging_mode,
                        crate::body::foil::ChargingMode::Current
                    ) && matches!(
                        self.foils[linked_foil_idx].charging_mode,
                        crate::body::foil::ChargingMode::Current
                    ) {
                        // Use the current from the first foil as the reference
                        let reference_current = self.foils[i].dc_current;

                        match self.foils[i].mode {
                            crate::body::foil::LinkMode::Parallel => {
                                // Same current for parallel mode
                                self.foils[linked_foil_idx].dc_current = reference_current;
                            }
                            crate::body::foil::LinkMode::Opposite => {
                                // Opposite current for opposite mode
                                self.foils[linked_foil_idx].dc_current = -reference_current;
                            }
                        }
                    }
                    // Note: Overpotential mode linked foils are handled by the master-slave system above
                }
            }
        }
    }

    /// Process linked pair with charge conservation (similar to existing but renamed for clarity)
    fn process_linked_pair_conservative(
        &mut self,
        a: usize,
        b: usize,
        rng: &mut rand::rngs::ThreadRng,
        recipients: &mut [bool],
    ) {
        let mode = self.foils[a].mode;
        loop {
            match mode {
                LinkMode::Parallel => {
                    if self.foils[a].accum >= 1.0 && self.foils[b].accum >= 1.0 {
                        if self.foil_can_add(a) && self.foil_can_add(b) {
                            if self.try_add_electron(a, rng, recipients)
                                && self.try_add_electron(b, rng, recipients)
                            {
                                self.foils[a].accum -= 1.0;
                                self.foils[b].accum -= 1.0;
                                continue;
                            }
                        }
                    }
                    if self.foils[a].accum <= -1.0 && self.foils[b].accum <= -1.0 {
                        if self.foil_can_remove(a) && self.foil_can_remove(b) {
                            if self.try_remove_electron(a, rng, recipients)
                                && self.try_remove_electron(b, rng, recipients)
                            {
                                self.foils[a].accum += 1.0;
                                self.foils[b].accum += 1.0;
                                continue;
                            }
                        }
                    }
                    break;
                }
                LinkMode::Opposite => {
                    if self.foils[a].accum >= 1.0 && self.foils[b].accum <= -1.0 {
                        if self.foil_can_add(a) && self.foil_can_remove(b) {
                            if self.try_add_electron(a, rng, recipients)
                                && self.try_remove_electron(b, rng, recipients)
                            {
                                self.foils[a].accum -= 1.0;
                                self.foils[b].accum += 1.0;
                                continue;
                            }
                        }
                    }
                    if self.foils[a].accum <= -1.0 && self.foils[b].accum >= 1.0 {
                        if self.foil_can_remove(a) && self.foil_can_add(b) {
                            if self.try_remove_electron(a, rng, recipients)
                                && self.try_add_electron(b, rng, recipients)
                            {
                                self.foils[a].accum += 1.0;
                                self.foils[b].accum -= 1.0;
                                continue;
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    fn foil_can_add(&self, idx: usize) -> bool {
        let foil = &self.foils[idx];
        foil.body_ids.iter().any(|&id| {
            self.bodies.iter().any(|b| {
                b.id == id
                    && b.species == Species::FoilMetal
                    && b.electrons.len() < crate::config::FOIL_MAX_ELECTRONS
            })
        })
    }

    fn foil_can_remove(&self, idx: usize) -> bool {
        let foil = &self.foils[idx];
        foil.body_ids.iter().any(|&id| {
            self.bodies
                .iter()
                .any(|b| b.id == id && b.species == Species::FoilMetal && !b.electrons.is_empty())
        })
    }

    fn try_add_electron(
        &mut self,
        idx: usize,
        rng: &mut rand::rngs::ThreadRng,
        recipients: &mut [bool],
    ) -> bool {
        let foil = &mut self.foils[idx];
        if let Some(&id) = foil.body_ids.as_slice().choose(rng) {
            if let Some((body_idx, body)) = self
                .bodies
                .iter_mut()
                .enumerate()
                .find(|(_, b)| b.id == id && b.species == Species::FoilMetal)
            {
                if body.electrons.len() < crate::config::FOIL_MAX_ELECTRONS {
                    body.electrons.push(Electron {
                        rel_pos: Vec2::zero(),
                        vel: Vec2::zero(),
                    });
                    recipients[body_idx] = true;
                    // Track signed electron change since last measurement
                    foil.electron_delta_since_measure += 1;
                    return true;
                }
            }
        }
        false
    }

    fn try_remove_electron(
        &mut self,
        idx: usize,
        rng: &mut rand::rngs::ThreadRng,
        recipients: &mut [bool],
    ) -> bool {
        let foil = &mut self.foils[idx];
        if let Some(&id) = foil.body_ids.as_slice().choose(rng) {
            if let Some((body_idx, body)) = self
                .bodies
                .iter_mut()
                .enumerate()
                .find(|(_, b)| b.id == id && b.species == Species::FoilMetal)
            {
                if !body.electrons.is_empty() {
                    body.electrons.pop();
                    recipients[body_idx] = true;
                    // Track signed electron change since last measurement
                    foil.electron_delta_since_measure -= 1;
                    return true;
                }
            }
        }
        false
    }

    // Fast index-based helpers to avoid O(N) id scans. Only used on hot overpotential path.
    #[inline]
    fn foil_can_add_idxmap(
        &self,
        idx: usize,
        id_to_index: &std::collections::HashMap<u64, usize>,
    ) -> bool {
        let foil = &self.foils[idx];
        for &id in &foil.body_ids {
            if let Some(&bi) = id_to_index.get(&id) {
                if bi < self.bodies.len() {
                    let b = &self.bodies[bi];
                    if b.species == Species::FoilMetal
                        && b.electrons.len() < crate::config::FOIL_MAX_ELECTRONS
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[inline]
    fn foil_can_remove_idxmap(
        &self,
        idx: usize,
        id_to_index: &std::collections::HashMap<u64, usize>,
    ) -> bool {
        let foil = &self.foils[idx];
        for &id in &foil.body_ids {
            if let Some(&bi) = id_to_index.get(&id) {
                if bi < self.bodies.len() {
                    let b = &self.bodies[bi];
                    if b.species == Species::FoilMetal && !b.electrons.is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[inline]
    fn try_add_electron_idxmap(
        &mut self,
        idx: usize,
        rng: &mut rand::rngs::ThreadRng,
        recipients: &mut [bool],
        id_to_index: &std::collections::HashMap<u64, usize>,
    ) -> bool {
        let foil = &mut self.foils[idx];
        if let Some(&id) = foil.body_ids.as_slice().choose(rng) {
            if let Some(&body_idx) = id_to_index.get(&id) {
                if body_idx < self.bodies.len() {
                    let body = &mut self.bodies[body_idx];
                    if body.species == Species::FoilMetal
                        && body.electrons.len() < crate::config::FOIL_MAX_ELECTRONS
                    {
                        body.electrons.push(Electron {
                            rel_pos: Vec2::zero(),
                            vel: Vec2::zero(),
                        });
                        recipients[body_idx] = true;
                        // Track signed electron change since last measurement
                        foil.electron_delta_since_measure += 1;
                        return true;
                    }
                }
            }
        }
        false
    }

    #[inline]
    fn try_remove_electron_idxmap(
        &mut self,
        idx: usize,
        rng: &mut rand::rngs::ThreadRng,
        recipients: &mut [bool],
        id_to_index: &std::collections::HashMap<u64, usize>,
    ) -> bool {
        let foil = &mut self.foils[idx];
        if let Some(&id) = foil.body_ids.as_slice().choose(rng) {
            if let Some(&body_idx) = id_to_index.get(&id) {
                if body_idx < self.bodies.len() {
                    let body = &mut self.bodies[body_idx];
                    if body.species == Species::FoilMetal && !body.electrons.is_empty() {
                        body.electrons.pop();
                        recipients[body_idx] = true;
                        // Track signed electron change since last measurement
                        foil.electron_delta_since_measure -= 1;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Direct electron manipulation for overpotential mode - bypasses current-based accumulator system
    fn process_overpotential_direct_electron_control(
        &mut self,
        electron_ratios: &std::collections::HashMap<u64, f32>,
        rng: &mut rand::rngs::ThreadRng,
        recipients: &mut [bool],
    ) {
        // Build a single id->index map for this hot path
        let id_to_index: std::collections::HashMap<u64, usize> = self
            .bodies
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id, i))
            .collect();

        // Process each overpotential foil individually for direct electron control
        for i in 0..self.foils.len() {
            if !matches!(
                self.foils[i].charging_mode,
                crate::body::foil::ChargingMode::Overpotential
            ) {
                continue;
            }

            // Get the effective current for this foil (master PID output or slave assigned current)
            let foil_id = self.foils[i].id;
            let _actual_ratio = electron_ratios.get(&foil_id).copied(); // Keep for potential future use
            let effective_current =
                if let Some(controller) = &self.foils[i].overpotential_controller {
                    // Master foil - use PID output
                    controller.last_output_current
                } else {
                    // Slave foil - use assigned current
                    self.foils[i].slave_overpotential_current
                };

            // Interpret controller output as electrons-per-step for responsiveness
            // Positive value = add electrons, negative = remove electrons
            let electron_transfer_rate = effective_current;

            // Direct electron manipulation - no accumulator, no charge conservation constraints
            if electron_transfer_rate > 0.0 {
                // Add electrons directly
                let num_electrons_to_add = electron_transfer_rate.floor() as i32;
                let fractional_part = electron_transfer_rate.fract();

                // Add whole electrons
                for _ in 0..num_electrons_to_add {
                    if self.foil_can_add_idxmap(i, &id_to_index) {
                        self.try_add_electron_idxmap(i, rng, recipients, &id_to_index);
                    }
                }

                // Handle fractional electron with probability
                if rand::random::<f32>() < fractional_part {
                    if self.foil_can_add_idxmap(i, &id_to_index) {
                        self.try_add_electron_idxmap(i, rng, recipients, &id_to_index);
                    }
                }
            } else if electron_transfer_rate < 0.0 {
                // Remove electrons directly
                let num_electrons_to_remove = (-electron_transfer_rate).floor() as i32;
                let fractional_part = (-electron_transfer_rate).fract();

                // Remove whole electrons
                for _ in 0..num_electrons_to_remove {
                    if self.foil_can_remove_idxmap(i, &id_to_index) {
                        self.try_remove_electron_idxmap(i, rng, recipients, &id_to_index);
                    }
                }

                // Handle fractional electron with probability
                if rand::random::<f32>() < fractional_part {
                    if self.foil_can_remove_idxmap(i, &id_to_index) {
                        self.try_remove_electron_idxmap(i, rng, recipients, &id_to_index);
                    }
                }
            }
        }
    }

    /// Process traditional current-based foils (non-overpotential mode)
    fn process_current_based_foils(
        &mut self,
        time: f32,
        dt: f32,
        electron_ratios: &std::collections::HashMap<u64, f32>,
        recipients: &mut [bool],
    ) {
        let mut rng = rand::rng();

        // Handle linked foils for current mode (ensure equal/opposite currents)
        // Process linked foils that are not in overpotential mode or are not slaves
        let mut processed_links = std::collections::HashSet::new();
        for i in 0..self.foils.len() {
            // Skip overpotential foils - they are handled by direct electron control
            if matches!(
                self.foils[i].charging_mode,
                crate::body::foil::ChargingMode::Overpotential
            ) {
                continue;
            }

            if let Some(link_id) = self.foils[i].link_id {
                // Create a unique pair identifier to avoid processing the same link twice
                let pair_key = if self.foils[i].id < link_id {
                    (self.foils[i].id, link_id)
                } else {
                    (link_id, self.foils[i].id)
                };

                if processed_links.contains(&pair_key) {
                    continue; // Already processed this link pair
                }
                processed_links.insert(pair_key);

                if let Some(linked_foil_idx) = self.foils.iter().position(|f| f.id == link_id) {
                    // For current mode linked foils, synchronize their currents
                    if matches!(
                        self.foils[i].charging_mode,
                        crate::body::foil::ChargingMode::Current
                    ) && matches!(
                        self.foils[linked_foil_idx].charging_mode,
                        crate::body::foil::ChargingMode::Current
                    ) {
                        // Use the current from the first foil as the reference
                        let reference_current = self.foils[i].dc_current;

                        match self.foils[i].mode {
                            crate::body::foil::LinkMode::Parallel => {
                                // Same current for parallel mode
                                self.foils[linked_foil_idx].dc_current = reference_current;
                            }
                            crate::body::foil::LinkMode::Opposite => {
                                // Opposite current for opposite mode
                                self.foils[linked_foil_idx].dc_current = -reference_current;
                            }
                        }
                    }
                    // Note: Overpotential mode linked foils are handled by the master-slave system above
                }
            }
        }

        // Update all accumulators for current-mode foils
        for i in 0..self.foils.len() {
            // Skip overpotential foils
            if matches!(
                self.foils[i].charging_mode,
                crate::body::foil::ChargingMode::Overpotential
            ) {
                continue;
            }

            let foil_id = self.foils[i].id;
            let actual_ratio = electron_ratios.get(&foil_id).copied();
            let current = Self::effective_current(
                &mut self.foils[i],
                time,
                actual_ratio,
                dt,
                self.frame as u64,
            );
            self.foils[i].accum += current * dt;
        }

        // Handle linked pairs first (they have priority and built-in charge conservation)
        let mut visited = vec![false; self.foils.len()];
        for i in 0..self.foils.len() {
            // Skip overpotential foils
            if matches!(
                self.foils[i].charging_mode,
                crate::body::foil::ChargingMode::Overpotential
            ) {
                visited[i] = true; // Mark as visited to skip
                continue;
            }

            if visited[i] {
                continue;
            }
            if let Some(link_id) = self.foils[i].link_id {
                if let Some(j) = self.foils.iter().position(|f| f.id == link_id) {
                    // Also skip if linked foil is overpotential mode
                    if matches!(
                        self.foils[j].charging_mode,
                        crate::body::foil::ChargingMode::Overpotential
                    ) {
                        visited[j] = true;
                        continue;
                    }

                    if !visited[j] {
                        visited[i] = true;
                        visited[j] = true;
                        self.process_linked_pair_conservative(i, j, &mut rng, recipients);
                        continue;
                    }
                }
            }
        }

        // For unlinked current-mode foils, enforce global charge conservation
        let mut add_ready: Vec<usize> = Vec::new();
        let mut remove_ready: Vec<usize> = Vec::new();

        for i in 0..self.foils.len() {
            if visited[i] {
                continue;
            }

            // Skip overpotential foils
            if matches!(
                self.foils[i].charging_mode,
                crate::body::foil::ChargingMode::Overpotential
            ) {
                continue;
            }

            // Check if foil is ready to add electrons (positive accumulator)
            if self.foils[i].accum >= 1.0 && self.foil_can_add(i) {
                add_ready.push(i);
            }
            // Check if foil is ready to remove electrons (negative accumulator)
            else if self.foils[i].accum <= -1.0 && self.foil_can_remove(i) {
                remove_ready.push(i);
            }
        }

        // Shuffle to ensure random pairing
        add_ready.shuffle(&mut rng);
        remove_ready.shuffle(&mut rng);

        // Process charge-conserving pairs: one adds, one removes
        let num_pairs = add_ready.len().min(remove_ready.len());

        for pair_idx in 0..num_pairs {
            let add_foil_idx = add_ready[pair_idx];
            let remove_foil_idx = remove_ready[pair_idx];

            // Attempt the charge-conserving pair operation
            if self.try_add_electron(add_foil_idx, &mut rng, recipients)
                && self.try_remove_electron(remove_foil_idx, &mut rng, recipients)
            {
                self.foils[add_foil_idx].accum -= 1.0;
                self.foils[remove_foil_idx].accum += 1.0;
            }
        }
    }

    /// Perform SEI formation for eligible bodies in the simulation.
    /// This is called after the main update step to allow for charge-based reactions.
    pub fn perform_sei_formation(&mut self) {
        if !self.config.sei_formation_enabled {
            return;
        }
        profile_scope!("sei_formation");

        let dt = self.dt;
        let prob_base = self.config.sei_formation_probability;
        let bias = self.config.sei_formation_bias;

        // Collect indices of bodies to convert to SEI
        // We use a parallel iterator to find candidates, but we need to be careful about
        // accessing the quadtree which is read-only here.
        // Since we need to mutate bodies later, we collect indices first.
        
        let conversions: Vec<usize> = self.bodies.par_iter().enumerate().filter_map(|(i, body)| {
             match body.species {
                Species::EC | Species::DMC | Species::VC | Species::FEC | Species::EMC => {
                    // Check neighbors using quadtree
                    // We use a slightly larger radius than contact to detect proximity
                    let search_radius = body.radius * 2.5; 
                    let neighbors = self.quadtree.find_neighbors_within(&self.bodies, i, search_radius);
                    
                    for &neighbor_idx in &neighbors {
                        if neighbor_idx == i { continue; }
                        // Safety check for index bounds (though quadtree should be consistent)
                        if neighbor_idx >= self.bodies.len() { continue; }
                        
                        let neighbor = &self.bodies[neighbor_idx];
                        
                        if matches!(neighbor.species, Species::LithiumMetal | Species::FoilMetal) {
                            // Found a metal surface neighbor
                            
                            // Probability logic:
                            // "rate of change... related to the charge"
                            // If the metal is negatively charged (excess electrons), it promotes reduction.
                            // We use the magnitude of negative charge.
                            
                            let charge_factor = if neighbor.charge < -0.1 {
                                neighbor.charge.abs() * bias
                            } else {
                                0.0
                            };
                            
                            // Base probability scaled by charge factor and timestep
                            // If charge_factor is 0, prob is 0.
                            let prob = prob_base * charge_factor * dt;
                            
                            if prob > 0.0 && rand::random::<f32>() < prob {
                                return Some(i);
                            }
                        }
                    }
                    None
                },
                _ => None,
            }
        }).collect();

        // Apply conversions
        for idx in conversions {
            if idx < self.bodies.len() {
                let body = &mut self.bodies[idx];
                // Double check species hasn't changed (though unlikely in single threaded apply)
                if matches!(body.species, Species::EC | Species::DMC | Species::VC | Species::FEC | Species::EMC) {
                    body.species = Species::SEI;
                    body.update_species(); // Update properties (mass, radius, etc.)
                    
                    // Ensure SEI is neutral and has no electrons
                    body.charge = 0.0;
                    body.electrons.clear();
                    body.vel *= 0.1; // Slow down significantly upon formation (solidification)
                }
            }
        }
    }
}

#[cfg(test)]
mod charge_conservation_tests {
    use super::*;
    use crate::body::foil::Foil;

    fn create_test_simulation_with_foils() -> Simulation {
        let mut sim = Simulation::new();

        // Create test foil bodies
        let foil_body1 = Body::new(
            Vec2::new(-10.0, 0.0),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::FoilMetal,
        );
        let foil_body2 = Body::new(
            Vec2::new(10.0, 0.0),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::FoilMetal,
        );

        sim.bodies.push(foil_body1);
        sim.bodies.push(foil_body2);

        // Create foils with positive and negative currents
        let mut foil1 = Foil::new(vec![sim.bodies[0].id], Vec2::zero(), 1.0, 1.0, 2.0, 0.0);
        foil1.accum = 1.5; // Ready to add electrons

        let mut foil2 = Foil::new(vec![sim.bodies[1].id], Vec2::zero(), 1.0, 1.0, -2.0, 0.0);
        foil2.accum = -1.5; // Ready to remove electrons

        sim.foils.push(foil1);
        sim.foils.push(foil2);

        sim
    }

    #[test]
    fn test_single_foil_with_positive_accum_does_nothing() {
        let mut sim = Simulation::new();

        // Create a single foil body
        let foil_body = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::FoilMetal,
        );
        sim.bodies.push(foil_body);

        // Create a single foil with positive current (wants to add electrons)
        let mut foil = Foil::new(vec![sim.bodies[0].id], Vec2::zero(), 1.0, 1.0, 2.0, 0.0);
        foil.accum = 1.5; // Ready to add electrons
        sim.foils.push(foil);

        let initial_electron_count = sim.bodies[0].electrons.len();
        let initial_accum = sim.foils[0].accum;
        let dt = sim.dt;
        let current = 2.0; // foil dc_current

        // Process foils - should do nothing since no partner to remove electrons
        let mut recipients = vec![false; sim.bodies.len()];
        sim.process_foils_with_charge_conservation(0.0, &mut recipients);

        // Verify no electrons were added but accumulator updated by current
        assert_eq!(
            sim.bodies[0].electrons.len(),
            initial_electron_count,
            "Single foil should not add electrons without a removal partner"
        );
        assert_eq!(
            sim.foils[0].accum,
            initial_accum + current * dt,
            "Accumulator should be updated by current flow even when no operations occur"
        );
        assert!(!recipients[0], "Body should not be marked as recipient");
    }

    #[test]
    fn test_single_foil_with_negative_accum_does_nothing() {
        let mut sim = Simulation::new();

        // Create a single foil body with an electron to remove
        let mut foil_body = Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::FoilMetal,
        );
        foil_body.electrons.push(Electron {
            rel_pos: Vec2::zero(),
            vel: Vec2::zero(),
        });
        sim.bodies.push(foil_body);

        // Create a single foil with negative current (wants to remove electrons)
        let mut foil = Foil::new(vec![sim.bodies[0].id], Vec2::zero(), 1.0, 1.0, -2.0, 0.0);
        foil.accum = -1.5; // Ready to remove electrons
        sim.foils.push(foil);

        let initial_electron_count = sim.bodies[0].electrons.len();
        let initial_accum = sim.foils[0].accum;
        let dt = sim.dt;
        let current = -2.0; // foil dc_current

        // Process foils - should do nothing since no partner to add electrons
        let mut recipients = vec![false; sim.bodies.len()];
        sim.process_foils_with_charge_conservation(0.0, &mut recipients);

        // Verify no electrons were removed but accumulator updated by current
        assert_eq!(
            sim.bodies[0].electrons.len(),
            initial_electron_count,
            "Single foil should not remove electrons without an addition partner"
        );
        assert_eq!(
            sim.foils[0].accum,
            initial_accum + current * dt,
            "Accumulator should be updated by current flow even when no operations occur"
        );
        assert!(!recipients[0], "Body should not be marked as recipient");
    }

    #[test]
    fn test_paired_foils_execute_charge_conserving_operations() {
        let mut sim = create_test_simulation_with_foils();

        let initial_electrons_foil1 = sim.bodies[0].electrons.len();
        let initial_electrons_foil2 = sim.bodies[1].electrons.len();
        let initial_accum1 = sim.foils[0].accum;
        let initial_accum2 = sim.foils[1].accum;
        let dt = sim.dt;
        let current1 = 2.0; // foil1 dc_current
        let current2 = -2.0; // foil2 dc_current

        // Add an electron to foil2 so it can be removed
        sim.bodies[1].electrons.push(Electron {
            rel_pos: Vec2::zero(),
            vel: Vec2::zero(),
        });

        // Process foils - should execute charge-conserving pair
        let mut recipients = vec![false; sim.bodies.len()];
        sim.process_foils_with_charge_conservation(0.0, &mut recipients);

        // Verify charge-conserving operations occurred
        assert_eq!(
            sim.bodies[0].electrons.len(),
            initial_electrons_foil1 + 1,
            "Foil 1 should have gained an electron"
        );
        assert_eq!(
            sim.bodies[1].electrons.len(),
            initial_electrons_foil2, // Had 1, lost 1, still 0
            "Foil 2 should have lost an electron"
        );

        // Verify accumulators: updated by current, then decremented/incremented by operations
        let expected_accum1 = initial_accum1 + current1 * dt - 1.0;
        let expected_accum2 = initial_accum2 + current2 * dt + 1.0;
        assert_eq!(
            sim.foils[0].accum, expected_accum1,
            "Foil 1 accumulator should be updated by current then decremented by operation"
        );
        assert_eq!(
            sim.foils[1].accum, expected_accum2,
            "Foil 2 accumulator should be updated by current then incremented by operation"
        );

        // Verify recipients were marked
        assert!(recipients[0], "Foil 1 body should be marked as recipient");
        assert!(recipients[1], "Foil 2 body should be marked as recipient");
    }

    #[test]
    fn test_total_electron_count_conservation() {
        let mut sim = create_test_simulation_with_foils();

        // Add some initial electrons
        sim.bodies[0].electrons.push(Electron {
            rel_pos: Vec2::zero(),
            vel: Vec2::zero(),
        });
        sim.bodies[1].electrons.push(Electron {
            rel_pos: Vec2::zero(),
            vel: Vec2::zero(),
        });
        sim.bodies[1].electrons.push(Electron {
            rel_pos: Vec2::zero(),
            vel: Vec2::zero(),
        });

        let initial_total_electrons: usize =
            sim.bodies.iter().map(|body| body.electrons.len()).sum();

        // Process foils multiple times
        for _ in 0..5 {
            let mut recipients = vec![false; sim.bodies.len()];
            sim.process_foils_with_charge_conservation(0.0, &mut recipients);

            // Check total electron count remains constant
            let current_total_electrons: usize =
                sim.bodies.iter().map(|body| body.electrons.len()).sum();

            assert_eq!(
                current_total_electrons, initial_total_electrons,
                "Total electron count should be conserved throughout simulation"
            );
        }
    }

    #[test]
    fn test_foils_at_capacity_limits() {
        let mut sim = Simulation::new();

        // Create foil bodies at max capacity
        let mut foil_body1 = Body::new(
            Vec2::new(-10.0, 0.0),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::FoilMetal,
        );
        let foil_body2 = Body::new(
            Vec2::new(10.0, 0.0),
            Vec2::zero(),
            1.0,
            1.1,
            0.0,
            Species::FoilMetal,
        );

        // Fill foil1 to max capacity
        for _ in 0..crate::config::FOIL_MAX_ELECTRONS {
            foil_body1.electrons.push(Electron {
                rel_pos: Vec2::zero(),
                vel: Vec2::zero(),
            });
        }

        sim.bodies.push(foil_body1);
        sim.bodies.push(foil_body2);

        // Create foils ready to operate
        let mut foil1 = Foil::new(vec![sim.bodies[0].id], Vec2::zero(), 1.0, 1.0, 2.0, 0.0);
        foil1.accum = 1.5; // Wants to add but can't (at capacity)

        let mut foil2 = Foil::new(vec![sim.bodies[1].id], Vec2::zero(), 1.0, 1.0, -2.0, 0.0);
        foil2.accum = -1.5; // Wants to remove but can't (empty)

        sim.foils.push(foil1);
        sim.foils.push(foil2);

        let initial_electrons_1 = sim.bodies[0].electrons.len();
        let initial_electrons_2 = sim.bodies[1].electrons.len();
        let initial_accum1 = sim.foils[0].accum;
        let initial_accum2 = sim.foils[1].accum;
        let dt = sim.dt;
        let current1 = 2.0; // foil1 dc_current
        let current2 = -2.0; // foil2 dc_current

        // Process foils - should do nothing due to capacity constraints
        let mut recipients = vec![false; sim.bodies.len()];
        sim.process_foils_with_charge_conservation(0.0, &mut recipients);

        // Verify no operations occurred due to capacity limits
        assert_eq!(
            sim.bodies[0].electrons.len(),
            initial_electrons_1,
            "Foil at max capacity should not gain electrons"
        );
        assert_eq!(
            sim.bodies[1].electrons.len(),
            initial_electrons_2,
            "Empty foil should not lose electrons"
        );

        // Accumulators should still be updated by current flow
        let expected_accum1 = initial_accum1 + current1 * dt;
        let expected_accum2 = initial_accum2 + current2 * dt;
        assert_eq!(
            sim.foils[0].accum, expected_accum1,
            "Accumulator should be updated by current even when operation fails"
        );
        assert_eq!(
            sim.foils[1].accum, expected_accum2,
            "Accumulator should be updated by current even when operation fails"
        );
    }
}
