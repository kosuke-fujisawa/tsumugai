//! # tsumugai
//!
//! A Rust library that parses Markdown scenarios into Command sequences and provides
//! step-by-step execution with Directive emission for visual novel-like applications.
//!
//! The library does NOT implement audio/video playback, rendering, or UI - it only
//! provides the execution logic and tells your application what to do through Directives.
//!
//! ## Example
//!
//! ```rust
//! use tsumugai::{parse, Engine, Step, WaitKind};
//!
//! let markdown = r#"
//! [SAY speaker=Ayumi]
//! Hello, world!
//!
//! [PLAY_BGM name=intro]
//! "#;
//!
//! let program = parse(markdown).unwrap();
//! let mut engine = Engine::new(program);
//!
//! loop {
//!     match engine.step() {
//!         Step::Next => continue,
//!         Step::Wait(WaitKind::User) => {
//!             // Handle user input, then continue
//!             continue;
//!         }
//!         Step::Wait(WaitKind::Branch(choices)) => {
//!             // Handle branch selection, then jump
//!             continue;
//!         }
//!         Step::Jump(label) => {
//!             engine.jump_to(&label).unwrap();
//!         }
//!         Step::Halt => break,
//!     }
//!     
//!     // Get emitted directives
//!     let directives = engine.take_emitted();
//!     for directive in directives {
//!         // Handle each directive (play audio, show text, etc.)
//!         println!("{:?}", directive);
//!     }
//! }
//! ```

pub mod engine;
pub mod ir;
pub mod parse;
pub mod resolve;

