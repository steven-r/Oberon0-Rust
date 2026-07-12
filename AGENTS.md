# Agent Working Guide

This repository uses the following persistent collaboration rules:

## Communication

- Use English for documentation, code comments, issue text, and chat discussions.

## Commits

- Use Conventional Commit format.
- Prefer detailed commit bodies.
- Use issue trailers on separate lines when applicable:
  - Fixes: #123
  - Refs: #123

## Changelog

- Keep CHANGELOG.md Unreleased entries grouped by category.
- Describe impact, not internal implementation details only.

## Consistency Rules

- Keep README.md, example README files, relevant docs/*.md files, and CHANGELOG.md synchronized with behavior changes.
- If state output behavior changes, update example expected outputs.

## Validation Before Commit

- Run cargo test for behavior-affecting changes.
- Ensure pre-commit hooks pass.
- Keep repository examples runnable when touched.

## Source of Truth

- Project decision log: docs/project-decisions.md
- Contributor process: CONTRIBUTING.md
- Copilot behavior hints: .github/copilot-instructions.md
