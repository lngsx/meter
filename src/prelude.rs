// Error things.
pub use miette::{Context, IntoDiagnostic, miette};

// Serde things.
pub use serde::{Deserialize, Serialize};

// Aliases.

/// The standard result for this application.
pub type AppResult<T = ()> = miette::Result<T>;
