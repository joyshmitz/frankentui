# External Asset Sources

This directory is reserved for provenance notes about user-provided or third-party
source archives used to generate checked-in showcase fixtures.

Do not commit raw ZIP/source bundles here. Keep converted, reviewable fixtures in
the crate that consumes them, and keep bulky local archives ignored.

Current local-only inputs:

- `quake-e1m1-the-slipgate-complex.zip` was used once to generate the
  `QUAKE_E1M1_VERTS` and `QUAKE_E1M1_TRIS` constants in
  `crates/ftui-demo-showcase/src/screens/3d_data.rs`.
