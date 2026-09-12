/// Bounded recursive dependency traversal helpers.
///
/// SQLite uses recursive CTEs with bound depth; no graph DB needed in v1.
pub const DEFAULT_MAX_DEPTH: u8 = 3;

pub fn bounded_depth(depth: Option<u8>) -> u8 {
    depth.unwrap_or(DEFAULT_MAX_DEPTH).min(5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_three() {
        assert_eq!(bounded_depth(None), 3);
    }

    #[test]
    fn clamps_above_five() {
        assert_eq!(bounded_depth(Some(10)), 5);
        assert_eq!(bounded_depth(Some(255)), 5);
    }

    #[test]
    fn preserves_valid_depths_including_zero() {
        assert_eq!(bounded_depth(Some(0)), 0);
        assert_eq!(bounded_depth(Some(1)), 1);
        assert_eq!(bounded_depth(Some(5)), 5);
    }
}
