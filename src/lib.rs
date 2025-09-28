//! # tsumugai
//!
//! A Rust library that parses Markdown scenarios into story commands and provides
//! step-by-step execution for visual novel-like applications using Clean Architecture principles.
//!
//! The library provides both high-level convenience APIs and low-level domain access
//! for maximum flexibility.
//!
//! ## Quick Start
//!
//! ```rust
//! use tsumugai::application::{engine::Engine, api::{NextAction, Directive}};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Simple usage - load and execute a scenario
//! let markdown = r#"
//! [SAY speaker=Hero]
//! Hello, world!
//! "#;
//! let mut engine = Engine::from_markdown(markdown)?;
//!
//! loop {
//!     let result = engine.step()?;
//!     
//!     // Handle directives
//!     for directive in &result.directives {
//!         match directive {
//!             Directive::Say { speaker, text } => {
//!                 println!("{}: {}", speaker, text);
//!             }
//!             _ => println!("Other directive: {:?}", directive),
//!         }
//!     }
//!     
//!     // Handle next action
//!     match result.next {
//!         NextAction::Next => continue,
//!         NextAction::WaitUser => {
//!             // Wait for user input, then continue
//!             break; // For demo
//!         }
//!         NextAction::WaitBranch => {
//!             // Present choices to user, then call engine.choose(index)
//!             engine.choose(0)?;
//!         }
//!         NextAction::Halt => break,
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Advanced Usage with Domain Layer
//!
//! ```rust
//! use tsumugai::infrastructure::parsing::{MarkdownScenarioParser, ScenarioParser};
//! use tsumugai::domain::{entities::StoryExecution, services::StoryExecutionService};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Use domain services directly for more control
//! let parser = MarkdownScenarioParser::with_default_id_generator();
//! let scenario = parser.parse("[SAY speaker=Hero]\nHello!").await?;
//!
//! let mut execution = StoryExecution::new(scenario)?;
//! let execution_service = StoryExecutionService::new();
//!
//! let result = execution_service.execute_next_command(&mut execution)?;
//! println!("Execution result: {:?}", result);
//! # Ok(())
//! # }
//! ```

pub mod application;
pub mod contracts;
pub mod domain;
pub mod engine;
pub mod infrastructure;
pub mod ir;
pub mod legacy_adapter;
pub mod parse;
pub mod resolve;

// New simplified architecture modules
pub mod types;
pub mod parser;
pub mod runtime;
pub mod storage;
pub mod facade;

// Stable public contracts - the main API for library users
pub use application::api::{ApiError, Directive, NextAction, StepResult};
pub use application::engine::Engine;

// New simplified API exports
pub use parser::parse as parse_scenario;
pub use runtime::step;
pub use storage::{save, load};
pub use types::{Ast, State, Event, Output};
pub use facade::SimpleEngine;

// Legacy contracts for backward compatibility
pub use contracts::{StepDirectives, StoryEngineError};

// High-level API exports
pub use story_engine::StoryEngine;

// Domain layer exports for advanced usage
// Note: Users can access domain and infrastructure modules directly

// Legacy API exports for backward compatibility
#[deprecated(since = "0.2.0", note = "Use `application::api::Directive` instead")]
pub use engine::{Directive as LegacyDirective, ResId};
#[deprecated(since = "0.2.0", note = "Use `application::engine::Engine` instead")]
pub use engine::{Engine as LegacyEngine, Step, WaitKind};
pub use ir::*;
#[deprecated(
    since = "0.2.0",
    note = "Use `application::engine::Engine::from_markdown()` instead"
)]
pub use parse::{ParseError, parse};
pub use resolve::*;

// High-level story engine implementation
mod story_engine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing_and_execution() {
        let markdown = r#"
[SAY speaker=Ayumi]
Hello, world!

[PLAY_BGM name=intro]
"#;

        let mut engine = Engine::from_markdown(markdown).unwrap();

