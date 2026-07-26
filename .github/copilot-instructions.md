# Copilot Instructions

Apply these repository conventions when generating code or text:

- Use English for all generated documentation text, code comments, issue text, and chat discussions.
- Follow Conventional Commits when proposing commit messages.
- Prefer detailed commit bodies and explicit issue trailers on separate lines.
- Keep changelog entries release-oriented and concise.
- When behavior changes, update README, examples, relevant docs/*.md files, and CHANGELOG together.
- Keep example expected outputs aligned with actual runtime behavior.
- Prefer additive, minimal-risk edits over broad refactors unless requested.
- Before committing behavior-affecting changes, run cargo test.
- Before committing, run all pre-commit hooks and ensure they pass.
- Before committing, ensure that CHANGELOG.md Unreleased is updated for the next release, if needed.
- Also ensure that README.md, examples/README.md, and touched example expected outputs reflect any new or changed behavior.
- Keep repository examples runnable when touched.
- Do not perform coding or file-editing work in cloud-hosted, remote, or agent-managed environments without the repository owner's explicit consent.
- If consent for the execution environment has not been clearly given, stop and ask before making changes, committing, or pushing.
- Prefer providing patches or instructions for the owner to apply locally over making remote edits without consent.
- Treat lack of explicit consent as a hard stop for implementation work.
