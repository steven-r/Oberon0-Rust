# Versioning Policy

This project follows Semantic Versioning (SemVer):

`MAJOR.MINOR.PATCH`

## Rules

1. `MAJOR` is incremented for incompatible API or behavior changes.
2. `MINOR` is incremented for backward-compatible features.
3. `PATCH` is incremented for backward-compatible bug fixes.

## Current Version

The authoritative compiler version is defined in [Cargo.toml](Cargo.toml) under:

`[package].version`

Current value: `0.1.0`

## Commit Convention and Versioning

Commit messages must follow Conventional Commits, validated by the commit-msg pre-commit hook.

Recommended mapping for release planning:

1. `feat:` typically implies `MINOR`
2. `fix:` typically implies `PATCH`
3. `feat!:` or `BREAKING CHANGE:` implies `MAJOR`