pub use engine::{Directive, Engine, ResId, Step, WaitKind};
pub use ir::*;
pub use parse::{ParseError, parse};
pub use resolve::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Directive;

    #[test]
    fn test_basic_parsing() {
        let markdown = r#"
[SAY speaker=Ayumi]
Hello, world!

[PLAY_BGM name=intro]
"#;

        let program = parse(markdown).unwrap();
        assert_eq!(program.cmds.len(), 2);

        match &program.cmds[0] {
            Command::Say { speaker, text } => {
                assert_eq!(speaker, "Ayumi");
                assert_eq!(text, "Hello, world!");
            }
            _ => panic!("Expected Say command"),
        }

        match &program.cmds[1] {
            Command::PlayBgm { name } => {
                assert_eq!(name, "intro");
            }
            _ => panic!("Expected PlayBgm command"),
        }
    }

    #[test]
    fn test_branch_parsing() {
        let markdown = r#"
[BRANCH choice=左へ label=go_left, choice=右へ label=go_right]

[LABEL name=go_left]

[LABEL name=go_right]
"#;

        let program = match parse(markdown) {
            Ok(p) => p,
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(program.cmds.len(), 3); // BRANCH + 2 LABELs

        match &program.cmds[0] {
            Command::Branch { choices } => {
                assert_eq!(choices.len(), 2);
                assert_eq!(choices[0].choice, "左へ");
                assert_eq!(choices[0].label, "go_left");
                assert_eq!(choices[1].choice, "右へ");
                assert_eq!(choices[1].label, "go_right");
            }
            _ => panic!("Expected Branch command"),
        }
    }

    #[test]
    fn test_engine_execution() {
        let markdown = r#"
[SAY speaker=Ayumi]
Hello!

[PLAY_BGM name=intro]

[WAIT 1.5s]
"#;

        let program = parse(markdown).unwrap();
        let mut engine = Engine::new(program);

        // First step: SAY command
        let step = engine.step();
        assert_eq!(step, Step::Wait(WaitKind::User));

        let directives = engine.take_emitted();
        assert_eq!(directives.len(), 1);
        match &directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "Ayumi");
                assert_eq!(text, "Hello!");
            }
            _ => panic!("Expected Say directive"),
        }

        // Second step: PLAY_BGM command
        let step = engine.step();
        assert_eq!(step, Step::Next);

        let directives = engine.take_emitted();
        assert_eq!(directives.len(), 1);
        match &directives[0] {
            Directive::PlayBgm { res } => {
                assert_eq!(res.logical, "intro");
                assert_eq!(res.resolved, None); // No resolver
            }
            _ => panic!("Expected PlayBgm directive"),
        }

        // Third step: WAIT command
        let step = engine.step();
        assert_eq!(step, Step::Wait(WaitKind::Timer(1.5)));

        let directives = engine.take_emitted();
        assert_eq!(directives.len(), 1);
        match &directives[0] {
            Directive::Wait { secs } => {
                assert_eq!(*secs, 1.5);
            }
            _ => panic!("Expected Wait directive"),
        }

        // Fourth step: End of program
        let step = engine.step();
        assert_eq!(step, Step::Halt);
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

        let program = parse(markdown).unwrap();
        let mut engine = Engine::new(program);

        // First step: JUMP command
        let step = engine.step();
        match step {
            Step::Jump(label) => {
                assert_eq!(label, "target");
                let directives = engine.take_emitted(); // Take jump directive
                assert_eq!(directives.len(), 1);
                engine.jump_to(&label).unwrap();
            }
            _ => panic!("Expected Jump step"),
        }

        // Should now be at the LABEL
        let step = engine.step();
        assert_eq!(step, Step::Next);
        let directives = engine.take_emitted(); // Take label directive
        assert_eq!(directives.len(), 1);

        // Next should be the SAY after the label
        let step = engine.step();
        assert_eq!(step, Step::Wait(WaitKind::User));

        let directives = engine.take_emitted(); // Take say directive
        assert_eq!(directives.len(), 1);
        match &directives[0] {
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

        let program = parse(markdown).unwrap();
        let mut engine = Engine::new(program);

        // SET command
        let step = engine.step();
        assert_eq!(step, Step::Next);

        // MODIFY command
        let step = engine.step();
        assert_eq!(step, Step::Next);

        // Check variable value
        assert_eq!(engine.vars().get("score"), Some(&Value::Int(15)));

        // JUMP_IF command should jump
        let step = engine.step();
        match step {
            Step::Jump(label) => {
                assert_eq!(label, "success");
                engine.take_emitted(); // Clear any emitted directives
                engine.jump_to(&label).unwrap();
            }
            _ => panic!("Expected Jump step"),
        }

        // Should now be at the LABEL
        let step = engine.step();
        assert_eq!(step, Step::Next);
        engine.take_emitted(); // Clear label directive

        // Next should be the success SAY
        let step = engine.step();
        assert_eq!(step, Step::Wait(WaitKind::User));

        let directives = engine.take_emitted();
        assert_eq!(directives.len(), 1);
        match &directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "System");
                assert_eq!(text, "Score is correct!");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    #[test]
    fn test_save_restore() {
        let markdown = r#"
[SET name=score value=10]

[SAY speaker=Ayumi]
Current score: 10
"#;

        let program = parse(markdown).unwrap();
        let mut engine = Engine::new(program);

        // Execute SET command
        engine.step();

        // Take a snapshot
        let save_data = engine.snapshot();
        assert_eq!(save_data.pc, 1);
        assert_eq!(save_data.vars.get("score"), Some(&Value::Int(10)));

        // Continue execution
        engine.step();
        assert_eq!(engine.snapshot().pc, 2);

        // Restore from snapshot
        engine.restore(save_data).unwrap();
        assert_eq!(engine.snapshot().pc, 1);
        assert_eq!(engine.vars().get("score"), Some(&Value::Int(10)));
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

        let program = parse(markdown).unwrap();
        let mut engine = Engine::with_resolver(program, Box::new(TestResolver));

        // First BGM should resolve
        engine.step();
        let directives = engine.take_emitted();
        match &directives[0] {
            Directive::PlayBgm { res } => {
                assert_eq!(res.logical, "intro");
                assert_eq!(res.resolved, Some(PathBuf::from("/test/intro.mp3")));
            }
            _ => panic!("Expected PlayBgm directive"),
        }

        // Second BGM should not resolve
        engine.step();
        let directives = engine.take_emitted();
        match &directives[0] {
            Directive::PlayBgm { res } => {
                assert_eq!(res.logical, "missing");
                assert_eq!(res.resolved, None);
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
        let program = parse(md).unwrap();
        let mut eng = Engine::new(program);

        // 1回目：SAY で待ち
        assert!(matches!(eng.step(), Step::Wait(WaitKind::User)));
        eng.take_emitted();

        // 2回目：BRANCHをEmitして待ち（Enterでは進まない）
        assert!(matches!(eng.step(), Step::Wait(WaitKind::Branch(_))));
        let ds = eng.take_emitted();
        assert!(matches!(ds.last(), Some(Directive::Branch { .. })));

        // Enterではなく jump_to でのみ進む
        // eng.step() をもう一度呼んでも Wait(Branch) のまま、かつ Branch を再Emitしない想定
        assert!(matches!(eng.step(), Step::Wait(WaitKind::Branch(_))));
        let again = eng.take_emitted();
        assert!(again.is_empty(), "should not re-emit branch");

        // 選択決定
        eng.jump_to("go").unwrap();
        assert!(matches!(eng.step(), Step::Next)); // LABELに到達
        eng.take_emitted(); // LABEL directive を消費
        assert!(matches!(eng.step(), Step::Wait(WaitKind::User))); // go の SAY に到達
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
        let p = parse(md).unwrap();
        let mut eng = Engine::new(p);

        // 1: 分岐に到達 → Wait(Branch)
        match eng.step() {
            Step::Wait(WaitKind::Branch(choices)) => {
                assert_eq!(choices.len(), 2);
                eng.jump_to(&choices[0].label).unwrap();
            }
            other => panic!("expected Branch wait, got {other:?}"),
        }

        // 2: 分岐先の LABEL に到達 → Next
        match eng.step() {
            Step::Next => { /* OK */ }
            other => panic!("expected Next, got {other:?}"),
        }
        eng.take_emitted(); // Clear label directive

        // 3: 分岐先の SAY に到達 → Wait(User)
        match eng.step() {
            Step::Wait(WaitKind::User) => { /* OK */ }
            other => panic!("expected User wait, got {other:?}"),
        }
    }

    /// Unit test: Parse label validation
    /// Verifies that undefined labels result in ParseError with line numbers included.
    /// Metric: Error message must contain "line N" for proper debugging.
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

        match parse(markdown_with_undefined_label) {
            Err(crate::ParseError::UndefinedLabel { label, line }) => {
                assert_eq!(label, "undefined_label");
                assert_eq!(line, 2); // JUMP is command 1 (0-indexed + 1)

                // Verify error message contains line information
                let error_msg = format!(
                    "{}",
                    crate::ParseError::UndefinedLabel {
                        label: label.clone(),
                        line
                    }
                );
                assert!(
                    error_msg.contains(&format!("line {}", line)),
                    "Error message should contain 'line {}', got: {}",
                    line,
                    error_msg
                );
            }
            Err(other_error) => {
                panic!("Expected UndefinedLabel error, got: {:?}", other_error);
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

        match parse(markdown_with_undefined_branch_label) {
            Err(crate::ParseError::UndefinedLabel { label, line }) => {
                assert_eq!(label, "undefined_target");
                assert_eq!(line, 2); // BRANCH is command 1 (0-indexed + 1)

                // Verify error message format
                let error_msg = format!(
                    "{}",
                    crate::ParseError::UndefinedLabel {
                        label: label.clone(),
                        line
                    }
                );
                assert!(
                    error_msg.contains("line"),
                    "Error message should contain 'line', got: {}",
                    error_msg
                );
                assert!(
                    error_msg.contains("undefined_target"),
                    "Error message should contain label name, got: {}",
                    error_msg
                );
            }
            Err(other_error) => {
                panic!("Expected UndefinedLabel error, got: {:?}", other_error);
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
        let result = parse(markdown_with_all_valid_labels);
        assert!(
            result.is_ok(),
            "All labels are defined, parsing should succeed: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.cmds.len(), 6); // SAY, JUMP, LABEL, BRANCH, LABEL, SAY
    }
}
