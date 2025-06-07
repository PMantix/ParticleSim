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