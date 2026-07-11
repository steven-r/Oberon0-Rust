# Release Checklist

Use this checklist before creating a new release tag.

## 1. Branch and working tree

1. Confirm you are on the intended branch:

       git branch --show-current

2. Confirm working tree is clean:

       git status --short

## 2. Quality checks

1. Run hooks and checks:

       pre-commit run --all-files

2. Verify compiler builds:

       cargo check

3. Optional: verify example pipeline:

       cargo run -- examples/hello-app/src/Main.ob0 --manifest examples/hello-app/oberon.toml --out-dir target/generated

## 3. Generate preview changelog (optional)

1. From latest tag to HEAD:

       scripts/changelog.sh --from-tag "$(git describe --tags --abbrev=0)" --to-ref HEAD

## 4. Create release

1. Patch release:

       scripts/release.sh patch

2. Minor release:

       scripts/release.sh minor

3. Major release:

       scripts/release.sh major

4. Dry run without commit/tag:

       scripts/release.sh patch --dry-run

## 5. Verify release artifacts

1. Confirm latest commit and tag:

       git log --oneline -n 3
       git tag --list

2. Confirm version consistency:

       grep -E '^version = "' Cargo.toml

3. Confirm changelog entry exists:

       head -n 40 CHANGELOG.md
