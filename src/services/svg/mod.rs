// src/services/svg/mod.rs
pub mod parser;
pub mod edge_detection;

pub use parser::parse_svg;
pub use edge_detection::detect_edge_type;
