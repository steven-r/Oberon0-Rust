#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Generate a Markdown changelog section from Conventional Commits.

Usage:
  scripts/changelog.sh [--from-tag TAG] [--to-ref REF] [--output FILE] [--title TITLE]

Options:
  --from-tag TAG   Start range from TAG (exclusive). If omitted, uses full history.
  --to-ref REF     End range at REF (default: HEAD).
  --output FILE    Output path (default: stdout).
  --title TITLE    Section title (default: Unreleased).
EOF
}

FROM_TAG=""
TO_REF="HEAD"
OUTPUT_FILE=""
TITLE="Unreleased"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from-tag)
      FROM_TAG="${2:-}"
      shift 2
      ;;
    --to-ref)
      TO_REF="${2:-}"
      shift 2
      ;;
    --output)
      OUTPUT_FILE="${2:-}"
      shift 2
      ;;
    --title)
      TITLE="${2:-}"
      shift 2
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

if [[ -n "$FROM_TAG" ]]; then
  RANGE="${FROM_TAG}..${TO_REF}"
else
  RANGE="$TO_REF"
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

log_file="$TMP_DIR/log.tsv"
git log --format='%s%x09%b' "$RANGE" > "$log_file"

if [[ ! -s "$log_file" ]]; then
  CONTENT="## ${TITLE} - $(date +%Y-%m-%d)\n\n_No changes found in range ${RANGE}._\n"
  if [[ -n "$OUTPUT_FILE" ]]; then
    printf "%b" "$CONTENT" > "$OUTPUT_FILE"
  else
    printf "%b" "$CONTENT"
  fi
  exit 0
fi

declare -a FEATURES=()
declare -a FIXES=()
declare -a DOCS=()
declare -a REFACTOR=()
declare -a PERF=()
declare -a TESTS=()
declare -a CHORES=()
declare -a BUILD=()
declare -a CI=()
declare -a REVERT=()
declare -a BREAKING=()
declare -a OTHER=()
CONVENTIONAL_RE='^([a-z]+)(\([^)]*\))?(!)?:[[:space:]](.+)$'

while IFS=$'\t' read -r subject body; do
  [[ -z "${subject}" ]] && continue

  if [[ "$subject" =~ $CONVENTIONAL_RE ]]; then
    type="${BASH_REMATCH[1]}"
    bang="${BASH_REMATCH[3]}"
    text="${BASH_REMATCH[4]}"
  else
    type="other"
    bang=""
    text="$subject"
  fi

  entry="- ${text}"

  if [[ -n "$bang" || "$body" == *"BREAKING CHANGE:"* ]]; then
    BREAKING+=("${entry}")
  fi

  case "$type" in
    feat) FEATURES+=("${entry}") ;;
    fix) FIXES+=("${entry}") ;;
    docs) DOCS+=("${entry}") ;;
    refactor) REFACTOR+=("${entry}") ;;
    perf) PERF+=("${entry}") ;;
    test) TESTS+=("${entry}") ;;
    chore) CHORES+=("${entry}") ;;
    build) BUILD+=("${entry}") ;;
    ci) CI+=("${entry}") ;;
    revert) REVERT+=("${entry}") ;;
    *) OTHER+=("- ${subject}") ;;
  esac
done < "$log_file"

emit_section() {
  local heading="$1"
  shift
  local -a items=("$@")
  if [[ ${#items[@]} -gt 0 ]]; then
    printf "### %s\n" "$heading"
    printf "\n"
    printf "%s\n" "${items[@]}"
    printf "\n"
  fi
}

{
  printf "## %s - %s\n\n" "$TITLE" "$(date +%Y-%m-%d)"
  emit_section "Breaking Changes" "${BREAKING[@]}"
  emit_section "Features" "${FEATURES[@]}"
  emit_section "Fixes" "${FIXES[@]}"
  emit_section "Performance" "${PERF[@]}"
  emit_section "Refactoring" "${REFACTOR[@]}"
  emit_section "Documentation" "${DOCS[@]}"
  emit_section "Tests" "${TESTS[@]}"
  emit_section "Build" "${BUILD[@]}"
  emit_section "CI" "${CI[@]}"
  emit_section "Chores" "${CHORES[@]}"
  emit_section "Reverts" "${REVERT[@]}"
  emit_section "Other" "${OTHER[@]}"
}
> "$TMP_DIR/changelog.md"

if [[ -n "$OUTPUT_FILE" ]]; then
  cp "$TMP_DIR/changelog.md" "$OUTPUT_FILE"
else
  cat "$TMP_DIR/changelog.md"
fi
