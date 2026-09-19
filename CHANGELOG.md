# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
with the 0.x policy described in the README: breaking changes bump the minor
version.

## [Unreleased]

## [0.1.0] - 2026-09-19

### Changed

- `NavLink::param` is only available on links created with `NavLink::named`.
  Calling it on `NavLink::to` is a compile error instead of a panic.
- `NavigationKind` and `NavigationEvent` are `#[non_exhaustive]` so 0.x can
  add variants and fields without a major bump.

### Added

- Crate metadata for crates.io (`description`, `repository`, `rust-version`,
  keywords, and categories).
