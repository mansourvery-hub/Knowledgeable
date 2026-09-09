/// Bounded recursive dependency traversal helpers.
///
///
/// Will use recursive CTEs in PostgreSQL per `tech_stack_and_rules.md -> PostgreSQL Rules`.
pub const DEFAULT_MAX_DEPTH: u8 = 3;

pub fn bounded_depth(depth: Option<u8>) -> u8 {
    depth.unwrap_or(DEFAULT_MAX_DEPTH).min(5)
}
