//! Unit tests for Engine - Individual step execution behavior
//! Tests the "1行の解釈→StepResult" behavior in isolation

use tsumugai::{Engine, NextAction, Directive};

// Note: This test file contains both new API tests and legacy tests
// that demonstrate the old Program-based API is still functional.
// For comprehensive coverage, both approaches are maintained.

#[cfg(test)]
mod engine_unit_tests {
    use super::*;

    /// Unit test: Single SAY command execution
    /// Metric: SAY should emit directive and wait for user
    #[test]
    fn test_single_say_execution() {
        let markdown = r#"
[SAY speaker=Test]
Hello
"#;
        
        let mut engine = Engine::from_markdown(markdown).unwrap();
        
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        assert_eq!(step_result.directives.len(), 1);
        
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "Test");
                assert_eq!(text, "Hello");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    /// Unit test: WAIT command execution
    /// Metric: WAIT should emit directive with correct duration and wait for user
    #[test]
    fn test_wait_execution() {
        let markdown = r#"
[WAIT 2.5s]
"#;
        
        let mut engine = Engine::from_markdown(markdown).unwrap();
        
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        assert_eq!(step_result.directives.len(), 1);
        
        match &step_result.directives[0] {
            Directive::Wait { seconds } => {
                assert_eq!(*seconds, 2.5);
            }
            _ => panic!("Expected Wait directive"),
        }
    }

    /// Unit test: LABEL command execution
    /// Metric: LABEL should continue execution without waiting
    #[test]
    fn test_label_execution() {
        let markdown = r#"
[LABEL name=test_label]
"#;
        
        let mut engine = Engine::from_markdown(markdown).unwrap();
        
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        // Labels emit JumpTo directives in the new API
        assert_eq!(step_result.directives.len(), 1);
        
        match &step_result.directives[0] {
            Directive::JumpTo { label } => {
                assert_eq!(label, "test_label");
            }
            _ => panic!("Expected JumpTo directive"),
        }
    }

    /// Unit test: JUMP command execution
    /// Metric: JUMP should automatically navigate to target label
    #[test]
    fn test_jump_execution() {
        let markdown = r#"
[JUMP label=target]

[SAY speaker=NotReached]
This should be skipped.

[LABEL name=target]

[SAY speaker=Test]
Target reached!
"#;
        
        let mut engine = Engine::from_markdown(markdown).unwrap();
        
        // First step should skip to the target and execute the SAY
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next); // LABEL
        
        // Next step should be the SAY after the label
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "Test");
                assert_eq!(text, "Target reached!");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    /// Unit test: BRANCH command execution  
    /// Metric: BRANCH should emit directive and wait for branch choice
    #[test]
    fn test_branch_execution() {
        let markdown = r#"
[BRANCH choice=A label=choice_a, choice=B label=choice_b]

[LABEL name=choice_a]
[SAY speaker=Test]
Chose A

[LABEL name=choice_b]
[SAY speaker=Test]
Chose B
"#;
        
        let mut engine = Engine::from_markdown(markdown).unwrap();
        
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitBranch);
        assert_eq!(step_result.directives.len(), 1);
        
        match &step_result.directives[0] {
            Directive::Branch { choices } => {
                assert_eq!(choices.len(), 2);
                assert_eq!(choices[0], "A");
                assert_eq!(choices[1], "B");
            }
            _ => panic!("Expected Branch directive"),
        }
        
        // Test choosing first option
        engine.choose(0).unwrap();
        
        // Should now be at choice_a label
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next); // LABEL
        
        // Next should be the SAY after label
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "Test");
                assert_eq!(text, "Chose A");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    /// Unit test: SET command execution
    /// Metric: SET should update variables and continue execution
    #[test]
    fn test_set_execution() {
        let markdown = r#"
[SET name=score value=100]
"#;
        
        let mut engine = Engine::from_markdown(markdown).unwrap();
        
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        
        // Verify variable was set
        assert_eq!(engine.get_var("score"), Some("100".to_string()));
    }

    /// Unit test: MODIFY command execution
    /// Metric: MODIFY should update existing variables correctly
    #[test]  
    fn test_modify_execution() {
        let markdown = r#"
[SET name=score value=50]
[MODIFY name=score op=add value=25]
"#;
        
        let mut engine = Engine::from_markdown(markdown).unwrap();
        
        // Execute SET
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        assert_eq!(engine.get_var("score"), Some("50".to_string()));
        
        // Execute MODIFY
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        assert_eq!(engine.get_var("score"), Some("75".to_string()));
        
    }

    /// Unit test: JUMP_IF condition evaluation
    /// Metric: JUMP_IF should correctly evaluate conditions and jump
    #[test]
    fn test_jump_if_execution() {
        let markdown = r#"
[SET name=score value=100]
[JUMP_IF var=score cmp=eq value=100 label=success]

[SAY speaker=Narrator]
This should be skipped

[LABEL name=success]

[SAY speaker=Test]
Success!
"#;
        
        let mut engine = Engine::from_markdown(markdown).unwrap();
        
        // Execute SET
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        
        // Execute JUMP_IF - should automatically jump to success label
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next); // at LABEL
        
        // Next should be the SAY after success label
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::WaitUser);
        
        match &step_result.directives[0] {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, "Test");
                assert_eq!(text, "Success!");
            }
            _ => panic!("Expected Say directive"),
        }
    }

    /// Unit test: Sequential execution
    /// Metric: Engine should execute commands sequentially and halt at end
    #[test]
    fn test_sequential_execution() {
        let markdown = r#"
[SET name=var1 value=1]
[SET name=var2 value=2]
[SET name=var3 value=3]
"#;
        
        let mut engine = Engine::from_markdown(markdown).unwrap();
        
        // Execute all SET commands
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Next);
        
        // Should halt at end
        let step_result = engine.step().unwrap();
        assert_eq!(step_result.next, NextAction::Halt);
        
        // Verify all variables were set
        assert_eq!(engine.get_var("var1"), Some("1".to_string()));
        assert_eq!(engine.get_var("var2"), Some("2".to_string()));
        assert_eq!(engine.get_var("var3"), Some("3".to_string()));
    }

    // Note: Low-level jump functionality is tested through JUMP commands
    // in the integration tests. The new API doesn't expose internal jump_to methods.

    // Note: Branch re-emission behavior is handled automatically by the new API.
    // The behavior is tested in the main library tests.
}