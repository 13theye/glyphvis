// src/config/runtime.rs
//
// Module responsible for sending configs modifed at runtime to the rest of the app

use nannou_osc as osc;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum EventType {
    GlyphChange(String),
    TransitionTrigger,
    GridTransform,
    ParameterUpdate,
}
