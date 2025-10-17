use std::time::Instant;

use crate::io::{SavedScenario, SimulationState};
use crate::renderer::state::{PlaybackModeStatus, PlaybackStatus, PLAYBACK_STATUS, SIM_TIME};

use super::simulation::Simulation;

const BASE_PLAYBACK_FPS: f32 = 60.0;

#[derive(Clone)]
pub struct SimulationSnapshot {
    pub state: SimulationState,
    pub frame: usize,
    pub sim_time: f32,
    pub dt: f32,
    pub last_thermostat_time: f32,
}

impl SimulationSnapshot {
    pub fn from_state(state: SimulationState) -> Self {
        Self {
            frame: state.frame,
            sim_time: state.sim_time,
            dt: state.dt,
            last_thermostat_time: state.last_thermostat_time,
            state,
        }
    }

    pub fn apply(&self, simulation: &mut Simulation) {
        self.state.clone().apply_to(simulation);
        simulation.frame = self.frame;
        simulation.dt = self.dt;
        simulation.last_thermostat_time = self.last_thermostat_time;
        *SIM_TIME.lock() = self.sim_time;
    }
}

#[derive(Clone)]
pub struct PlaybackController {
    is_playing: bool,
    auto_resume: bool,
    speed: f32,
    last_instant: Option<Instant>,
    accumulator: f32,
}

impl PlaybackController {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            auto_resume: false,
            speed: 1.0,
            last_instant: None,
            accumulator: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.is_playing = false;
        self.auto_resume = false;
        self.last_instant = None;
        self.accumulator = 0.0;
    }

    pub fn start(&mut self, auto_resume: bool) {
        self.is_playing = true;
        self.auto_resume = auto_resume;
        self.last_instant = Some(Instant::now());
        self.accumulator = 0.0;
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
        self.auto_resume = false;
        self.last_instant = None;
        self.accumulator = 0.0;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.0);
    }

    pub fn frames_to_advance(&mut self, now: Instant) -> usize {
        if !self.is_playing {
            self.last_instant = Some(now);
            return 0;
        }
        let last = self.last_instant.unwrap_or(now);
        self.last_instant = Some(now);
        let elapsed = now.saturating_duration_since(last);
        self.accumulator += elapsed.as_secs_f32() * self.speed * BASE_PLAYBACK_FPS;
        let frames = self.accumulator.floor() as usize;
        self.accumulator -= frames as f32;
        frames
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn auto_resume(&self) -> bool {
        self.auto_resume
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }
}

impl Default for PlaybackController {
    fn default() -> Self {
        Self::new()
    }
}

pub enum PlaybackProgress {
    NoChange,
    Advanced,
    ReachedLive { should_resume_live: bool },
}

impl Simulation {
    pub fn initialize_history(&mut self) {
        // Initialize simple history system
        self.simple_history.clear();
        self.history_cursor = 0;
        self.history_dirty = false;
        self.playback.reset();

        // Add initial snapshot to simple history
        self.push_simple_history_snapshot();

        self.publish_playback_status();
    }

    pub fn push_history_snapshot(&mut self) {
        // Use the simple history implementation
        self.push_simple_history_snapshot();
        self.publish_playback_status();
    }

    pub fn mark_history_dirty(&mut self) {
        self.history_dirty = true;
    }

    pub fn flush_history_if_dirty(&mut self) {
        if self.history_dirty {
            self.push_history_snapshot();
        }
    }

    pub fn apply_snapshot(&mut self, index: usize) -> bool {
        if let Some(state) = self.simple_history.get(index) {
            // Apply the simulation state directly
            let snapshot = SimulationSnapshot::from_state(state.clone());
            snapshot.apply(self);
            true
        } else {
            false
        }
    }

    pub fn seek_history(&mut self, index: usize) {
        if self.apply_snapshot(index) {
            self.history_cursor = index;
            self.history_dirty = false;
            self.playback.pause();
            self.publish_playback_status();
        }
    }

    pub fn truncate_future_history(&mut self) {
        // Truncate history from cursor forward
        let new_len = self.history_cursor + 1;
        self.simple_history.truncate(new_len);
    }

    pub fn resume_live_from_current(&mut self) {
        self.truncate_future_history();
        // Update cursor to latest available frame
        self.history_cursor = self.simple_history.len().saturating_sub(1);
        self.playback.pause();
        self.publish_playback_status();
    }

    pub fn go_to_latest(&mut self) {
        // Go to the latest frame in simple history
        let latest_index = self.simple_history.len().saturating_sub(1);
        if self.apply_snapshot(latest_index) {
            self.history_cursor = latest_index;
            self.history_dirty = false;
        }
        self.playback.pause();
        self.publish_playback_status();
    }

    pub fn start_playback(&mut self, auto_resume: bool) {
        // Check if we have any history available
        if self.simple_history.is_empty() {
            return;
        }
        self.playback.start(auto_resume);
        self.publish_playback_status();
    }

    pub fn pause_playback(&mut self) {
        self.playback.pause();
        self.publish_playback_status();
    }

