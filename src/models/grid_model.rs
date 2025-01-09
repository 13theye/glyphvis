// src/models/grid.rs

use std::collections::HashMap;
use super::data_model::Project;
use crate::services::grid_service::{self, PathElement, ViewBox, EdgeType};

#[derive(Debug)]
pub struct Grid {
    pub elements: HashMap<String, GridElement>,
    pub width: u32,
    pub height: u32,
    pub viewbox: ViewBox,
}

#[derive(Debug)]
pub struct GridElement {
    pub id: String,           // e.g. "ver-1-1"
    pub position: (u32, u32), // grid coordinates (x,y)
    pub path: PathElement,    // the actual SVG path data
    pub edge_type: EdgeType,  // edge detection information
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
                println!("line: {}", line);
                // Extract ID
                if let Some(id_start) = line.find("id=\"") {
                    if let Some(id_end) = line[id_start + 4..].find('\"') {
                        let id = line[id_start + 4..id_start + 4 + id_end].to_string();
                        //println!("id: {}", id);
                        if let Some(element) = grid_service::parse_svg_element(line) {
                            //println!("element: {:#?}", element);
                            return Some((id, element));
                        }       
                    }
                }
                None
            })
            .collect();

        //println!("Base elements: {:#?}", base_elements);

        // Generate grid elements for each position
        for y in 0..project.grid_y {
            for x in 0..project.grid_x {
                for (base_id, base_path) in &base_elements {
                    let grid_id = format!("{},{} : {}", x, y, base_id);
                    
                    // Detect edge type for this element
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

        println!("Elements: {:#?}", elements);

        // Remove redundant edges
        //Grid::remove_redundant_edges(&mut elements, project.grid_x, project.grid_y);

        Grid {
            elements,
            width: project.grid_x,
            height: project.grid_y,
            viewbox,
        }
    }

    fn remove_redundant_edges(elements: &mut HashMap<String, GridElement>, width: u32, height: u32) {
        // Create a list of elements to remove
        let mut to_remove = Vec::new();
        
        // Helper closure to get grid ID
        let get_grid_id = |x: u32, y: u32, id: &str| -> String {
            format!("{},{} : {}", x, y, id)
        };

        // Check each element
        for (grid_id, element) in elements.iter() {
            let (x, y) = element.position;
            
            match element.edge_type {
                EdgeType::North if y > 0 => {
                    // If there's a South edge in the tile above, mark this for removal
                    let above_y = y - 1;
                    if elements.values().any(|e| {
                        e.position == (x, above_y) && 
                        e.edge_type == EdgeType::South
                    }) {
                        to_remove.push(grid_id.clone());
                    }
                },
                EdgeType::South if y < height - 1 => {
                    // Keep southern edges (they'll be removed when processing northern edges)
                },
                EdgeType::East if x < width - 1 => {
                    // If there's a West edge in the tile to the right, mark this for removal
                    let right_x = x + 1;
                    if elements.values().any(|e| {
                        e.position == (right_x, y) && 
                        e.edge_type == EdgeType::West
                    }) {
                        to_remove.push(grid_id.clone());
                    }
                },
                EdgeType::West if x > 0 => {
                    // Keep western edges (they'll be removed when processing eastern edges)
                },
                // Handle corners similarly
                EdgeType::Northwest | EdgeType::Northeast | EdgeType::Southwest | EdgeType::Southeast => {
                    // Remove based on position in grid
                    if x > 0 && y > 0 {
                        to_remove.push(grid_id.clone());
                    }
                },
                _ => {}
            }
        }

        // Remove the redundant elements
        for grid_id in to_remove {
            elements.remove(&grid_id);
        }
    }

    pub fn get_element(&self, x: u32, y: u32, id: &str) -> Option<&GridElement> {
        let grid_id = format!("{},{} : {}", x, y, id);
        self.elements.get(&grid_id)
    }

    pub fn get_elements_at(&self, x: u32, y: u32) -> Vec<&GridElement> {
        self.elements
            .iter()
            .filter(|(_, element)| element.position == (x, y))
            .map(|(_, element)| element)
            .collect()
    }

    pub fn should_draw_element(&self, element: &GridElement) -> bool {
        // If it's not an edge, always draw it
        //if element.edge_type == EdgeType::None {
            {return true;
        }

        let (x, y) = element.position;
        
        // For edges, check if we should draw based on position and edge type
        match element.edge_type {
            EdgeType::North => y == 0,
            EdgeType::South => y == self.height - 1,
            EdgeType::East => x == self.width - 1,
            EdgeType::West => x == 0,
            EdgeType::Northwest => x == 0 && y == 0,
            EdgeType::Northeast => x == self.width - 1 && y == 0,
            EdgeType::Southwest => x == 0 && y == self.height - 1,
            EdgeType::Southeast => x == self.width - 1 && y == self.height - 1,
            EdgeType::None => true,
        }
    }
}