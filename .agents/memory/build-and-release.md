<!-- agent-memory v1.0 -->

# Build and Release — orai

## Facts
- [2026-04-11] CI builds 8 targets on tag push via .github/workflows/release.yml
- [2026-04-11] Targets: Linux musl/glibc (x86_64 + aarch64), macOS (x86_64 + aarch64), Android (aarch64 + x86_64)
- [2026-04-11] Homebrew tap at ~/src/homebrew-orai (manual update until HOMEBREW_TAP_TOKEN is set)
- [2026-04-11] Landing page at https://ergofobe.github.io/orai/ deployed via .github/workflows/pages.yml
- [2026-04-11] Install script at https://ergofobe.github.io/orai/install.sh (macOS/Linux/Android)

## Decisions
- [2026-04-11] Use musl for universal Linux binary, glibc as alternative
- [2026-04-11] Use cargo-ndk for Android builds (not cross — it fails with missing libunwind)
- [2026-04-11] Version bump right before tagging, not during development
- [2026-04-11] Development on main branch — no feature branches

## Gotchas
- [2026-04-11] macOS sed -i requires '' arg — handled in release.yml
- [2026-04-11] Homebrew formula must be updated manually — HOMEBREW_TAP_TOKEN not set in CI
- [2026-04-11] cross tool fails for Android — use cargo-ndk with pre-installed NDK on CI runners