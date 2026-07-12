# Contributing

Thanks for contributing to this project.

## Start Here

- Read docs/project-decisions.md for the current team agreements.
- Follow VERSIONING.md and RELEASE_CHECKLIST.md for release-related work.

## Commit Messages

- Use Conventional Commits.
- Prefer detailed commit messages with a descriptive body.
- Use issue trailers on separate lines when relevant:
  - Fixes: #123
  - Refs: #123

## Quality Checks

Before opening a pull request:

- Run cargo test.
- Run pre-commit hooks.
- Ensure examples you touched still run and match documented output.

## Documentation Expectations

When behavior changes:

- Update README.md where needed.
- Update example README files where needed.
- Update CHANGELOG.md Unreleased section.
