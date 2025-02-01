// src/services/osc_receiver.rs
use nannou::prelude::*;
use nannou_osc as osc;

pub struct OscService {
    receiver: osc::Receiver,
}

impl OscService {
    pub fn new(port: u16) -> Self {
        let receiver = if let Ok(osc_receiver) = osc::receiver(port) {
            osc_receiver
        } else {
            panic!("Failed to bind to port {}", port);
        };
        Self { receiver }
    }
}
