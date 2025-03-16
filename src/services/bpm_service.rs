// src/services/bpm_service.rs

// incomplete

#[derive(Default, Debug)]
pub struct BpmService {}

impl BpmService {
    pub fn new() -> Self {
        Self {}
    }

    pub fn division_to_duration(division: u32, bpm: u32) -> f32 {
        let seconds_per_beat = 60.0 / bpm as f32;
        seconds_per_beat / (division as f32 / 4.0)
    }
}
