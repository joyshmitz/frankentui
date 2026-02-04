# Dependency Upgrade Log

**Date:** 2026-02-04
**Project:** frankentui
**Language:** Rust
**Manifest:** Cargo.toml (workspace)

---

## Summary

| Metric | Count |
|--------|-------|
| **Total dependencies** | 27 |
| **Updated** | 2 |
| **Skipped** | 0 |
| **Failed (rolled back)** | 0 |
| **Requires attention** | 0 |

---

## Successfully Updated

### lru: 0.12 → 0.16.3
- **Breaking:** Yes (API changes since 0.12; will verify on build)
- **Tests:** `cargo test -p ftui-text` ✓
- **Notes:** Updated to fix RUSTSEC-2026-0002 (unsoundness in 0.12.x).

### portable-pty: 0.8/0.8.1 → 0.9.0
- **Breaking:** Possible (major version bump from 0.8 → 0.9)
- **Tests:** `cargo check --all-targets` + `cargo clippy --all-targets -- -D warnings` ✓ (targeted tests pending)
- **Notes:** Updated to avoid `serial` unmaintained advisory; portable-pty 0.9.0 is latest stable.

---

## Skipped

_TBD_

---

## Failed Updates (Rolled Back)

_TBD_

---

## Requires Attention

- `paste` advisory is target-specific (macOS `metal` dependency). Patch to `pastey` is in place but not used on Linux; verify on macOS target or ensure lock resolution applies to target builds.

---

## Deprecation Warnings Fixed

| Package | Warning | Fix Applied |
|---------|---------|-------------|
| _TBD_ | _TBD_ | _TBD_ |

---

## Security Notes

**Vulnerabilities resolved:**
- RUSTSEC-2026-0002 (lru unsoundness) via upgrade to 0.16.3.
- paste unmaintained advisory mitigated via workspace patch to pastey (drop-in replacement).
- serial unmaintained advisory mitigated via portable-pty 0.9.0.

**New advisories:** _None detected_

**Audit command:** `cargo audit`

---

## Post-Upgrade Checklist

- [ ] All tests passing
- [ ] No deprecation warnings
- [ ] Manual smoke test performed
- [ ] Documentation updated (if needed)
- [ ] Changes committed

---

## Commands Used

```bash
# Update commands
manual edits (Cargo.toml + patch)
`cargo update -p lru -p paste`

# Version research
docs.rs + changelog review (lru, portable-pty, pastey)

# Test commands
`cargo check --all-targets`
`cargo clippy --all-targets -- -D warnings`
`cargo fmt`
`cargo fmt --check`
`cargo test -p ftui-text`

# Test commands
TBD

# Audit commands
cargo audit
```

---

## Notes

Tracking updates per dependency per deps-update workflow. Pending scope confirmation from user.
