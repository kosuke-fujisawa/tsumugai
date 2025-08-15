This file defines the development rules and guardrails for Claude Code when contributing to the tsumugai project.

# Platform

- Rust (library crate)
- Tauri (backend IPC and platform layer)
- Svelte (frontend UI)

# Principles

- Always operate on code, not UI editors.
- Maintain a clean Rust API, modular structure, and clear src/lib.rs entry.
- Separate UI logic (Svelte) from domain logic (Rust).
- Do not touch frontend code unless explicitly requested.
- Ensure all Rust code compiles and passes `cargo test`.

# Tasks Claude is Allowed to Perform

- Add or modify `.rs` modules under `src/`
- Implement Markdown command parsers and command emitters
- Write integration tests for command parsing
- Maintain crate-level documentation and examples
- Add utilities for IPC communication with Tauri frontend

# ❌ Tasks Claude Must Not Perform

- Modify `src-tauri/tauri.conf.json` or any Svelte files
- Introduce dependencies without asking
- Create binaries; this is a library crate
- Touch assets or runtime UI rendering logic

## Asset Restriction Rules

Assets in `assets/` directory (BGM, images, sounds, scenarios) are provided for testing and demonstration purposes only. These files:

- Must NOT be modified unless explicitly requested for testing purposes
- Serve as test data for resource resolution and parser validation
- Follow the BasicResolver naming conventions for integration testing
- Are excluded from production builds (library users provide their own assets)

# ✅ Example Flow

When asked to implement support for a new command:

1.  Create a new variant in the `Command` enum.
2.  Extend the Markdown parser to detect and emit this variant.
3.  Add tests for round-trip parsing.
4.  Document it in `README.md` and `CLAUDE.md`.

# Testing

All code must include tests:

```bash
cargo test
```

Use `#[cfg(test)]` blocks and validate parsers, command structures, and context handlers.

# File Structure

```
src/
├── lib.rs          # public API
├── parser.rs       # Markdown -> Commands
├── command.rs      # Command enum and helpers
├── engine.rs       # Trait defining how commands are executed
├── context.rs      # Variable and state management
└── test_data/      # Sample scripts for testing
```

Claude Code should always clarify before acting when ambiguity exists, and document the location, role, and impact of each generated artifact.

# Development Rules

## 1. Confirm-First Protocol (CFP)

-   **Application:** Before running the API, clearly define the "target values," "acceptance criteria," and "scope of changes" for each issue.
-   **Example:**
    -   **Objective:** Achieve 100% detection rate for undefined labels in `parse()`.
    -   **Acceptance:** The error type must include the line number. `cargo test parser::labels` must pass. Minimal reproduction via CLI.
    -   **Scope:** Only `parse/*`, `engine/*` is not targeted.
-   **Benefit:** Prevents implementation from getting lost due to ambiguous specifications.

## 2. TDD (Red→Green→Refactor)

-   **Application:** First, write a snapshot test for the Directive, then implement the minimal functionality, and finally, refactor.
-   **Example:** In `tests/golden_mixed_ja_en.rs`, compare `[SAY]` / `[BRANCH]` using JSON.

## 3. Separate Structural and Behavioral Changes

-   **Application:** Folder moves and type renames should be in separate PRs. Do not mix them with behavior changes in `step()`.
-   **Benefit:** Makes it easier to isolate the cause of regressions.

## 4. Commit Discipline (Small, Independent, Descriptive)

-   **Application:** Strictly adhere to one logical change per commit, e.g., `feat(parser): WAIT “1.5s” support`.
-   Continue to follow the zero-warning standard in CI.

## 5. Issue Template & 3-Point Acceptance Criteria

-   **Application:** For each issue, always include: ① Logs/Metrics (e.g., `parse.errors.missing_label`), ② Visual Confirmation (reproduction steps), ③ Test Name.
-   Since tsumugai has no UI, ② is replaced by CLI reproduction.

## 6. Debate-First Review Process

-   **Application:** For specification-related discussions, first present your position, impact, and alternatives in JSON format. Start implementation after receiving "approved-to-apply."
-   **Benefit:** Prevents breaking changes to core APIs (`Engine::step`/`Directive`).

## 7. Quality Principles (Single Responsibility, Avoid Over-Abstraction, Early Return)

-   **Application:** `parser/*` should be pure functions, `engine/*` should only handle state transitions, and `resolve/*` should be limited to I/O boundaries.
-   "Presentation" logic is strictly out of scope.

## 8. Error Handling Policy

-   **Application:** Always include row and column numbers in errors, e.g., `Error::MissingLabel { name, line, col }`. Include alternative suggestions (e.g., candidate labels) in the message.
-   This aligns with the "provide alternatives" principle in the documentation.

## 9. CI Guardrails

-   **Application:** Enforce checks for green tests, zero warnings, and the mandatory "approved-to-apply" label.
-   The practice of restricting LLM-generated content to `tools/ai/**` can also be adopted.
