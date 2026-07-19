//! Request-scoped context types for keyed counter server functions.

/// Partition / user id provided by the page during SSR (before cookies exist).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E2ePartition(pub String);
