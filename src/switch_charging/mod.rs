use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Mutex;

use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use once_cell::sync::Lazy;
use quarkstrom::egui;
use serde::{Deserialize, Serialize};

use crate::body::foil::{ChargingMode, Foil, OverpotentialController};

pub type FoilId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "+A")]
    PosA,
    #[serde(rename = "+B")]
    PosB,
    #[serde(rename = "-A")]
    NegA,
    #[serde(rename = "-B")]
    NegB,
}

impl Role {
    pub const ALL: [Role; 4] = [Role::PosA, Role::PosB, Role::NegA, Role::NegB];

    pub fn display(&self) -> &'static str {
        match self {
            Role::PosA => "+A",
            Role::PosB => "+B",
            Role::NegA => "-A",
            Role::NegB => "-B",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Role::PosA => "Acting cathode A",
            Role::PosB => "Acting cathode B",
            Role::NegA => "Acting anode A",
            Role::NegB => "Acting anode B",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Current,
    Overpotential,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Current
    }
}

impl Mode {
    
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StepSetpoint {
    pub mode: Mode,
    pub value: f64,
}

impl Default for StepSetpoint {
    fn default() -> Self {
        Self {
            mode: Mode::Current,
            value: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StepActiveInactiveSetpoints {
    pub active: StepSetpoint,
    pub inactive: StepSetpoint,
}

impl Default for StepActiveInactiveSetpoints {
    fn default() -> Self {
        Self {
            active: StepSetpoint { mode: Mode::Overpotential, value: 0.9 },
            inactive: StepSetpoint { mode: Mode::Overpotential, value: 1.0 },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SwitchChargingConfig {
    pub role_to_foil: HashMap<Role, Vec<FoilId>>,
    pub sim_dt_s: f64,
    pub switch_rate_hz: f64,
    pub delta_steps: u32,
    pub step_setpoints: HashMap<u8, StepSetpoint>,
    /// If true, use per-step Active/Inactive setpoints; otherwise use legacy per-step setpoints
    pub use_active_inactive_setpoints: bool,
    /// Per-step setpoints for active vs inactive foils
    pub step_active_inactive: HashMap<u8, StepActiveInactiveSetpoints>,
}

impl Default for SwitchChargingConfig {
    fn default() -> Self {
        let mut cfg = Self {
            role_to_foil: HashMap::new(),
            sim_dt_s: default_sim_dt_s(),
            switch_rate_hz: 1.0,
            delta_steps: 10000,
            step_setpoints: HashMap::from([
                (0, StepSetpoint { mode: Mode::Overpotential, value: 0.9 }),
                (1, StepSetpoint { mode: Mode::Overpotential, value: 0.9 }),
                (2, StepSetpoint { mode: Mode::Overpotential, value: 0.9 }),
                (3, StepSetpoint { mode: Mode::Overpotential, value: 0.9 }),
            ]),
            use_active_inactive_setpoints: true,
            step_active_inactive: HashMap::new(),
        };
        cfg.ensure_all_steps();
        cfg.ensure_all_step_active_inactive();
        cfg.recompute_from_steps();
        cfg
    }
}

impl SwitchChargingConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.sim_dt_s.is_finite() || self.sim_dt_s <= 0.0 {
            return Err("Simulation timestep must be a positive finite value".into());
        }
        if !self.switch_rate_hz.is_finite() || self.switch_rate_hz <= 0.0 {
            return Err("Switching frequency must be positive".into());
        }
        if self.delta_steps == 0 {
            return Err("Switch dwell Δt must be at least one simulation step".into());
        }

        for role in Role::ALL.iter() {
            if !self.role_to_foil.contains_key(role) || self.role_to_foil[role].is_empty() {
                return Err(format!("Missing foil assignment for role {}", role));
            }
        }

        let mut seen = HashSet::new();
        for (role, foil_ids) in &self.role_to_foil {
            for &foil_id in foil_ids {
                if !seen.insert(foil_id) {
                    return Err(format!(
                        "Foil {foil_id} has been assigned to multiple roles (including {role})"
                    ));
                }
            }
        }

        for step in 0u8..4u8 {
            let Some(setpoint) = self.step_setpoints.get(&step) else {
                return Err(format!("Missing setpoint for step {}", step + 1));
            };
            if !setpoint.value.is_finite() {
                return Err(format!("Step {} setpoint must be finite", step + 1));
            }
        }

        // Validate per-step active/inactive setpoints if enabled
        for step in 0u8..4u8 {
            if !self.step_active_inactive.contains_key(&step) {
                return Err(format!("Missing active/inactive setpoints for step {}", step + 1));
            }
            let sai = &self.step_active_inactive[&step];
            if !sai.active.value.is_finite() || !sai.inactive.value.is_finite() {
                return Err(format!("Step {} active/inactive setpoints must be finite", step + 1));
            }
        }

        Ok(())
    }

    pub fn recompute_from_steps(&mut self) {
        if self.delta_steps > 0 && self.sim_dt_s > 0.0 {
            self.switch_rate_hz = 1.0 / (self.delta_steps as f64 * self.sim_dt_s);
        }
    }

    pub fn cycle_period_s(&self) -> f64 {
        4.0 * self.delta_steps as f64 * self.sim_dt_s
    }

    pub fn foils_for_role(&self, role: Role) -> &[FoilId] {
        self.role_to_foil.get(&role).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn ensure_all_steps(&mut self) {
        for step in 0..4u8 {
            self.step_setpoints
                .entry(step)
                .or_insert_with(StepSetpoint::default);
        }
    }

    pub fn ensure_all_step_active_inactive(&mut self) {
        for step in 0..4u8 {
            self.step_active_inactive
                .entry(step)
                .or_insert_with(StepActiveInactiveSetpoints::default);
        }
    }
}

pub fn roles_for_step(step_index: u8) -> (Role, Role) {
    match step_index % 4 {
        0 => (Role::PosA, Role::NegA),
        1 => (Role::PosB, Role::NegB),
        2 => (Role::PosB, Role::NegA),
        _ => (Role::PosA, Role::NegB),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunState {
    Idle,
    Running,
    Paused,
}

impl Default for RunState {
    fn default() -> Self {
        RunState::Idle
    }
}

#[derive(Clone, Debug)]
pub enum SwitchControl {
    Start,
    Pause,
    Stop,
    UpdateConfig(SwitchChargingConfig),
}

#[derive(Clone, Debug)]
pub enum SwitchStatus {
    RunState(RunState),
    ActiveStep {
        step_index: u8,
        dwell_remaining: u32,
    },
    ConfigApplied(SwitchChargingConfig),
    ValidationFailed(String),
}

pub type ControlSender = Sender<SwitchControl>;
pub type ControlReceiver = Receiver<SwitchControl>;
pub type StatusSender = Sender<SwitchStatus>;
pub type StatusReceiver = Receiver<SwitchStatus>;

#[derive(Debug)]
pub struct UiHandles {
    pub control_tx: ControlSender,
    pub status_rx: StatusReceiver,
}

#[derive(Debug)]
pub struct SimHandles {
    pub control_rx: ControlReceiver,
    pub status_tx: StatusSender,
}

pub fn create_channels() -> (UiHandles, SimHandles) {
    let (control_tx, control_rx) = channel::unbounded();
    let (status_tx, status_rx) = channel::unbounded();
    (
        UiHandles {
            control_tx: control_tx.clone(),
            status_rx,
        },
        SimHandles {
            control_rx,
            status_tx,
        },
    )
}

static UI_HANDLES: Lazy<Mutex<Option<UiHandles>>> = Lazy::new(|| Mutex::new(None));

pub fn install_ui_handles(handles: UiHandles) {
    *UI_HANDLES.lock().unwrap() = Some(handles);
}

pub fn take_ui_handles() -> Option<UiHandles> {
    UI_HANDLES.lock().unwrap().take()
}

#[derive(Clone, Debug)]
pub struct SwitchScheduler {
    pub current_step: u8,
    pub steps_left: u32,
    pub armed: bool,
}

impl Default for SwitchScheduler {
    fn default() -> Self {
        Self {
            current_step: 0,
            steps_left: 0,
            armed: false,
        }
    }
}

impl SwitchScheduler {
    pub fn start(&mut self, cfg: &SwitchChargingConfig) {
        self.armed = true;
        self.current_step = 0;
        self.steps_left = cfg.delta_steps.max(1);
    }

    pub fn pause(&mut self) {
        self.armed = false;
    }

    pub fn stop(&mut self) {
        self.armed = false;
        self.current_step = 0;
        self.steps_left = 0;
    }

    pub fn sync_with_config(&mut self, cfg: &SwitchChargingConfig) {
        if cfg.delta_steps == 0 {
            self.steps_left = 0;
            self.current_step = 0;
            self.armed = false;
            return;
        }
        if self.steps_left == 0 && self.armed {
            self.steps_left = cfg.delta_steps;
        } else if self.steps_left > cfg.delta_steps {
            self.steps_left = cfg.delta_steps;
        }
    }

    pub fn on_tick(
        &mut self,
        cfg: &SwitchChargingConfig,
    ) -> Option<((Vec<FoilId>, Vec<FoilId>), StepSetpoint)> {
        if !self.armed || cfg.delta_steps == 0 {
            return None;
        }
        if self.steps_left == 0 {
            self.current_step = (self.current_step + 1) % 4;
            self.steps_left = cfg.delta_steps;
        }

        self.steps_left = self.steps_left.saturating_sub(1);
        let step = self.current_step;
        let (positive, negative) = roles_for_step(step);
        let pos_ids = cfg.foils_for_role(positive).to_vec();
        let neg_ids = cfg.foils_for_role(negative).to_vec();
        
        if pos_ids.is_empty() || neg_ids.is_empty() {
            return None;
        }
        
        let setpoint = cfg.step_setpoints.get(&step)?.clone();

        Some(((pos_ids, neg_ids), setpoint))
    }

    pub fn current_step(&self) -> u8 {
        self.current_step
    }

    pub fn dwell_remaining(&self) -> u32 {
        self.steps_left
    }
}

#[derive(Clone, Debug)]
pub struct FoilStateSnapshot {
    charging_mode: ChargingMode,
    dc_current: f32,
    ac_current: f32,
    switch_hz: f32,
    overpotential_controller: Option<OverpotentialController>,
    slave_overpotential_current: f32,
}

impl FoilStateSnapshot {
    pub fn from_foil(foil: &Foil) -> Self {
        Self {
            charging_mode: foil.charging_mode,
            dc_current: foil.dc_current,
            ac_current: foil.ac_current,
            switch_hz: foil.switch_hz,
            overpotential_controller: foil.overpotential_controller.clone(),
            slave_overpotential_current: foil.slave_overpotential_current,
        }
    }

    pub fn apply(&self, foil: &mut Foil) {
        foil.charging_mode = self.charging_mode;
        foil.dc_current = self.dc_current;
        foil.ac_current = self.ac_current;
        foil.switch_hz = self.switch_hz;
        foil.overpotential_controller = self.overpotential_controller.clone();
        foil.slave_overpotential_current = self.slave_overpotential_current;
    }
}

#[derive(Clone, Debug)]
struct FoilDescriptor {
    id: FoilId,
    label: String,
}

#[derive(Debug)]
pub struct SwitchUiState {
    pub config: SwitchChargingConfig,
    pub validation_error: Option<String>,
    pub run_state: RunState,
    pub pending_role: Option<Role>,
    pub last_step_status: Option<(u8, u32)>,
    available_foils: Vec<FoilDescriptor>,
    control_tx: Option<ControlSender>,
    status_rx: Option<StatusReceiver>,
    last_selected_foil: Option<FoilId>,
    pub json_buffer: String,
    pub status_message: Option<String>,
    pub import_error: Option<String>,
    config_dirty: bool,
}

impl Default for SwitchUiState {
    fn default() -> Self {
        Self::new()
    }
}

impl SwitchUiState {
    pub fn new() -> Self {
        let handles = take_ui_handles();
        let (control_tx, status_rx) = if let Some(handles) = handles {
            (Some(handles.control_tx), Some(handles.status_rx))
        } else {
            let (ui_handles, _) = create_channels();
            (Some(ui_handles.control_tx), Some(ui_handles.status_rx))
        };

        let mut state = Self {
            config: SwitchChargingConfig::default(),
            validation_error: None,
            run_state: RunState::Idle,
            pending_role: None,
            last_step_status: None,
            available_foils: Vec::new(),
            control_tx,
            status_rx,
            last_selected_foil: None,
            json_buffer: String::new(),
            status_message: None,
            import_error: None,
            config_dirty: false,
        };
        state.update_validation();
        state
    }

    pub fn sync_sim_dt(&mut self, dt_fs: f32) {
        let dt_s = (dt_fs as f64) * 1e-15;
        if dt_s.is_finite() && dt_s > 0.0 && (dt_s - self.config.sim_dt_s).abs() > f64::EPSILON {
            self.config.sim_dt_s = dt_s;
            // Don't automatically recompute user-set values during simulation
            // self.config.recompute_from_steps();
            self.update_validation();
        }
    }

    pub fn update_available_foils(&mut self, foils: &[Foil]) {
        self.available_foils = foils
            .iter()
            .map(|foil| FoilDescriptor {
                id: foil.id,
                label: format!("Foil {} ({} particles)", foil.id, foil.body_ids.len()),
            })
            .collect();
        self.available_foils.sort_by_key(|entry| entry.id);
    }

    pub fn consume_selected_foil(&mut self, selected: Option<FoilId>) {
        if let Some(role) = self.pending_role {
            if let Some(foil_id) = selected {
                if Some(foil_id) != self.last_selected_foil {
                    // Check if this foil is already assigned to this role
                    let already_assigned = self.config.foils_for_role(role).contains(&foil_id);
                    
                    if !already_assigned {
                        self.add_foil_to_role(role, foil_id);
                        self.pending_role = None;
                        let role_foils = self.config.foils_for_role(role);
                        if role_foils.len() == 1 {
                            self.status_message = Some(format!("Assigned foil {foil_id} to role {role}"));
                        } else {
                            self.status_message = Some(format!("Added foil {foil_id} to role {role} ({} foils total)", role_foils.len()));
                        }
                    } else {
                        self.status_message = Some(format!("Foil {foil_id} is already assigned to role {role}"));
                        self.pending_role = None;
                    }
                }
            }
        }
        self.last_selected_foil = selected;
    }

    pub fn poll_status(&mut self) {
        loop {
            let Some(rx) = &self.status_rx else {
                return;
            };
            match rx.try_recv() {
                Ok(SwitchStatus::RunState(state)) => {
                    self.run_state = state;
                    if matches!(state, RunState::Idle) {
                        self.pending_role = None;
                    }
                }
                Ok(SwitchStatus::ActiveStep {
                    step_index,
                    dwell_remaining,
                }) => {
                    self.last_step_status = Some((step_index, dwell_remaining));
                }
                Ok(SwitchStatus::ConfigApplied(cfg)) => {
                    // Only update config if we don't have pending local changes
                    if !self.config_dirty {
                        self.config = cfg;
                        self.config.ensure_all_steps();
                        self.config.ensure_all_step_active_inactive();
                        self.update_validation();
                    }
                    self.config_dirty = false;
                    self.status_message = Some("Configuration applied to simulation".into());
                }
                Ok(SwitchStatus::ValidationFailed(message)) => {
                    self.validation_error = Some(message);
                    self.config_dirty = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.status_rx = None;
                    break;
                }
            }
        }
    }

    pub fn start(&mut self) {
        if self.validation_error.is_some() {
            return;
        }
        if self.config_dirty {
            self.send_update();
        }
        self.send_control(SwitchControl::Start);
        self.run_state = RunState::Running;
        self.status_message = Some("Switch charging running".into());
    }

    pub fn pause(&mut self) {
        self.send_control(SwitchControl::Pause);
        self.run_state = RunState::Paused;
        self.status_message = Some("Switch charging paused".into());
    }

    pub fn stop(&mut self) {
        self.send_control(SwitchControl::Stop);
        self.run_state = RunState::Idle;
        self.last_step_status = None;
        self.status_message = Some("Switch charging stopped".into());
    }

    pub fn send_update(&mut self) {
        let mut cfg = self.config.clone();
        cfg.ensure_all_steps();
        cfg.ensure_all_step_active_inactive();
        self.send_control(SwitchControl::UpdateConfig(cfg));
        self.config_dirty = false;
    }

    /// Add a foil to an existing role assignment (supports multiple foils per role)
    pub fn add_foil_to_role(&mut self, role: Role, foil_id: FoilId) {
        self.ensure_paused_for_edit();
        let foils = self.config.role_to_foil.entry(role).or_insert_with(Vec::new);
        if !foils.contains(&foil_id) {
            foils.push(foil_id);
            self.config_dirty = true;
            self.update_validation();
            self.send_update();
        }
    }

    /// Remove a specific foil from a role assignment
    pub fn remove_foil_from_role(&mut self, role: Role, foil_id: FoilId) {
        self.ensure_paused_for_edit();
        if let Some(foils) = self.config.role_to_foil.get_mut(&role) {
            foils.retain(|&id| id != foil_id);
            if foils.is_empty() {
                self.config.role_to_foil.remove(&role);
            }
            self.config_dirty = true;
            self.update_validation();
            self.send_update();
        }
    }

    fn ensure_paused_for_edit(&mut self) {
        if self.run_state == RunState::Running {
            self.pause();
        }
    }

    pub fn remove_role(&mut self, role: Role) {
        self.ensure_paused_for_edit();
        self.config.role_to_foil.remove(&role);
        self.config_dirty = true;
        self.update_validation();
        self.send_update(); // Send update immediately
    }

    pub fn edit_step(&mut self, step: u8, new_setpoint: StepSetpoint) {
        let changed = self
            .config
            .step_setpoints
            .get(&step)
            .map(|current| current != &new_setpoint)
            .unwrap_or(true);
        if changed {
            self.config.step_setpoints.insert(step, new_setpoint);
            self.config_dirty = true;
            self.update_validation();
            self.send_update(); // Send update immediately
        }
    }

    pub fn set_delta_steps(&mut self, steps: u32) {
        let steps = steps.max(1);
        if self.config.delta_steps != steps {
            self.config.delta_steps = steps;
            // Calculate frequency from steps according to the current dt
            self.config.recompute_from_steps();
            self.config_dirty = true;
            self.update_validation();
            self.send_update(); // Send update immediately
        }
    }

    fn send_control(&self, msg: SwitchControl) {
        if let Some(tx) = &self.control_tx {
            let _ = tx.send(msg);
        }
    }

    fn update_validation(&mut self) {
        self.validation_error = self.config.validate().err();
    }
}

pub fn ui_switch_charging(ui: &mut egui::Ui, state: &mut SwitchUiState) {
    state.poll_status();

    ui.heading("🔀 Switch Charging");

    ui.horizontal(|ui| {
        ui.label("Runtime state:");
        let state_label = match state.run_state {
            RunState::Idle => "Idle",
            RunState::Running => "Running",
            RunState::Paused => "Paused",
        };
        ui.strong(state_label);
        if let Some((step, dwell)) = state.last_step_status {
            let (pos, neg) = roles_for_step(step);
            ui.separator();
            ui.label(format!(
                "Step {} ({} → {}) | dwell remaining: {}",
                step + 1,
                pos.display(),
                neg.display(),
                dwell
            ));
        }
    });

    if let Some(msg) = &state.status_message {
        ui.colored_label(egui::Color32::LIGHT_BLUE, msg);
    }

    if let Some(err) = &state.validation_error {
        ui.colored_label(egui::Color32::LIGHT_RED, format!("⚠️ {err}"));
    } else {
        ui.colored_label(egui::Color32::LIGHT_GREEN, "Configuration valid");
    }

    ui.separator();

    ui.group(|ui| {
        ui.label("Electrode Assignments");
        egui::Grid::new("switch-roles-grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                for role in Role::ALL.iter() {
                    ui.label(format!("{}", role.description()));
                    
                    // Get assigned foils (create owned copy to avoid borrow issues)
                    let assigned_foils: Vec<FoilId> = state.config.foils_for_role(*role).to_vec();
                    
                    if !assigned_foils.is_empty() {
                        ui.vertical(|ui| {
                            for foil_id in &assigned_foils {
                                ui.horizontal(|ui| {
                                    // Show foil name/label
                                    let foil_name = state.available_foils
                                        .iter()
                                        .find(|f| f.id == *foil_id)
                                        .map(|f| f.label.clone())
                                        .unwrap_or_else(|| format!("Foil {}", foil_id));
                                    
                                    ui.label(format!("• {}", foil_name));
                                    
                                    // Remove button for this specific foil
                                    if ui.small_button("❌").on_hover_text("Remove this foil from role").clicked() {
                                        state.remove_foil_from_role(*role, *foil_id);
                                    }
                                });
                            }
                        });
                    } else {
                        ui.label("(no foils assigned)");
                    }
                    
                    // Add and Clear buttons
                    ui.horizontal(|ui| {
                        if ui.button(format!("+ Add Foil to {}", role.display())).clicked() {
                            state.pending_role = Some(*role);
                            state.status_message =
                                Some(format!("Select a foil in the viewport to add to role {}", role));
                        }
                        
                        if !assigned_foils.is_empty() {
                            if ui.button("Clear All").clicked() {
                                state.remove_role(*role);
                            }
                        }
                    });
                    ui.end_row();
                }
            });

        if let Some(role) = state.pending_role {
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("Click a foil to assign role {}", role.display()),
            );
        }

        ui.horizontal_wrapped(|ui| {
            ui.label("Available foils:");
            if state.available_foils.is_empty() {
                ui.label("None detected");
            } else {
                for entry in &state.available_foils {
                    ui.label(format!("{}", entry.label));
                }
            }
        });
    });

    ui.separator();

    ui.group(|ui| {
        ui.label("Switching Rate");
        
        // Steps per half-cycle is the primary control; frequency is derived
        ui.horizontal(|ui| {
            ui.label("Steps per half-cycle:");
            let mut steps = state.config.delta_steps;
            ui.add(egui::DragValue::new(&mut steps).speed(1.0));
            if steps != state.config.delta_steps {
                state.set_delta_steps(steps);
            }
        });

        // Frequency is now display-only, derived from steps and dt
        ui.horizontal(|ui| {
            ui.label("Frequency:");
            ui.label(format!("{:.6}", state.config.switch_rate_hz));
            ui.weak("Hz (calculated)");
        });

        ui.label(format!(
            "Cycle period: {:.6} s ({} total steps)",
            state.config.cycle_period_s(),
            state.config.delta_steps * 4
        ));
    });

    ui.separator();

    ui.group(|ui| {
        ui.label("Step Active/Inactive Setpoints");

        // Toggle to use active/inactive setpoints
        let mut use_active_inactive = state.config.use_active_inactive_setpoints;
        if ui.checkbox(&mut use_active_inactive, "Use Active/Inactive per step").changed() {
            state.config.use_active_inactive_setpoints = use_active_inactive;
            state.config_dirty = true;
            state.send_update();
        }

        ui.label("For each step, define what active foils and inactive foils should do:");

        // Show editor grid for per-step active/inactive setpoints
        egui::Grid::new("step-active-inactive-grid")
            .num_columns(6)
            .spacing([8.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.heading("Step");
                ui.heading("Active Mode");
                ui.heading("Active Value");
                ui.heading("Inactive Mode");
                ui.heading("Inactive Value");
                ui.heading("Info");
                ui.end_row();

                for step in 0..4u8 {
                    let (pos, neg) = roles_for_step(step);
                    let mut sai = state.config.step_active_inactive.get(&step).cloned().unwrap_or_default();
                    let mut changed = false;

                    // Check if this is the currently active step
                    let is_active_step = state.last_step_status.map(|(active_step, _)| active_step == step).unwrap_or(false);
                    
                    let step_label = if is_active_step {
                        format!("▶ {}", step + 1)
                    } else {
                        format!("{}", step + 1)
                    };
                    
                    ui.colored_label(
                        if is_active_step { egui::Color32::WHITE } else { ui.style().visuals.text_color() },
                        step_label
                    );

                    // Active mode
                    let amode_before = sai.active.mode;
                    egui::ComboBox::from_id_source(format!("step-active-mode-{}", step))
                        .selected_text(match sai.active.mode { Mode::Current => "Current", Mode::Overpotential => "Overpotential" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut sai.active.mode, Mode::Current, "Current");
                            ui.selectable_value(&mut sai.active.mode, Mode::Overpotential, "Overpotential");
                        });
                    if sai.active.mode != amode_before { changed = true; }

                    // Active value
                    let mut av = sai.active.value;
                    let label_a = match sai.active.mode { Mode::Current => "Current (e/fs)", Mode::Overpotential => "Target ratio" };
                    ui.horizontal(|ui| {
                        ui.label(label_a);
                        if ui.add(egui::DragValue::new(&mut av).speed(0.01).clamp_range(-10_000.0..=10_000.0)).changed() {
                            sai.active.value = av; changed = true;
                        }
                    });

                    // Inactive mode
                    let imode_before = sai.inactive.mode;
                    egui::ComboBox::from_id_source(format!("step-inactive-mode-{}", step))
                        .selected_text(match sai.inactive.mode { Mode::Current => "Current", Mode::Overpotential => "Overpotential" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut sai.inactive.mode, Mode::Current, "Current");
                            ui.selectable_value(&mut sai.inactive.mode, Mode::Overpotential, "Overpotential");
                        });
                    if sai.inactive.mode != imode_before { changed = true; }

                    // Inactive value
                    let mut iv = sai.inactive.value;
                    let label_i = match sai.inactive.mode { Mode::Current => "Current (e/fs)", Mode::Overpotential => "Target ratio" };
                    ui.horizontal(|ui| {
                        ui.label(label_i);
                        if ui.add(egui::DragValue::new(&mut iv).speed(0.01).clamp_range(-10_000.0..=10_000.0)).changed() {
                            sai.inactive.value = iv; changed = true;
                        }
                    });

                    // Info about which foils are active
                    ui.label(format!("{} → {} (active)", pos.display(), neg.display()));

                    if changed {
                        state.config.step_active_inactive.insert(step, sai);
                        state.config_dirty = true;
                        state.send_update();
                    }
                    ui.end_row();
                }
            });
    });

    ui.separator();

    ui.group(|ui| {
        ui.label("Step Setpoints");
        egui::Grid::new("switch-steps-grid")
            .num_columns(3)
            .spacing([8.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.heading("Step");
                ui.heading("Mode");
                ui.heading("Setpoint");
                ui.end_row();

                for step in 0..4u8 {
                    let (pos, neg) = roles_for_step(step);
                    
                    // Check if this is the currently active step
                    let is_active_step = state.last_step_status.map(|(active_step, _)| active_step == step).unwrap_or(false);
                    
                    // Highlight the active step with a background color
                    let bg_color = if is_active_step {
                        Some(egui::Color32::from_rgb(50, 100, 50)) // Green background for active step
                    } else {
                        None
                    };
                    
                    if let Some(color) = bg_color {
                        ui.painter().rect_filled(
                            ui.available_rect_before_wrap(),
                            0.0,
                            color,
                        );
                    }
                    
                    let step_label = if is_active_step {
                        format!("▶ {}: {} → {}", step + 1, pos.display(), neg.display())
                    } else {
                        format!("{}: {} → {}", step + 1, pos.display(), neg.display())
                    };
                    
                    ui.colored_label(
                        if is_active_step { egui::Color32::WHITE } else { ui.style().visuals.text_color() },
                        step_label
                    );

                    let mut setpoint = state
                        .config
                        .step_setpoints
                        .get(&step)
                        .cloned()
                        .unwrap_or_default();

                    let mut changed = false;

                    let original_mode = setpoint.mode;

                    let mode_response = egui::ComboBox::from_id_source(format!("switch-step-mode-{step}"))
                        .selected_text(match setpoint.mode {
                            Mode::Current => "Current",
                            Mode::Overpotential => "Overpotential",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut setpoint.mode, Mode::Current, "Current");
                            ui.selectable_value(
                                &mut setpoint.mode,
                                Mode::Overpotential,
                                "Overpotential",
                            );
                        });
                    
                    if mode_response.response.changed() || setpoint.mode != original_mode {
                        changed = true;
                    }

                    let label = match setpoint.mode {
                        Mode::Current => "Current (e/fs)",
                        Mode::Overpotential => "Target ratio",
                    };
                    ui.horizontal(|ui| {
                        ui.label(label);
                        let mut value = setpoint.value;
                        ui.add(egui::DragValue::new(&mut value).speed(0.01).clamp_range(-10_000.0..=10_000.0));
                        if (value - setpoint.value).abs() > f64::EPSILON {
                            setpoint.value = value;
                            changed = true;
                        }
                    });

                    if changed {
                        state.edit_step(step, setpoint);
                    }
                    ui.end_row();
                }
            });
    });

    ui.separator();

    ui.horizontal(|ui| {
        let run_enabled = state.validation_error.is_none();
        if ui
            .add_enabled(
                run_enabled && state.run_state != RunState::Running,
                egui::Button::new("▶ Run"),
            )
            .clicked()
        {
            state.start();
        }
        if ui
            .add_enabled(
                state.run_state == RunState::Running,
                egui::Button::new("⏸ Pause"),
            )
            .clicked()
        {
            state.pause();
        }
        if ui.button("⏹ Stop").clicked() {
            state.stop();
        }
        if ui.button("💾 Apply Config").clicked() {
            state.send_update();
        }
    });

    ui.separator();

    ui.group(|ui| {
        ui.label("Import / Export Configuration");
        ui.horizontal(|ui| {
            if ui.button("Export JSON").clicked() {
                match serde_json::to_string_pretty(&state.config) {
                    Ok(json) => {
                        state.json_buffer = json;
                        state.status_message = Some("Configuration exported".into());
                        state.import_error = None;
                    }
                    Err(err) => {
                        state.import_error = Some(format!("Failed to export: {err}"));
                    }
                }
            }
            if ui.button("Import JSON").clicked() {
                match serde_json::from_str::<SwitchChargingConfig>(&state.json_buffer) {
                    Ok(mut cfg) => {
                        cfg.ensure_all_steps();
                        state.config = cfg;
                        state.config_dirty = true;
                        state.update_validation();
                        state.send_update(); // Send update immediately after import
                        state.status_message = Some("Configuration imported".into());
                        state.import_error = None;
                    }
                    Err(err) => {
                        state.import_error = Some(format!("Import error: {err}"));
                    }
                }
            }
        });
        if let Some(err) = &state.import_error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
        }
        ui.add(
            egui::TextEdit::multiline(&mut state.json_buffer)
                .desired_rows(4)
                .lock_focus(true)
                .desired_width(f32::INFINITY),
        );
    });
}

pub fn default_sim_dt_s() -> f64 {
    (crate::config::DEFAULT_DT_FS as f64) * 1e-15
}

#[allow(dead_code)]
pub fn example_scheduler_loop(cfg: &SwitchChargingConfig, scheduler: &mut SwitchScheduler) {
    scheduler.start(cfg);
    let total_ticks = cfg.delta_steps.saturating_mul(4);
    for _ in 0..total_ticks {
        if let Some(((pos, neg), setpoint)) = scheduler.on_tick(cfg) {
            let _ = (pos, neg, setpoint);
            // In the real simulation this is where apply_foil_connection(pair, mode, value) would run.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_config() -> SwitchChargingConfig {
        let mut cfg = SwitchChargingConfig::default();
        cfg.role_to_foil.insert(Role::PosA, vec![1]);
        cfg.role_to_foil.insert(Role::NegA, vec![2]);
        cfg.role_to_foil.insert(Role::PosB, vec![3]);
        cfg.role_to_foil.insert(Role::NegB, vec![4]);
        cfg.delta_steps = 2;
        cfg.recompute_from_steps();
        cfg.ensure_all_steps();
        cfg
    }

    #[test]
    fn roles_follow_expected_sequence() {
        let expected = vec![
            (Role::PosA, Role::NegA),
            (Role::PosB, Role::NegB),
            (Role::PosB, Role::NegA),
            (Role::PosA, Role::NegB),
        ];
        for i in 0..8u8 {
            assert_eq!(roles_for_step(i), expected[(i % 4) as usize]);
        }
    }

    #[test]
    fn hz_to_steps_conversion_matches_dt() {
        let mut cfg = make_valid_config();
        cfg.sim_dt_s = 1e-4;
        // Given a target frequency, derive steps and verify
        let target_hz = 10.0;
        let expected_steps = (1.0 / (target_hz * cfg.sim_dt_s)).round().max(1.0) as u32;
        cfg.delta_steps = expected_steps;
        cfg.recompute_from_steps();
        assert!((cfg.switch_rate_hz - target_hz).abs() < 1e-6);
        cfg.delta_steps = 500;
        cfg.recompute_from_steps();
        assert!((cfg.switch_rate_hz - 20.0).abs() < 1e-6);

        cfg.sim_dt_s = 5e-5;
        let target_hz2 = 50.0;
        let expected_steps2 = (1.0 / (target_hz2 * cfg.sim_dt_s)).round().max(1.0) as u32;
        cfg.delta_steps = expected_steps2;
        cfg.recompute_from_steps();
        assert!((cfg.switch_rate_hz - target_hz2).abs() < 1e-6);
    }

    #[test]
    fn scheduler_cycles_in_order() {
        let cfg = make_valid_config();
        let mut scheduler = SwitchScheduler::default();
        scheduler.start(&cfg);

        let mut pairs = Vec::new();
        for _ in 0..8 {
            let ((a, b), _) = scheduler.on_tick(&cfg).expect("scheduler should emit");
            pairs.push((a, b));
        }
        let expected = vec![
            (vec![1], vec![2]),
            (vec![1], vec![2]),
            (vec![3], vec![4]),
            (vec![3], vec![4]),
            (vec![3], vec![2]),
            (vec![3], vec![2]),
            (vec![1], vec![4]),
            (vec![1], vec![4]),
        ];
        
        assert_eq!(pairs.len(), expected.len());
        for (i, (actual, expected)) in pairs.iter().zip(expected.iter()).enumerate() {
            assert_eq!(actual.0, expected.0, "Mismatch at step {} pos", i);
            assert_eq!(actual.1, expected.1, "Mismatch at step {} neg", i);
        }
    }

    #[test]
    fn validation_catches_errors() {
        let mut cfg = make_valid_config();
        assert!(cfg.validate().is_ok());

        cfg.role_to_foil.remove(&Role::NegB);
        assert!(cfg.validate().is_err());
        cfg.role_to_foil.insert(Role::NegB, vec![3]);
        assert!(cfg.validate().is_err());
        cfg.role_to_foil.insert(Role::NegB, vec![4]);

        cfg.switch_rate_hz = 0.0;
        assert!(cfg.validate().is_err());
        cfg.switch_rate_hz = 10.0;
        cfg.delta_steps = 0;
        assert!(cfg.validate().is_err());
    }
}
