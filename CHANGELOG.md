# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **New Simplified Architecture**: Three-module design (`parser`, `runtime`, `storage`)
  - `parser::parse(markdown: &str) -> Result<Ast>` - Parse Markdown DSL to AST
  - `runtime::step(state: State, ast: &Ast, event: Option<Event>) -> (State, Output)` - Stateless execution engine
  - `runtime::step_with_debug(state, ast, event, debug_config)` - Execution with debug logging
  - `storage::{save(state: &State) -> Vec<u8>, load(bytes: &[u8]) -> Result<State>}` - Save/load functionality
- **Core Types Module**: Consolidated types in `types/` module
  - `Ast` - Abstract syntax tree representation
  - `State` - Runtime state with program counter and variables
  - `Event` - External events (user choices, etc.)
  - `Output` - Execution output with lines, choices, and effects
  - `Directive` - Simplified directive type for Issue #9 implementation
- **Debug Logging System** (Issue #3):
  - `DebugConfig` - Configurable debug logging
  - `LogLevel` - Trace, Debug, Info, Warn, Error levels
  - `DebugCategory` - Engine, Variables, Resources, Performance, Flow categories
  - Environment variable support (`TSUMUGAI_DEBUG`)
  - Branch, jump, and variable state tracking
- **Lint System** (Issue #2):
  - `lint::lint(ast: &Ast) -> LintResult` - Static analysis of scenarios
  - `lint::lint_with_config(ast, config)` - Configurable linting
  - **Syntax checks**: Command structure validation
  - **Reference checks**: Undefined labels, undeclared conditions detection
  - **Quality checks**: Consecutive WAITs, duplicate BGM, text length limits
  - **Flow analysis**: Unreachable code detection, infinite loop detection
  - Configurable via `LintConfig`
- **Static Validation** (Issue #9):
  - `parser::check::check(ast: &Ast) -> CheckResult` - Dry-run validation
  - Condition declaration and usage validation (:::conditions blocks)
  - Undeclared condition warnings
  - Unused condition warnings
- **Comprehensive Test Suite**:
  - Unit tests for parser, runtime, storage, lint, and debug modules
  - Integration tests for complete scenario flows
  - Golden snapshot tests for output consistency
  - Debug logging tests (jumps, branches, variables)
  - Flow analysis tests (unreachable code, infinite loops)
- **JSON Schemas**: Added schema definitions for `Output` and `State` in `schemas/`
- **Feature Flag**: `facade` feature for backward compatibility (will be removed in next major version)

### Changed
- **Architecture Migration**: Moved from DDD/Clean Architecture to simplified 3-module design
- **API Simplification**: New functional API is now the recommended approach
- **Documentation**: Updated README.md and CLAUDE.md to reflect new architecture

### Deprecated
- **Legacy Facade API**: `facade` and `legacy_adapter` modules are now behind `facade` feature flag
  - These will be removed in version 1.0.0
  - Users should migrate to the new simplified API (`parser`, `runtime`, `storage`)

### Migration Guide

#### From Legacy API to New API

**Before (Legacy)**:
```rust
use tsumugai::facade::SimpleEngine;

let mut engine = SimpleEngine::new(seed);
engine.load_scenario_from_markdown(markdown)?;
let output = engine.step(None)?;
```

**After (New API)**:
```rust
use tsumugai::{parser, runtime, types::State};

let ast = parser::parse(markdown)?;
let state = State::with_seed(seed);
let (new_state, output) = runtime::step(state, &ast, None);
```

**Benefits**:
- Simpler, more predictable API
- Better testability
- Stateless execution
- Easier to integrate with custom state management

## [0.1.0] - 2024-XX-XX

### Added
- Initial release with DDD/Clean Architecture
- Markdown scenario parser
- Story execution engine
- Application layer with high-level Engine API
- Domain layer with entities and services
- Infrastructure layer with parsing and persistence

---

## Versioning Policy

- **MAJOR** version: Breaking changes, API incompatibility
- **MINOR** version: New features, backward compatible
- **PATCH** version: Bug fixes, backward compatible

## Support Policy

- Current major version: Full support
- Previous major version: Security fixes only for 6 months
- Older versions: No support

## Future Roadmap

### Version 1.0.0 (Planned)
- Remove `facade` feature and legacy modules
- Stabilize new API
- Complete API documentation
- Performance optimizations

### Version 0.2.0 (Next)
- Enhanced error messages with better context
- Additional scenario validation rules
- Performance improvements for large scenarios
- Extended documentation and examples
