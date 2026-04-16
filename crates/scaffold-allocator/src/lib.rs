//! Test allocator that caps memory usage for scaffold test binaries.
//!
//! Add this crate as a dev-dependency and force-link it from test builds with
//! `#[cfg(test)] use scaffold_allocator as _;`.

