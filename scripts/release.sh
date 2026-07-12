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

normalize_markdown_spacing() {
  local input_file="$1"
  awk '
    BEGIN { blank = 0 }
    {
      if ($0 ~ /^[[:space:]]*$/) {
        blank++
        if (blank <= 1) {
          print ""
        }
      } else {
        blank = 0
        print
      }
    }
  ' "$input_file"
}

trim_outer_blank_lines() {
  local input_file="$1"
  awk '
    { lines[NR] = $0 }
    END {
      start = 1
      while (start <= NR && lines[start] ~ /^[[:space:]]*$/) {
        start++
      }

      finish = NR
      while (finish >= start && lines[finish] ~ /^[[:space:]]*$/) {
        finish--
      }

      for (i = start; i <= finish; i++) {
        print lines[i]
      }
    }
  ' "$input_file"
}

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

if [[ ! -f CHANGELOG.md ]]; then
  cat > CHANGELOG.md <<'EOF'
# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

EOF
fi

if ! grep -q '^## Unreleased$' CHANGELOG.md; then
  echo "CHANGELOG.md must contain a '## Unreleased' section before creating a release" >&2
  exit 1
fi

tmp_unreleased_raw="$(mktemp)"
tmp_unreleased_body="$(mktemp)"
tmp_prefix="$(mktemp)"
tmp_suffix="$(mktemp)"

awk '
  BEGIN { capture = 0 }
  /^## Unreleased$/ { capture = 1; next }
  capture && /^## / { exit }
  capture { print }
' CHANGELOG.md > "$tmp_unreleased_raw"

trim_outer_blank_lines "$tmp_unreleased_raw" > "$tmp_unreleased_body"

if [[ ! -s "$tmp_unreleased_body" ]]; then
  echo "CHANGELOG.md Unreleased section is empty; update it before creating a release" >&2
  exit 1
fi

release_date="$(date +%Y-%m-%d)"
tmp_section="$(mktemp)"
{
  printf "## %s - %s\n\n" "$new_tag" "$release_date"
  cat "$tmp_unreleased_body"
  printf "\n"
} > "$tmp_section"

if [[ "$DRY_RUN" == "true" ]]; then
  echo "Dry run completed. Proposed version: ${new_version}" >&2
  echo "Preview changelog section:" >&2
  cat "$tmp_section"
  echo "No files were modified." >&2
  exit 0
fi

awk '
  /^## Unreleased$/ { exit }
  { print }
' CHANGELOG.md > "$tmp_prefix"

awk '
  BEGIN { capture = 0 }
  /^## Unreleased$/ { capture = 1; next }
  capture && /^## / { print; capture = 0; next }
  !capture { print }
' CHANGELOG.md > "$tmp_suffix"

new_changelog="$(mktemp)"
{
  cat "$tmp_prefix"
  printf "## Unreleased\n\n"
  cat "$tmp_section"
  cat "$tmp_suffix"
} > "$new_changelog"

normalized_changelog="$(mktemp)"
normalize_markdown_spacing "$new_changelog" > "$normalized_changelog"
mv "$normalized_changelog" CHANGELOG.md

sed -i -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"$/s//version = \"${new_version}\"/" Cargo.toml
sed -i -E "s/^Current value: \`[0-9]+\.[0-9]+\.[0-9]+\`$/Current value: \`${new_version}\`/" VERSIONING.md

cargo check

git add Cargo.toml Cargo.lock VERSIONING.md CHANGELOG.md
git commit -m "chore(release): ${new_tag}"
git tag -a "$new_tag" -m "$new_tag"

echo "Release created: ${new_tag}" >&2
