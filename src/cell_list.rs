use crate::body::Body;
use ultraviolet::Vec2;

pub struct CellList {
    pub domain_width: f32,  // Half-width of domain
    pub domain_height: f32, // Half-height of domain
    pub cell_size: f32,
    grid_size_x: usize,
    grid_size_y: usize,
    cells: Vec<Vec<usize>>, // indices of bodies per cell
}

impl CellList {
    pub fn new(domain_width: f32, domain_height: f32, cell_size: f32) -> Self {
        let grid_size_x = ((2.0 * domain_width) / cell_size).ceil() as usize + 1;
        let grid_size_y = ((2.0 * domain_height) / cell_size).ceil() as usize + 1;
        Self {
            domain_width,
            domain_height,
            cell_size,
            grid_size_x,
            grid_size_y,
            cells: Vec::new(),
        }
    }

    pub fn rebuild(&mut self, bodies: &[Body]) {
        self.grid_size_x = ((2.0 * self.domain_width) / self.cell_size).ceil() as usize + 1;
        self.grid_size_y = ((2.0 * self.domain_height) / self.cell_size).ceil() as usize + 1;
        self.cells.clear();
        self.cells.resize(self.grid_size_x * self.grid_size_y, Vec::new());
        for (i, b) in bodies.iter().enumerate() {
            let (cx, cy) = self.coord(b.pos);
            if cx < self.grid_size_x && cy < self.grid_size_y {
                self.cells[cx + cy * self.grid_size_x].push(i);
            }
        }
    }

    pub fn update_domain_size(&mut self, domain_width: f32, domain_height: f32) {
        self.domain_width = domain_width;
        self.domain_height = domain_height;
        // Grid size will be recalculated on next rebuild
    }

    fn coord(&self, pos: Vec2) -> (usize, usize) {
        let min_x = -self.domain_width;
        let min_y = -self.domain_height;
        let x = ((pos.x - min_x) / self.cell_size).floor() as isize;
        let y = ((pos.y - min_y) / self.cell_size).floor() as isize;
        let x = x.clamp(0, self.grid_size_x as isize - 1) as usize;
        let y = y.clamp(0, self.grid_size_y as isize - 1) as usize;
        (x, y)
    }

    pub fn find_neighbors_within(&self, bodies: &[Body], i: usize, cutoff: f32) -> Vec<usize> {
        let (cx, cy) = self.coord(bodies[i].pos);
        let range = (cutoff / self.cell_size).ceil() as isize;
        let mut neighbors = Vec::new();
        let cutoff_sq = cutoff * cutoff;
        for dy in -range..=range {
            for dx in -range..=range {
                let x = cx as isize + dx;
                let y = cy as isize + dy;
                if x < 0 || y < 0 || x >= self.grid_size_x as isize || y >= self.grid_size_y as isize {
                    continue;
                }
                let cell_idx = x as usize + y as usize * self.grid_size_x;
                for &idx in &self.cells[cell_idx] {
                    if idx != i {
                        let r2 = (bodies[idx].pos - bodies[i].pos).mag_sq();
                        if r2 < cutoff_sq {
                            neighbors.push(idx);
                        }
                    }
                }
            }
        }
        neighbors
    }

    /// Count nearby metallic neighbors within `cutoff` distance of body `i`.
    ///
    /// This uses the already constructed cell list and avoids any heap
    /// allocations. Only neighbors whose species is `LithiumMetal` or
    /// `FoilMetal` are counted.
    pub fn metal_neighbor_count(&self, bodies: &[Body], i: usize, cutoff: f32) -> usize {
        use crate::body::Species;
        let (cx, cy) = self.coord(bodies[i].pos);
        let range = (cutoff / self.cell_size).ceil() as isize;
        let cutoff_sq = cutoff * cutoff;
        let mut count = 0usize;
        for dy in -range..=range {
            for dx in -range..=range {
                let x = cx as isize + dx;
                let y = cy as isize + dy;
                if x < 0 || y < 0 || x >= self.grid_size_x as isize || y >= self.grid_size_y as isize {
                    continue;
                }
                let cell_idx = x as usize + y as usize * self.grid_size_x;
                for &idx in &self.cells[cell_idx] {
                    if idx == i { continue; }
                    let r2 = (bodies[idx].pos - bodies[i].pos).mag_sq();
                    if r2 < cutoff_sq
                        && matches!(bodies[idx].species, Species::LithiumMetal | Species::FoilMetal)
                    {
                        count += 1;
                    }
                }
            }
        }
        count
    }
}
