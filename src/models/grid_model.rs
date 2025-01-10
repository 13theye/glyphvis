use crate::models::data_model::Project;
use std::collections::HashMap;
use crate::services::grid_service;
use crate::services::grid_service::{ PathElement, GridElement, ViewBox };

#[derive(Debug)]
pub struct Grid {
    pub elements: HashMap<String, GridElement>,
    pub width: u32,
    pub height: u32,
    pub viewbox: ViewBox,
}

impl Grid {
    pub fn new(project: &Project) -> Self {
        let mut elements = HashMap::new();
        let viewbox = ViewBox {
            min_x: 0.0,
            min_y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        
        // Parse base SVG elements
        let base_elements: Vec<(String, PathElement)> = project.svg_base_tile
            .lines()
            .filter(|line| line.contains("<path") || line.contains("<circle"))
            .filter_map(|line| {
                // Extract ID
                //println!("line: {}", line);
                if let Some(id_start) = line.find("id=\"") {
                    if let Some(id_end) = line[id_start + 4..].find('\"') {
                        let id = line[id_start + 4..id_start + 4 + id_end].to_string();
                        if let Some(element) = grid_service::parse_svg_element(line) {
                            return Some((id, element));
                        }
                    }
                }
                None
            })
            .collect();

        // Generate grid elements
        for y in 0..project.grid_y {
            for x in 0..project.grid_x {
                for (base_id, base_path) in &base_elements {
                    let grid_id = format!("{},{} : {}", x, y, base_id);
                    let edge_type = grid_service::detect_edge_type(base_path, &viewbox);
                    let element = GridElement {
                        id: base_id.clone(),
                        position: (x, y),
                        path: base_path.clone(),
                        edge_type,
                    };
                    elements.insert(grid_id, element);
                }
            }
        }

        Grid {
            elements,
            width: project.grid_x,
            height: project.grid_y,
            viewbox,
        }
    }

    pub fn get_elements_at(&self, x: u32, y: u32) -> Vec<&GridElement> {
        grid_service::get_elements_at(&self.elements, x, y)
    }

    pub fn should_draw_element(&self, element: &GridElement) -> bool {
        grid_service::should_draw_element(
            element,
            self.width,
            self.height,
            &self.elements
        )
    }
}