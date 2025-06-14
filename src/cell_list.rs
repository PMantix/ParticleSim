use crate::body::Body;
use ultraviolet::Vec2;

pub struct CellList {
    pub bounds: f32,
    pub cell_size: f32,
    grid_size: usize,
    cells: Vec<Vec<usize>>, // indices of bodies per cell
}

impl CellList {
    pub fn new(bounds: f32, cell_size: f32) -> Self {
        let grid_size = ((2.0 * bounds) / cell_size).ceil() as usize + 1;
        Self {
            bounds,
            cell_size,
            grid_size,
            cells: Vec::new(),
        }
    }

    pub fn rebuild(&mut self, bodies: &[Body]) {
        self.grid_size = ((2.0 * self.bounds) / self.cell_size).ceil() as usize + 1;
        self.cells.clear();
        self.cells.resize(self.grid_size * self.grid_size, Vec::new());
        for (i, b) in bodies.iter().enumerate() {
            let (cx, cy) = self.coord(b.pos);
            if cx < self.grid_size && cy < self.grid_size {
                self.cells[cx + cy * self.grid_size].push(i);
            }
        }
    }

    fn coord(&self, pos: Vec2) -> (usize, usize) {
        let min = -self.bounds;
        let x = ((pos.x - min) / self.cell_size).floor() as isize;
        let y = ((pos.y - min) / self.cell_size).floor() as isize;
        let x = x.clamp(0, self.grid_size as isize - 1) as usize;
        let y = y.clamp(0, self.grid_size as isize - 1) as usize;
        (x, y)
    }

    pub fn find_neighbors_within(&self, bodies: &[Body], i: usize, cutoff: f32, out: &mut Vec<usize>) {
        let (cx, cy) = self.coord(bodies[i].pos);
        let range = (cutoff / self.cell_size).ceil() as isize;
        out.clear();
        let cutoff_sq = cutoff * cutoff;
        for dy in -range..=range {
            for dx in -range..=range {
                let x = cx as isize + dx;
                let y = cy as isize + dy;
                if x < 0 || y < 0 || x >= self.grid_size as isize || y >= self.grid_size as isize {
                    continue;
                }
                let cell_idx = x as usize + y as usize * self.grid_size;
                for &idx in &self.cells[cell_idx] {
                    if idx != i {
                        let r2 = (bodies[idx].pos - bodies[i].pos).mag_sq();
                        if r2 < cutoff_sq {
                            out.push(idx);
                        }
                    }
                }
            }
        }
    }
}
