//! Contract-derived constants.
//!
//! This module is rewritten by `scripts/generate_contract.py` from the pinned
//! OpenCode OpenAPI snapshot. It intentionally contains protocol metadata only
//! in the first vertical slice; complex OpenAPI 3.1 models remain behind the
//! handwritten compatibility layer until a generator proves correct for them.

include!("operations.rs");
