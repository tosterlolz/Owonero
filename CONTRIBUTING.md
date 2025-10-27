Thank you for wanting to contribute to Owonero!

This document explains the recommended workflow, code style, tests, and how to create a clean pull request so maintainers can review and merge your changes quickly.

1) Reporting issues
- Search existing issues before opening a new one.
- Provide a concise title, steps to reproduce, expected vs actual behavior, and relevant logs or error output.
- For performance or mining issues include: OS, CPU model, number of threads used, exact command line, and any environment variables (e.g., OWONERO_MINING_ITERATIONS).

2) Feature requests and design proposals
- For larger changes open a discussion or an RFC-style issue describing the problem, proposed design, and backwards-compatibility concerns.
- Small features can be implemented directly as a PR but follow the PR guidelines below.

3) Branching and workflow
- Fork the repository and create a topic branch for your work: `git checkout -b feature/my-feature`
- Keep changes focused and small; one logical change per PR.
- Rebase/squash locally to keep a clean commit history if requested by reviewers.

4) Code style and formatting
- The project is Rust. Run `cargo fmt` before opening a PR.
- Run `cargo clippy` and fix warnings where feasible.
- Use descriptive variable and function names. Add comments for non-obvious logic.

5) Tests
- Add unit tests for new logic and functions. Prefer small, fast tests.
- Run the test suite locally: `cargo test`.
- If you add external dependencies, explain why they are needed in the PR description.

6) CI and checks
- Your PR must pass CI (GitHub Actions). The CI runs `cargo build --release` and `cargo test` on Linux, macOS and Windows runners.
- If CI fails for flaky tests, include logs and attempt to reproduce locally before asking maintainers to rerun.

7) Pull request checklist
- [ ] Descriptive title and summary
- [ ] Link to related issue (if any)
- [ ] Tests added/updated
- [ ] `cargo fmt` run
- [ ] No unresolved merge conflicts
- [ ] Verify CI passing

8) Security and sensitive data
- Do not include private keys, tokens, or other secrets in commits or issues.
- For security issues, please contact the maintainers privately (create a GitHub issue and mark it private if you need guidance) or follow the repository's security policy if provided.

9) Licensing and contributor agreement
- By opening a PR you confirm your contribution is licensed under the MIT license used by this project.

10) Help and communication
- For quick questions, open a discussion on GitHub or post in the repository's community channels (see README links).

Thanks again — we appreciate your contributions!