    pub fn set_playback_speed(&mut self, speed: f32) {
        self.playback.set_speed(speed);
        self.publish_playback_status();
    }

    pub fn advance_playback(&mut self, now: Instant) -> PlaybackProgress {
        let frames = self.playback.frames_to_advance(now);
        if frames == 0 || self.simple_history.is_empty() {
            return PlaybackProgress::NoChange;
        }

        let newest = self.simple_history.len().saturating_sub(1);
        let mut advanced = false;

        for _ in 0..frames {
            if self.history_cursor + 1 <= newest {
                self.history_cursor += 1;
                if self.apply_snapshot(self.history_cursor) {
                    self.history_dirty = false;
                    advanced = true;
                }
            } else {
                let should_resume = self.playback.auto_resume();
                self.playback.pause();
                self.publish_playback_status();
                return PlaybackProgress::ReachedLive {
                    should_resume_live: should_resume,
                };
            }
        }

        if self.history_cursor + 1 > newest {
            let should_resume = self.playback.auto_resume();
            self.playback.pause();
            self.publish_playback_status();
            return PlaybackProgress::ReachedLive {
                should_resume_live: should_resume,
            };
        }

        if advanced {
            self.publish_playback_status();
            PlaybackProgress::Advanced
        } else {
            let should_resume = self.playback.auto_resume();
            self.playback.pause();
            self.publish_playback_status();
            PlaybackProgress::ReachedLive {
                should_resume_live: should_resume,
            }
        }
    }

    pub fn load_state(&mut self, saved: SavedScenario) {
        let SavedScenario {
            current,
            history,
            history_cursor,
            history_capacity,
            ui,
        } = saved;

        // Apply persisted UI selections to global statics so the GUI restores controls
        {
            use crate::renderer::state as rstate;
            *rstate::PERSIST_UI_CHARGING_MODE.lock() = Some(ui.charging_mode.clone());
            *rstate::PERSIST_UI_CONV_IS_OVER.lock() = Some(ui.conventional_is_overpotential);
            *rstate::PERSIST_UI_CONV_CURRENT.lock() = Some(ui.conventional_current_setpoint);
            *rstate::PERSIST_UI_CONV_TARGET.lock() = Some(ui.conventional_target_ratio);
        }

        let snapshot = SimulationSnapshot::from_state(current);
        snapshot.apply(self);

        // Reset derived systems
        self.compressed_history = super::compressed_history::CompressedHistorySystem::new_default();

        self.history_capacity = history_capacity.max(1);

        if history.is_empty() {
            self.initialize_history();
            return;
        }

        self.simple_history = history.into();

        if self.simple_history.len() > self.history_capacity {
            let drop_count = self.simple_history.len() - self.history_capacity;
            for _ in 0..drop_count {
                self.simple_history.pop_front();
            }
        }

        if self.simple_history.is_empty() {
            self.initialize_history();
            return;
        }

        let latest_index = self.simple_history.len().saturating_sub(1);
        let cursor = history_cursor.min(latest_index);
        self.history_cursor = cursor;
        if !self.apply_snapshot(cursor) {
            let fallback_cursor = self.simple_history.len().saturating_sub(1);
            if self.apply_snapshot(fallback_cursor) {
                self.history_cursor = fallback_cursor;
            }
        }
        self.history_dirty = false;
        self.playback.reset();
        self.publish_playback_status();
    }

    pub fn is_viewing_history(&self) -> bool {
        if self.simple_history.is_empty() {
            return false;
        }
        let latest_index = self.simple_history.len().saturating_sub(1);
        self.history_cursor < latest_index
    }

    pub fn publish_playback_status(&mut self) {
        let mut status = PLAYBACK_STATUS.lock();
        let (history_len, latest_index, cursor, sim_time, frame, dt) =
            if !self.simple_history.is_empty() {
                let history_len = self.simple_history.len();
                let latest_index = history_len.saturating_sub(1);
                let cursor_clamped = self.history_cursor.min(latest_index);

                if let Some(state) = self.simple_history.get(cursor_clamped) {
                    (
                        history_len,
                        latest_index,
                        cursor_clamped,
                        state.sim_time,
                        state.frame,
                        state.dt,
                    )
                } else {
                    (
                        history_len,
                        latest_index,
                        cursor_clamped,
                        self.frame as f32 * self.dt,
                        self.frame,
                        self.dt,
                    )
                }
            } else {
                (0, 0, 0, self.frame as f32 * self.dt, self.frame, self.dt)
            };

    let mode = if cursor >= latest_index {
            if self.playback.is_playing() {
                PlaybackModeStatus::HistoryPlaying
            } else {
                PlaybackModeStatus::Live
            }
        } else if self.playback.is_playing() {
            PlaybackModeStatus::HistoryPlaying
        } else {
            PlaybackModeStatus::HistoryPaused
        };

        // Present 1-based indices for UI friendliness; keep internal 0-based
        *status = PlaybackStatus {
            history_len,
            latest_index,
            cursor,
            is_playing: self.playback.is_playing(),
            mode,
            speed: self.playback.speed(),
            sim_time,
            frame,
            dt,
        };
    }
}
