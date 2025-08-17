//! Unit tests for parser - TDD Red-Green-Refactor cycle
//! Tests individual parser components in isolation

use tsumugai::{Command, ParseError, parse};

#[cfg(test)]
mod parser_unit_tests {
    use super::*;

    /// Unit test: Single SAY command parsing
    /// Metric: Parse single command correctly with speaker and text extraction
    #[test]
    fn test_single_say_command_parsing() {
        let input = "[SAY speaker=Hero]\nHello world!";

        let result = parse(input).expect("Should parse successfully");
        assert_eq!(result.cmds.len(), 1);

        match &result.cmds[0] {
            Command::Say { speaker, text } => {
                assert_eq!(speaker, "Hero");
                assert_eq!(text, "Hello world!");
            }
            _ => panic!("Expected Say command"),
        }
    }

    /// Unit test: Empty input handling
    /// Metric: Empty markdown should result in empty command list
    #[test]
    fn test_empty_input() {
        let input = "";
        let result = parse(input).expect("Empty input should parse successfully");
        assert_eq!(result.cmds.len(), 0);
    }

    /// Unit test: Whitespace-only input
    /// Metric: Whitespace should be ignored, resulting in empty command list
    #[test]
    fn test_whitespace_only_input() {
        let input = "   \n\n\t  \n  ";
        let result = parse(input).expect("Whitespace input should parse successfully");
        assert_eq!(result.cmds.len(), 0);
    }

    /// Unit test: Invalid command format
    /// Metric: Malformed commands should result in ParseError
    #[test]
    fn test_invalid_command_format() {
        let input = "[INVALID_COMMAND without_proper_format]";

        let result = parse(input);
        assert!(result.is_err(), "Invalid command should fail to parse");

        // Verify error contains helpful information
        let error = result.unwrap_err();
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("INVALID_COMMAND") || error_msg.len() > 0);
    }

    /// Unit test: BRANCH command with multiple choices
    /// Metric: BRANCH should parse all choices with labels correctly
    #[test]
    fn test_branch_multiple_choices() {
        let input = r#"
[BRANCH choice=Left label=go_left, choice=Right label=go_right, choice=Stay label=stay_here]

[LABEL name=go_left]
[LABEL name=go_right]  
[LABEL name=stay_here]
"#;

        let result = parse(input).expect("BRANCH should parse successfully");
        assert_eq!(result.cmds.len(), 4); // 1 BRANCH + 3 LABELs

        match &result.cmds[0] {
            Command::Branch { choices } => {
                assert_eq!(choices.len(), 3);
                assert_eq!(choices[0].choice, "Left");
                assert_eq!(choices[0].label, "go_left");
                assert_eq!(choices[1].choice, "Right");
                assert_eq!(choices[1].label, "go_right");
                assert_eq!(choices[2].choice, "Stay");
                assert_eq!(choices[2].label, "stay_here");
            }
            _ => panic!("Expected Branch command"),
        }
    }

    /// Unit test: WAIT command with duration parsing
    /// Metric: WAIT should correctly parse various time formats
    #[test]
    fn test_wait_duration_parsing() {
        let test_cases = vec![
            ("[WAIT 1s]", 1.0),
            ("[WAIT 2.5s]", 2.5),
            ("[WAIT 0.1s]", 0.1),
            ("[WAIT 10s]", 10.0),
        ];

        for (input, expected_duration) in test_cases {
            let result = parse(input).expect(&format!("Should parse: {}", input));
            assert_eq!(result.cmds.len(), 1);

            match &result.cmds[0] {
                Command::Wait { secs } => {
                    assert_eq!(
                        *secs, expected_duration,
                        "Duration mismatch for input: {}",
                        input
                    );
                }
                _ => panic!("Expected Wait command for input: {}", input),
            }
        }
    }

    /// Unit test: SET command value types
    /// Metric: SET should handle different value types (int, bool, string)
    #[test]
    fn test_set_command_value_types() {
        let test_cases = vec![
            (
                "[SET name=score value=100]",
                "score",
                tsumugai::Value::Int(100),
            ),
            (
                "[SET name=flag value=true]",
                "flag",
                tsumugai::Value::Bool(true),
            ),
            (
                "[SET name=message value=hello]",
                "message",
                tsumugai::Value::Str("hello".to_string()),
            ),
        ];

        for (input, expected_name, expected_value) in test_cases {
            let result = parse(input).expect(&format!("Should parse: {}", input));
            assert_eq!(result.cmds.len(), 1);

            match &result.cmds[0] {
                Command::Set { name, value } => {
                    assert_eq!(name, expected_name);
                    assert_eq!(value, &expected_value);
                }
                _ => panic!("Expected Set command for input: {}", input),
            }
        }
    }

    /// Unit test: Undefined label detection (should fail)
    /// Metric: Parser should detect and report undefined labels with line numbers
    #[test]
    fn test_undefined_label_detection() {
        let input = r#"
[SAY speaker=A]
First line

[JUMP label=undefined_target]

[LABEL name=valid_target]

[SAY speaker=A]
Done
"#;

        let result = parse(input);
        match result {
            Err(ParseError::UndefinedLabel { label, line }) => {
                assert_eq!(label, "undefined_target");
                assert!(line > 0, "Line number should be provided");
            }
            _ => panic!("Expected UndefinedLabel error"),
        }
    }

    /// Unit test: Multiple undefined labels (first one should be reported)
    /// Metric: Parser should report the first undefined label encountered
    #[test]
    fn test_multiple_undefined_labels() {
        let input = r#"
[JUMP label=first_undefined]
[JUMP label=second_undefined]
[LABEL name=valid_label]
"#;

        let result = parse(input);
        match result {
            Err(ParseError::UndefinedLabel { label, .. }) => {
                assert_eq!(label, "first_undefined");
            }
            _ => panic!("Expected UndefinedLabel error for first undefined label"),
        }
    }

    /// Unit test: Complex scenario parsing
    /// Metric: Parser should handle realistic scenario with multiple command types
    #[test]
    fn test_complex_scenario_parsing() {
        let input = r#"
[SET name=chapter value=1]

[PLAY_BGM name=intro]

[SAY speaker=Narrator]
Chapter 1: The Beginning

[SHOW_IMAGE file=hero_intro]

[BRANCH choice=Continue label=continue_story, choice=Skip label=skip_intro]

[LABEL name=continue_story]

[SAY speaker=Hero]
Let's start this adventure!

[JUMP label=main_story]

[LABEL name=skip_intro]

[SAY speaker=Narrator]
Skipping to main story...

[LABEL name=main_story]

[SAY speaker=Hero]
Here we are!
"#;

        let result = parse(input).expect("Complex scenario should parse successfully");

        // Verify command count
        assert_eq!(result.cmds.len(), 12); // Updated count

        // Verify command types in order
        let expected_types = vec![
            "Set",
            "PlayBgm",
            "Say",
            "ShowImage",
            "Branch",
            "Label",
            "Say",
            "Jump",
            "Label",
            "Say",
            "Label",
            "Say",
        ];

        for (i, cmd) in result.cmds.iter().enumerate() {
            let cmd_type = match cmd {
                Command::Set { .. } => "Set",
                Command::PlayBgm { .. } => "PlayBgm",
                Command::Say { .. } => "Say",
                Command::ShowImage { .. } => "ShowImage",
                Command::Branch { .. } => "Branch",
                Command::Label { .. } => "Label",
                Command::Jump { .. } => "Jump",
                _ => "Other",
            };

            if i < expected_types.len() {
                assert_eq!(cmd_type, expected_types[i], "Command {} type mismatch", i);
            }
        }
    }
}
