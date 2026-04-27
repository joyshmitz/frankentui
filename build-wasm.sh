#!/usr/bin/env bash
# build-wasm.sh — Maximum-optimized WASM build for the FrankenTUI showcase demo.
#
# Optimization pipeline:
#   1. Rust compiler: opt-level="z", LTO, codegen-units=1, panic=abort, strip
#   2. RUSTFLAGS: enable modern WASM features for better codegen
#   3. wasm-opt: -Oz --converge --all-features (runs passes until no improvement)
#
# Temporarily removes the ftui-extras opt-level=3 override (which bloats WASM
# by disabling size optimizations), builds the in-tree WASM showcase crate, and
# optionally builds an adjacent/out-of-tree frankenterm-web crate when explicitly
# requested via FRANKENTERM_WEB_CRATE_DIR.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

CARGO_TOML="Cargo.toml"
BACKUP="${CARGO_TOML}.bak"

# Ensure wasm-pack is available.
if ! command -v wasm-pack &>/dev/null; then
  echo "ERROR: wasm-pack is not installed. Install with: cargo install wasm-pack" >&2
  exit 1
fi

# ── Step 0: Set WASM-specific compiler flags ──────────────────────────────────
# Enable modern WASM features that all major browsers support (Chrome 91+,
# Firefox 89+, Safari 15+). These unlock better codegen from LLVM:
#   bulk-memory     — memcpy/memset as single wasm instructions
#   mutable-globals — avoid indirection for thread-local-like patterns
#   nontrapping-fptoint — faster float→int without trapping semantics
#   sign-ext        — i32.extend8_s etc, avoids shift-based sign extension
#   reference-types — required by some wasm-bindgen features
#   multivalue      — functions can return multiple values (avoids stack spills)
#
# IMPORTANT: Use CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS, NOT RUSTFLAGS.
# RUSTFLAGS applies to ALL compilations including proc macros (serde_derive,
# wasm-bindgen-macro) that compile for the HOST target, which would fail with
# WASM-specific target features.
WASM_FLAGS="-C target-feature=+bulk-memory,+mutable-globals,+nontrapping-fptoint,+sign-ext,+reference-types,+multivalue"
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} ${WASM_FLAGS}"

echo ">> WASM RUSTFLAGS: $CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS"

# ── Step 1: Patch Cargo.toml to remove ftui-extras opt-level override ────────
echo ">> Patching $CARGO_TOML (removing ftui-extras opt-level=3 override)..."

restore_cargo() {
  if [[ -f "$BACKUP" ]]; then
    echo ">> Restoring $CARGO_TOML..."
    mv "$BACKUP" "$CARGO_TOML"
  fi
}
trap restore_cargo EXIT

cp "$CARGO_TOML" "$BACKUP"

# Remove the [profile.release.package.ftui-extras] section and its opt-level line.
# This is a simple sed that removes both the section header and the opt-level line.
sed -i '/^\[profile\.release\.package\.ftui-extras\]$/,/^$/d' "$CARGO_TOML"
# Also remove the comment line before it if it's still there.
sed -i '/^# VFX-heavy crate: prefer speed over binary size/d' "$CARGO_TOML"

# ── Step 2: Build WASM crates ────────────────────────────────────────────────
# wasm-pack runs wasm-opt automatically using flags from
# [package.metadata.wasm-pack.profile.release] in each crate's Cargo.toml:
#   wasm-opt = ["-Oz", "--all-features", "--converge"]
if [[ -n "${FRANKENTERM_WEB_CRATE_DIR:-}" ]]; then
  if [[ ! -d "$FRANKENTERM_WEB_CRATE_DIR" ]]; then
    echo "ERROR: FRANKENTERM_WEB_CRATE_DIR does not exist: $FRANKENTERM_WEB_CRATE_DIR" >&2
    exit 1
  fi

  echo ">> Building external frankenterm-web from $FRANKENTERM_WEB_CRATE_DIR..."
  wasm-pack build "$FRANKENTERM_WEB_CRATE_DIR" \
    --target web \
    --out-dir "$SCRIPT_DIR/pkg/frankenterm-web" \
    --out-name FrankenTerm \
    --release
else
  echo ">> Skipping frankenterm-web: this checkout has no crates/frankenterm-web."
  echo "   Set FRANKENTERM_WEB_CRATE_DIR=/path/to/adjacent/frankenterm-web crate to build it."
fi

echo ">> Building ftui-showcase-wasm (demo runner)..."
wasm-pack build crates/ftui-showcase-wasm \
  --target web \
  --out-dir ../../pkg \
  --release

# ── Step 3: Report sizes ────────────────────────────────────────────────────
echo ""
echo "── WASM binary sizes ──"
while IFS= read -r f; do
  if [ -f "$f" ]; then
    size_bytes=$(stat -c%s "$f" 2>/dev/null || stat -f%z "$f" 2>/dev/null)
    size_mb=$(echo "scale=2; $size_bytes / 1048576" | bc)
    echo "  $f: ${size_mb} MB ($size_bytes bytes)"
  fi
done < <(find pkg -type f -name '*.wasm' | sort)

echo ""
echo "── Build complete ──"
echo "Serve from the project root with: python3 -m http.server 8080"
echo "Open: http://localhost:8080/crates/ftui-showcase-wasm/frankentui_showcase_demo.html"
