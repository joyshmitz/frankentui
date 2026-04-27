# Reproduction Archives

This directory keeps historical reproduction snippets that are useful for
debugging or future regression-test promotion but are not part of the active
Cargo test matrix. Files with the `.rs.txt` suffix are Rust snippets preserved
as text on purpose.

Active regression tests should live in a crate's `tests/` directory, the
workspace `tests/` directory, or an inline `#[cfg(test)]` module that is wired
from the crate root.
