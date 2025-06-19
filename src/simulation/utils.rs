use crate::body::Body;

/// Returns true if an electron should be allowed to hop from src to dst
pub fn can_transfer_electron(src: &Body, dst: &Body) -> bool {
    // Calculate the difference between the current and the neutral count.
    // A positive value means surplus; zero means neutral; negative means deficiency.
    let src_diff = src.electrons.len() as i32 - src.neutral_electron_count() as i32;
    let dst_diff = dst.electrons.len() as i32 - dst.neutral_electron_count() as i32;
    
    // If source has neutral or surplus electrons, transfer is allowed when donor is “richer” than acceptor.
    if src_diff >= 0 {
        src_diff > dst_diff
    } else {
        // If source is deficient, only allow transfer if it is very deficient (for example, less than -1)
        // and the target is also deficient.
        src_diff < -1 && dst_diff < 0
    }
}

/// Select all connected FoilMetal bodies within a cutoff distance using BFS.
pub fn select_clump(bodies: &[Body], start: usize, cutoff: f32) -> Vec<usize> {
    use crate::body::Species;
    let mut visited = vec![false; bodies.len()];
    let mut stack = vec![start];
    let mut result = Vec::new();
    while let Some(idx) = stack.pop() {
        if idx >= bodies.len() || visited[idx] { continue; }
        if bodies[idx].species != Species::FoilMetal { continue; }
        visited[idx] = true;
        result.push(idx);
        for (i, other) in bodies.iter().enumerate() {
            if !visited[i] && other.species == Species::FoilMetal {
                if (other.pos - bodies[idx].pos).mag() <= cutoff {
                    stack.push(i);
                }
            }
        }
    }
    result
}