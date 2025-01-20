// src/views/grid_manager.rs

use nannou::prelude::*;
use std::collections::{ HashMap, HashSet };

use crate::models::{ ViewBox, EdgeType, PathElement, Project };
use crate::services::svg::{parse_svg, detect_edge_type};
use crate::services::grid::*;
use crate::views::Transform2D;

