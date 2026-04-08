//! Thin CLI surface for TT v2.
//!
//! The canonical v2 CLI should be a narrow client over daemon APIs rather than
//! a second application layer.

use anyhow as _;
use tt_daemon as _;

pub const TT_CLI_GENERATION: &str = "v2";
