# Project Decisions

This file is the single source of truth for team-level working agreements.

## Communication

- Use English for documentation, code comments, issue text, and chat discussions.
- Keep wording precise and implementation-oriented.

## Commit Policy

- Use Conventional Commits.
- Prefer detailed commit messages with:
  - one concise title
  - one or more explanatory body paragraphs
  - issue trailers on separate lines when applicable
- Preferred trailers:
  - Fixes: #123
  - Refs: #123

## Changelog Policy

- Keep CHANGELOG.md release-oriented and readable.
- Group entries under Unreleased by category:
  - Features
  - Fixes
  - Documentation
  - Tests
  - Chores
- Write concise bullets that describe user-visible impact.

## Documentation Policy

- Document language subset and behavior changes in docs/*.md.
- Keep examples and their README expected outputs in sync.
- When behavior changes, update README.md and CHANGELOG.md in the same change.

## Validation Before Commit

- Run cargo test for behavior-affecting changes.
- Ensure pre-commit hooks pass.
- Keep repository examples runnable when touched.

## Current Known Limitation

- Procedure-local VAR declarations are not yet supported in the current subset.
- Procedure-scope shadowing examples use parameters until this is implemented.
- Tracking issue: #16.

## Decision Update Process

When a new team convention is agreed:

1. Update this file.
2. If tooling behavior should follow it, update AGENTS.md and .github/copilot-instructions.md.
3. Add a short note to CHANGELOG.md if it affects contributor workflow.
