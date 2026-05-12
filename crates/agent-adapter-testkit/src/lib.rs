//! Testkit crate for agent-adapter.

pub mod contract;
pub mod design_traceability;
pub mod docs_lint;
pub mod fake;
pub mod fixtures;
pub mod legacy_policy_lint;
pub mod real_provider_gates;
pub mod schema_lint;
pub use contract::*;
pub use design_traceability::*;
pub use docs_lint::*;
pub use fake::*;
pub use fixtures::*;
pub use legacy_policy_lint::*;
pub use real_provider_gates::*;
pub use schema_lint::*;
