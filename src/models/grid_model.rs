/// src/models/grid_model.rs
/// data model and constructors for the assembled grid of SVG elements

use crate::models::data_model::Project;
use std::collections::HashMap;
use crate::services::path_service;
use crate::services::path_service::{ PathElement, GridElement, EdgeType };


#[derive(Debug, Clone)]
pub struct ViewBox {
    pub min_x: f32,
    pub min_y: f32,
    pub width: f32,
    pub height: f32,
}

impl ViewBox {
    pub fn max_x(&self) -> f32 { self.min_x + self.width }
    pub fn max_y(&self) -> f32 { self.min_y + self.height }
}

#[derive(Debug)]
pub struct Grid {
    pub elements: HashMap<String, GridElement>,
    pub width: u32,
    pub height: u32,
    pub viewbox: ViewBox,
}

impl Grid {
    pub fn new(project: &Project) -> Self {
        println!("\n=== Creating Grid ({}x{}) ===", project.grid_x, project.grid_y);
        let mut elements = HashMap::new();

        // parse viewbox
        let parse_viewbox = |svg_content: &str | -> Option<ViewBox>{
            let viewbox_data: Vec<String> = svg_content
                .lines()
                .filter(|line| line.contains("<svg id"))
                .filter_map(|line| {
                    if let Some(viewbox_start) = line.find("viewBox=\"") {
                        if let Some(viewbox_end) = line[viewbox_start + 9..].find('\"') {
                            return Some(line[viewbox_start + 9..viewbox_start + 9 + viewbox_end].to_string());
                        }
                    }
                    None
                })
                .collect();
        
            if viewbox_data.is_empty() {
                eprintln!("Error: No SVG element with viewBox attribute found");
                eprintln!("SVG content:\n{}", project.svg_base_tile);
                std::process::exit(1);
            }
        
            viewbox_data.get(0)
                .and_then(|data| {
                    let viewbox_values: Vec<f32> = data
                        .split(' ')
                        .filter_map(|value| value.parse::<f32>().ok())
                        .collect();
                    
                    if viewbox_values.len() != 4 {
                        eprintln!("Error: ViewBox must contain exactly 4 values");
                        eprintln!("Found viewBox=\"{}\" with {} values", data, viewbox_values.len());
                        eprintln!("Values parsed: {:?}", viewbox_values);
                        std::process::exit(1);
                    }
        
                    Some(ViewBox {
                        min_x: viewbox_values[0],
                        min_y: viewbox_values[1],
                        width: viewbox_values[2],
                        height: viewbox_values[3],
                    })
                })
        };
        
        let viewbox = match parse_viewbox(&project.svg_base_tile) {
            Some(viewbox) => viewbox,
            None => {
                eprintln!("Error: Failed to parse viewBox values");
                eprintln!("Please ensure the SVG contains a valid viewBox attribute");
                eprintln!("Format should be: viewBox=\"<min-x> <min-y> <width> <height>\"");
                std::process::exit(1);
            }
        };
        
        // Parse base SVG elements
        let base_elements: Vec<(String, PathElement)> = project.svg_base_tile
            .lines()
            .filter(|line| line.contains("<path") || line.contains("<circle"))
            .filter_map(|line| {
                if let Some(id_start) = line.find("id=\"") {
                    if let Some(id_end) = line[id_start + 4..].find('\"') {
                        let id = line[id_start + 4..id_start + 4 + id_end].to_string();
                        if let Some(element) = path_service::parse_svg_element(line) {
                            return Some((id, element));
                        }
                    }
                }
                None
            })
            .collect();

        println!("\n=== Base Elements ===");
        for (id, path) in &base_elements {
            println!("{}: {:?}", id, path);
        }

        // Generate grid elements with 1-based coordinates
        println!("\n=== Generating Grid Elements ===");
        for y in 1..=project.grid_y {
            for x in 1..=project.grid_x {
                for (base_id, base_path) in &base_elements {
                    let grid_id = format!("{},{} : {}", x, y, base_id);
                    let edge_type = path_service::detect_edge_type(base_path, &viewbox);
                    let element = GridElement {
                        id: base_id.clone(),
                        position: (x, y),
                        path: base_path.clone(),
                        edge_type,
                    };
                    
                    // Only print edge elements for brevity
                    if edge_type != EdgeType::None {
                        println!("Created {} at ({},{}) - {:?}", base_id, x, y, edge_type);
                    }
                    
                    elements.insert(grid_id, element);
                }
            }
        }

        // Pre-calculate and print which elements should be drawn
        println!("\n=== Drawing Decisions ===");
        let grid = Grid {
            elements,
            width: project.grid_x,
            height: project.grid_y,
            viewbox,
        };

        for y in 1..=project.grid_y {
            for x in 1..=project.grid_x {
                for element in grid.get_elements_at(x, y) {
                    if element.edge_type != EdgeType::None {
                        let should_draw = path_service::should_draw_element(
                            element,
                            grid.width,
                            grid.height,
                            &grid.elements
                        );
                        println!("{} at ({},{}) - Draw: {}", 
                                element.id, x, y, should_draw);
                    }
                }
            }
        }

        println!("\n=== Grid Creation Complete ===\n");
        grid
    }

    pub fn get_elements_at(&self, x: u32, y: u32) -> Vec<&GridElement> {
        path_service::get_elements_at(&self.elements, x, y)
    }

    pub fn should_draw_element(&self, element: &GridElement) -> bool {
        path_service::should_draw_element(
            element,
            self.width,
            self.height,
            &self.elements
        )
    }

    pub fn calculate_grid_position(
        x: u32, 
        y: u32, 
        grid_height: u32,
        offset_x: f32,
        offset_y: f32, 
        tile_size: f32
    ) -> (f32, f32) {
        // Convert to 0-based indexing
        let x_idx = x - 1;
        let y_idx = y - 1;
        
        // Invert y coordinate to match SVG coordinate system (top-left origin)
        let inverted_y = (grid_height - 1) - y_idx;
        
        let pos_x = offset_x + (x_idx as f32 * tile_size) + (tile_size / 2.0);
        let pos_y = offset_y + (inverted_y as f32 * tile_size) + (tile_size / 2.0);
        
        (pos_x, pos_y)
    }
}