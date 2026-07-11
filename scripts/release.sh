#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Create a SemVer release: bump version, update changelog, commit, and tag.

Usage:
  scripts/release.sh <patch|minor|major> [--dry-run]

Examples:
  scripts/release.sh patch
  scripts/release.sh minor --dry-run
EOF
}

if [[ $# -lt 1 ]]; then
  usage >&2
  exit 2
fi

BUMP_TYPE="$1"
shift

DRY_RUN="false"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! "$BUMP_TYPE" =~ ^(patch|minor|major)$ ]]; then
  echo "Invalid bump type: $BUMP_TYPE" >&2
  usage >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

current_version="$(grep -E '^version = "[0-9]+\.[0-9]+\.[0-9]+"$' Cargo.toml | head -n1 | sed -E 's/version = "([0-9]+\.[0-9]+\.[0-9]+)"/\1/')"
if [[ -z "$current_version" ]]; then
  echo "Failed to read version from Cargo.toml" >&2
  exit 1
fi

IFS='.' read -r major minor patch <<< "$current_version"
case "$BUMP_TYPE" in
  patch)
    patch=$((patch + 1))
    ;;
  minor)
    minor=$((minor + 1))
    patch=0
    ;;
  major)
    major=$((major + 1))
    minor=0
    patch=0
    ;;
esac
new_version="${major}.${minor}.${patch}"
new_tag="v${new_version}"

if git rev-parse "$new_tag" >/dev/null 2>&1; then
  echo "Tag already exists: $new_tag" >&2
  exit 1
fi

last_tag="$(git describe --tags --abbrev=0 2>/dev/null || true)"

tmp_section="$(mktemp)"
if [[ -n "$last_tag" ]]; then
  scripts/changelog.sh --from-tag "$last_tag" --to-ref HEAD --title "$new_tag" --output "$tmp_section"
else
  scripts/changelog.sh --to-ref HEAD --title "$new_tag" --output "$tmp_section"
fi

if [[ "$DRY_RUN" == "true" ]]; then
  echo "Dry run completed. Proposed version: ${new_version}" >&2
  echo "Preview changelog section:" >&2
  cat "$tmp_section"
  echo "No files were modified." >&2
  exit 0
fi

if [[ ! -f CHANGELOG.md ]]; then
  cat > CHANGELOG.md <<'EOF'
# Changelog

All notable changes to this project will be documented in this file.

EOF
fi

new_changelog="$(mktemp)"
{
  head -n 3 CHANGELOG.md
  printf "\n"
  cat "$tmp_section"
  printf "\n"
  tail -n +4 CHANGELOG.md
} > "$new_changelog"
mv "$new_changelog" CHANGELOG.md

sed -i -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"$/s//version = \"${new_version}\"/" Cargo.toml
sed -i -E "s/^Current value: \`[0-9]+\.[0-9]+\.[0-9]+\`$/Current value: \`${new_version}\`/" VERSIONING.md

cargo check

git add Cargo.toml VERSIONING.md CHANGELOG.md
git commit -m "chore(release): ${new_tag}"
git tag -a "$new_tag" -m "$new_tag"

echo "Release created: ${new_tag}" >&2
