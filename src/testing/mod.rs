/// E2E testing module for keyboard-driven TUI interactions.
/// Provides a harness to spawn Zeta instances in PTYs and verify screen output.
pub mod e2e;

pub use e2e::ZetaE2eInstance;
