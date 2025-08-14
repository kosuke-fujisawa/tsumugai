This file defines the development rules and guardrails for Claude Code when contributing to the tsumugai project.

 Platform

Rust (library crate)

Tauri (backend IPC and platform layer)

Svelte (frontend UI)

 Principles

Always operate on code, not UI editors.

Maintain a clean Rust API, modular structure, and clear src/lib.rs entry.

Separate UI logic (Svelte) from domain logic (Rust).

Do not touch frontend code unless explicitly requested.

Ensure all Rust code compiles and passes cargo test.

 Tasks Claude is Allowed to Perform

Add or modify .rs modules under src/

Implement Markdown command parsers and command emitters

Write integration tests for command parsing

Maintain crate-level documentation and examples

Add utilities for IPC communication with Tauri frontend

❌ Tasks Claude Must Not Perform

Modify src-tauri/tauri.conf.json or any Svelte files

Introduce dependencies without asking

Create binaries; this is a library crate

Touch assets or runtime UI rendering logic

✅ Example Flow

When asked to implement support for a new command:

Create a new variant in the Command enum.

Extend the Markdown parser to detect and emit this variant.

Add tests for round-trip parsing.

Document it in README.md and claude.md.

 Testing

All code must include tests:

cargo test

Use #[cfg(test)] blocks and validate parsers, command structures, and context handlers.

 File Structure

src/
├── lib.rs          # public API
├── parser.rs       # Markdown -> Commands
├── command.rs      # Command enum and helpers
├── engine.rs       # Trait defining how commands are executed
├── context.rs      # Variable and state management
└── test_data/      # Sample scripts for testing

Claude Code should always clarify before acting when ambiguity exists, and document the location, role, and impact of each generated artifact.