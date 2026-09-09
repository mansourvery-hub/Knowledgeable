/// Bounded recursive dependency traversal helpers.
///
/// SQLite uses recursive CTEs with bound depth; no graph DB needed in v1.
pub const DEFAULT_MAX_DEPTH: u8 = 3;

pub fn bounded_depth(depth: Option<u8>) -> u8 {
    depth.unwrap_or(DEFAULT_MAX_DEPTH).min(5)
}
