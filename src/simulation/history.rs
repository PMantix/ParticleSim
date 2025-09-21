use std::time::Instant;

use crate::io::SimulationState;
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
        // Clear existing history and create initial snapshot
        self.compressed_history = super::compressed_history::CompressedHistorySystem::new_default();
        self.history_cursor = 0;
        self.history_dirty = false;
        self.playback.reset();
        
        // Create and add initial snapshot
        let light_snapshot = super::compressed_history::LightSnapshot::from(&*self);
        *SIM_TIME.lock() = light_snapshot.sim_time;
        self.compressed_history.push_frame(light_snapshot);
        
        self.publish_playback_status();
    }

    pub fn push_history_snapshot(&mut self) {
        self.truncate_future_history();
        
        // Create lightweight snapshot and add to compressed history
        let light_snapshot = super::compressed_history::LightSnapshot::from(&*self);
        *SIM_TIME.lock() = light_snapshot.sim_time;
        self.compressed_history.push_frame(light_snapshot);
        
        // Update cursor to latest frame
        if let Some((_, newest)) = self.compressed_history.get_frame_range() {
            self.history_cursor = newest;
        }
        
        self.history_dirty = false;
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

    pub fn apply_snapshot(&mut self, frame: usize) -> bool {
        if let Ok(light_snapshot) = self.compressed_history.reconstruct_frame(frame) {
            let simulation_state = SimulationState::from(&light_snapshot);
            let full_snapshot = SimulationSnapshot::from_state(simulation_state);
            full_snapshot.apply(self);
            true
        } else {
            false
        }
    }

    pub fn seek_history(&mut self, frame: usize) {
        if self.apply_snapshot(frame) {
            self.history_cursor = frame;
            self.history_dirty = false;
            self.playback.pause();
            self.publish_playback_status();
        }
    }

    pub fn truncate_future_history(&mut self) {
        // For now, we'll implement a simple version that doesn't truncate future
        // since the compressed history system manages its own cleanup
        // TODO: Implement proper future truncation in CompressedHistorySystem
    }

    pub fn resume_live_from_current(&mut self) {
        self.truncate_future_history();
        // Update cursor to latest available frame
        if let Some((_, newest)) = self.compressed_history.get_frame_range() {
            self.history_cursor = newest;
        }
        self.playback.pause();
        self.publish_playback_status();
    }

    pub fn go_to_latest(&mut self) {
        // Get the latest frame from compressed history
        if let Some((_, newest)) = self.compressed_history.get_frame_range() {
            if self.apply_snapshot(newest) {
                self.history_cursor = newest;
                self.history_dirty = false;
            }
        }
        self.playback.pause();
        self.publish_playback_status();
    }

    pub fn start_playback(&mut self, auto_resume: bool) {
        // Check if we have any history available
        if self.compressed_history.get_frame_range().is_none() {
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
        if frames == 0 || self.compressed_history.get_frame_range().is_none() {
            return PlaybackProgress::NoChange;
        }

        let (_oldest, newest) = self.compressed_history.get_frame_range().unwrap();
        let mut advanced = false;
        
        for _ in 0..frames {
            if self.history_cursor + 1 <= newest {
                self.history_cursor += 1;
                if let Ok(light_snapshot) = self.compressed_history.reconstruct_frame(self.history_cursor) {
                    let simulation_state = SimulationState::from(&light_snapshot);
                    let full_snapshot = SimulationSnapshot::from_state(simulation_state);
                    full_snapshot.apply(self);
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

    pub fn load_state(&mut self, state: SimulationState) {
        let snapshot = SimulationSnapshot::from_state(state);
        snapshot.apply(self);
        // Recreate compressed history with default config
        self.compressed_history = super::compressed_history::CompressedHistorySystem::new_default();
        self.initialize_history();
        self.history_cursor = 0;
        self.history_dirty = false;
        self.playback.reset();
        self.publish_playback_status();
    }

    pub fn publish_playback_status(&mut self) {
        let mut status = PLAYBACK_STATUS.lock();
        let (history_len, latest_index, cursor, sim_time, frame, dt) = if let Some((_oldest, newest)) = self.compressed_history.get_frame_range() {
            let history_len = 1; // For compatibility, we'll report 1 when we have frames
            let cursor_clamped = self.history_cursor.max(0).min(newest);
            if let Ok(light_snapshot) = self.compressed_history.reconstruct_frame(cursor_clamped) {
                (history_len, newest, cursor_clamped, light_snapshot.sim_time, light_snapshot.frame, light_snapshot.dt)
            } else {
                (history_len, newest, cursor_clamped, self.frame as f32 * self.dt, self.frame, self.dt)
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