        // First step: SAY command
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        assert_eq!(step_result.directives.len(), 1);

        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "Ayumi");
                assert_eq!(text, "Hello, world!");
            }
            _ => panic!("Expected Say directive"),
        }

        // Second step: PLAY_BGM command
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        assert_eq!(step_result.directives.len(), 1);

        match &step_result.directives[0] {
            Directive::PlayBgm { path } => {
                assert_eq!(path, &None); // No resolver
            }
            _ => panic!("Expected PlayBgm directive"),
        }
    }

    #[test]
    fn test_branch_execution() {
        let markdown = r#"
[BRANCH choice=左へ label=go_left, choice=右へ label=go_right]

[LABEL name=go_left]
[SAY speaker=Guide]
You went left.

[LABEL name=go_right]
[SAY speaker=Guide] 
You went right.
"#;

        let mut engine = Engine::from_markdown(markdown).unwrap();

        // First step: BRANCH command
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitBranch);
        assert_eq!(step_result.directives.len(), 1);

        match &step_result.directives[0] {
            Directive::Branch { choices } => {
                assert_eq!(choices.len(), 2);
                assert_eq!(choices[0], "左へ");
                assert_eq!(choices[1], "右へ");
            }
            _ => panic!("Expected Branch directive"),
        }

        // Choose first option (left)
        engine.choose(0).unwrap();

        // Should now be at left label
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // Next should be the SAY after left label
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "Guide");
                assert_eq!(text, "You went left.");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    #[test]
    fn test_wait_execution() {
        let markdown = r#"
[SAY speaker=Ayumi]
Hello!

[WAIT 1.5s]
"#;

        let mut engine = Engine::from_markdown(markdown).unwrap();

        // First step: SAY command
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "Ayumi");
                assert_eq!(text, "Hello!");
            }
            _ => panic!("Expected Say directive"),
        }

        // Second step: WAIT command
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        match &step_result.directives[0] {
            Directive::Wait { seconds } => {
                assert_eq!(*seconds, 1.5);
            }
            _ => panic!("Expected Wait directive"),
        }

        // Third step: End of program
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Halt);
    }

    #[test]
    fn test_jump_execution() {
        let markdown = r#"
[JUMP label=target]

[SAY speaker=Ayumi]
This should be skipped.

[LABEL name=target]

[SAY speaker=Ayumi]
Target reached!
"#;

        let mut engine = Engine::from_markdown(markdown).unwrap();

        // First step: JUMP command
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // Next step: should land at the LABEL
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // Next should be the SAY after the label
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "Ayumi");
                assert_eq!(text, "Target reached!");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    #[test]
    fn test_variables() {
        let markdown = r#"
[SET name=score value=10]

[MODIFY name=score op=add value=5]

[JUMP_IF var=score cmp=eq value=15 label=success]

[SAY speaker=System]
This should be skipped.

[LABEL name=success]

[SAY speaker=System]
Score is correct!
"#;

        let mut engine = Engine::from_markdown(markdown).unwrap();

        // SET command
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // MODIFY command
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // Check variable value
        assert_eq!(engine.get_var("score"), Some("15".to_string()));

        // JUMP_IF command should execute
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // Should now be at the LABEL
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // Next should be the success SAY
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "System");
                assert_eq!(text, "Score is correct!");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    #[test]
    fn test_variable_persistence() {
        let markdown = r#"
[SET name=score value=10]

[SAY speaker=Ayumi]
Current score: 10
"#;

        let mut engine = Engine::from_markdown(markdown).unwrap();

        // Execute SET command
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // Check variable was set
        assert_eq!(engine.get_var("score"), Some("10".to_string()));

        // Continue execution
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);

        // Variable should still be accessible
        assert_eq!(engine.get_var("score"), Some("10".to_string()));
    }

    #[test]
    fn test_resolver() {
        use std::path::PathBuf;

        struct TestResolver;
        impl Resolver for TestResolver {
            fn resolve_bgm(&self, logical: &str) -> Option<PathBuf> {
                if logical == "intro" {
                    Some(PathBuf::from("/test/intro.mp3"))
                } else {
                    None
                }
            }
        }

        let markdown = r#"
[PLAY_BGM name=intro]
[PLAY_BGM name=missing]
"#;

        let mut engine =
            Engine::from_markdown_with_resolver(markdown, Box::new(TestResolver)).unwrap();

        // First BGM should resolve
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        match &step_result.directives[0] {
            Directive::PlayBgm { path } => {
                assert_eq!(path, &Some("/test/intro.mp3".to_string()));
            }
            _ => panic!("Expected PlayBgm directive"),
        }

        // Second BGM should not resolve
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        match &step_result.directives[0] {
            Directive::PlayBgm { path } => {
                assert_eq!(path, &None);
            }
            _ => panic!("Expected PlayBgm directive"),
        }
    }

    #[test]
    fn branch_stops_until_jump() {
        let md = r#"
[SAY speaker=A]
x

[BRANCH choice=Go label=go, choice=Stop label=stop]

[LABEL name=go]

[SAY speaker=A]
go

[LABEL name=stop]

[SAY speaker=A]
stop
"#;
        let mut eng = Engine::from_markdown(md).unwrap();

        // 1回目：SAY で待ち
        let step_result = eng.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);

        // 2回目：BRANCHで待ち
        let step_result = eng.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitBranch);
        match &step_result.directives[0] {
            Directive::Branch { choices } => {
                assert_eq!(choices.len(), 2);
            }
            _ => panic!("Expected Branch directive"),
        }

        // 選択決定
        eng.choose(0).unwrap(); // Choose "Go"

        // LABELに到達
        let step_result = eng.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // go の SAY に到達
        let step_result = eng.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "A");
                assert_eq!(text, "go");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    #[test]
    fn branch_requires_choice_then_say_waits_enter() {
        let md = r#"
[BRANCH choice=右へ label=go_right, choice=左へ label=go_left]
# scene: go_right
[LABEL name=go_right]
[SAY speaker=A] R
# scene: go_left
[LABEL name=go_left]
[SAY speaker=A] L
"#;
        let mut eng = Engine::from_markdown(md).unwrap();

        // 1: 分岐に到達 → WaitBranch
        let step_result = eng.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitBranch);
        match &step_result.directives[0] {
            Directive::Branch { choices } => {
                assert_eq!(choices.len(), 2);
            }
            _ => panic!("Expected Branch directive"),
        }

        // Choose first option
        eng.choose(0).unwrap();

        // 2: 分岐先の LABEL に到達 → Next
        let step_result = eng.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);

        // 3: 分岐先の SAY に到達 → WaitUser
        let step_result = eng.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "A");
                assert_eq!(text, "R");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    /// Unit test: Parse label validation
    /// Verifies that undefined labels result in ApiError with line numbers included.
    /// Metric: Error message must contain line information for proper debugging.
    #[test]
    fn parse_label_validation() {
        let markdown_with_undefined_label = r#"
[SAY speaker=A]
Hello

[JUMP label=undefined_label]

[LABEL name=valid_label]

[SAY speaker=A]
Done
"#;

        match Engine::from_markdown(markdown_with_undefined_label) {
            Err(ApiError::Parse {
                line,
                column: _,
                message,
            }) => {
                assert!(message.contains("undefined_label"));
                assert!(message.contains("Undefined label"));
                assert!(line > 0, "Line number should be provided");
            }
            Err(other_error) => {
                panic!("Expected Parse error, got: {other_error:?}");
            }
            Ok(_) => {
                panic!("Expected parse error for undefined label, but parsing succeeded");
            }
        }
    }

    #[test]
    fn parse_label_validation_branch() {
        let markdown_with_undefined_branch_label = r#"
[SAY speaker=A]
Hello

[BRANCH choice=Go label=undefined_target, choice=Stay label=valid_target]

[LABEL name=valid_target]

[SAY speaker=A]
Done
"#;

        match Engine::from_markdown(markdown_with_undefined_branch_label) {
            Err(ApiError::Parse {
                line,
                column: _,
                message,
            }) => {
                assert!(message.contains("undefined_target"));
                assert!(message.contains("Undefined label"));
                assert!(line > 0, "Line number should be provided");
            }
            Err(other_error) => {
                panic!("Expected Parse error, got: {other_error:?}");
            }
            Ok(_) => {
                panic!("Expected parse error for undefined branch label, but parsing succeeded");
            }
        }
    }

    #[test]
    fn parse_label_validation_all_valid() {
        let markdown_with_all_valid_labels = r#"
[SAY speaker=A]
Hello

[JUMP label=target]

[LABEL name=target]

[BRANCH choice=Go label=end, choice=Loop label=target]

[LABEL name=end]

[SAY speaker=A]
Done
"#;

        // This should parse successfully
        let result = Engine::from_markdown(markdown_with_all_valid_labels);
        assert!(
            result.is_ok(),
            "All labels are defined, parsing should succeed: {:?}",
            result.err()
        );
    }
}
