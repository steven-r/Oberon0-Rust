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

Current value: `0.6.0`

## Commit Convention and Versioning

Commit messages must follow Conventional Commits, validated by the commit-msg pre-commit hook.

Recommended mapping for release planning:

1. `feat:` typically implies `MINOR`
2. `fix:` typically implies `PATCH`
3. `feat!:` or `BREAKING CHANGE:` implies `MAJOR`

## Automated workflow

This repository provides local scripts for changelog generation and SemVer releases:

1. `scripts/changelog.sh` generates Markdown release notes from Conventional Commits.
2. `scripts/release.sh <patch|minor|major>` bumps version, updates changelog, creates a release commit, and creates an annotated tag.

Use dry run mode to preview release effects without commit/tag:

`scripts/release.sh patch --dry-run`
