# Working in xpd

## Release and build invariants

1. **No unvendored system-header C crates in default builds.** Linux and musllinux wheels cross-compile with `maturin build --release --zig`, where system headers such as `/usr/include/openssl` are unavailable. Any Linux C dependency (including transitive proc-macro build dependencies such as `openssl-sys` via `sqlx-macros`) must be vendored in both `[target.'cfg(target_os = "linux")'.dependencies]` and `[target.'cfg(target_os = "linux")'.build-dependencies]`, or kept behind an opt-in feature.
2. **Keep `Release` builds fast and fail-fast.** Keep `lto = "thin"` and `codegen-units = 16` in `[profile.release]`, keep `cancel-in-progress: ${{ !startsWith(github.ref, 'refs/tags/') }}` in `.github/workflows/release.yml`, and keep all six wheel targets in one `fail-fast: true` matrix so wheel builds finish in under 10 minutes and never queue behind stale runs.
3. **Never push over a red `CI` or `Release` run on `main`.** `lefthook` `pre-push` mirrors `CI` (`cargo deny check`, `check_sample_marks.py`, `check_rule_conflicts.py`). Fix any failure locally before pushing.
4. **Ship a release in one push, then tag.** Bump `Cargo.toml`, `Cargo.lock`, `.claude-plugin/plugin.json`, and `mcp.json` together in one commit on `main`. Wait for that commit's `Release` wheel run on `main` to pass, then push the signed tag `vX.Y.Z` to publish to PyPI and create the GitHub Release.
