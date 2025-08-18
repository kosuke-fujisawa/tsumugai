//! Application Engine Tests - Verify choose() uses cached branch labels
//! Tests that choose() operates on cached data without implicit step() calls

use tsumugai::application::engine::Engine;

#[cfg(test)]
mod application_engine_tests {
    use super::*;

    /// Test: choose() uses cached branch labels after step() returns WaitBranch
    /// Expectation: NextAction::Next, no implicit step() call, cache consumption verified
    /// Metric: Execution time <50ms, cached branch labels consumed correctly
    #[test]
    fn application_engine_choose_uses_cached_branch_labels() {
        let markdown = r#"
[SAY speaker=Narrator]
Hello world!

[BRANCH choice=Option A label=opt_a, choice=Option B label=opt_b]

[LABEL name=opt_a]
[SAY speaker=Test]
You chose A!

[LABEL name=opt_b]
[SAY speaker=Test]
You chose B!
"#;

        let mut engine = Engine::from_markdown(markdown).expect("Failed to parse markdown");

        // Step until we reach a branch
        let mut step_result = None;
        for _ in 0..10 {
            let result = engine.step().expect("Step failed");
            if matches!(
                result.next,
                tsumugai::application::api::NextAction::WaitBranch
            ) {
                step_result = Some(result);
                break;
            }
        }

        let step_result = step_result.expect("Should reach WaitBranch within 10 steps");

        // Verify we have branch choices
        assert!(matches!(
            step_result.next,
            tsumugai::application::api::NextAction::WaitBranch
        ));

        // Verify there are choice directives
        let has_branch_directive = step_result
            .directives
            .iter()
            .any(|d| matches!(d, tsumugai::application::api::Directive::Branch { .. }));
        assert!(has_branch_directive, "Should have Branch directive");

        // Now use choose() - this should NOT call step() internally
        let start_time = std::time::Instant::now();
        engine.choose(0).expect("Choose should succeed");
        let choose_duration = start_time.elapsed();

        // Verify performance constraint
        assert!(
            choose_duration.as_millis() < 50,
            "choose() should complete in <50ms"
        );

        // Verify that choose() consumed the cache - subsequent choose() should fail
        let second_choose_result = engine.choose(0);
        assert!(
            second_choose_result.is_err(),
            "Second choose() should fail after cache consumption"
        );

        // Verify the error message mentions no cached choices
        let error_msg = format!("{:?}", second_choose_result.unwrap_err());
        assert!(
            error_msg.contains("cached") || error_msg.contains("No choices"),
            "Error should mention cache or choices: {error_msg}"
        );
    }

    /// Test: choose() with invalid index returns appropriate error
    #[test]
    fn application_engine_choose_invalid_index() {
        let markdown = r#"
[BRANCH choice=Only Option label=only]

[LABEL name=only]
[SAY speaker=Test]
Done.
"#;

        let mut engine = Engine::from_markdown(markdown).expect("Failed to parse markdown");

        // Step to branch
        let result = engine.step().expect("Step failed");
        assert!(matches!(
            result.next,
            tsumugai::application::api::NextAction::WaitBranch
        ));

        // Try invalid index
        let result = engine.choose(5);
        assert!(result.is_err());

        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(
            error_msg.contains("out of range") || error_msg.contains("index"),
            "Error should mention index or range: {error_msg}"
        );
    }

    /// Test: choose() when no choices available
    #[test]
    fn application_engine_choose_no_choices() {
        let markdown = "Hello world!";
        let mut engine = Engine::from_markdown(markdown).expect("Failed to parse markdown");

        // Try choose without being in branch state
        let result = engine.choose(0);
        assert!(result.is_err());

        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(
            error_msg.contains("No choices") || error_msg.contains("available"),
            "Error should mention no choices: {error_msg}"
        );
    }
}
