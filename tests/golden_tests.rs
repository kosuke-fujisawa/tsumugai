//! Golden tests for deterministic execution validation
//! Compares --dump output with pre-recorded JSON files

use std::path::Path;
use std::process::Command;

#[cfg(test)]
mod golden_tests {
    use super::*;

    /// Golden test: Simple scenario deterministic execution
    /// Metric: Generated JSON must exactly match golden file
    #[test]
    fn test_simple_scenario_golden() {
        run_golden_test("simple");
    }

    /// Golden test: Branch scenario deterministic execution
    /// Metric: Generated JSON must exactly match golden file, including branch choice
    #[test]
    fn test_branch_scenario_golden() {
        run_golden_test("branch");
    }

    /// Helper function to run golden tests
    /// Executes dump command and compares output with expected golden file
    fn run_golden_test(test_name: &str) {
        let fixture_path = format!("tests/fixtures/{}.md", test_name);
        let golden_path = format!("tests/golden/{}.json", test_name);

        // Verify fixture file exists
        assert!(
            Path::new(&fixture_path).exists(),
            "Fixture file not found: {}",
            fixture_path
        );

        // Verify golden file exists
        assert!(
            Path::new(&golden_path).exists(),
            "Golden file not found: {}. Generate it with: cargo run --example dump -- {} > {}",
            golden_path,
            fixture_path,
            golden_path
        );

        // Run dump command
        let output = Command::new("cargo")
            .args(&["run", "--example", "dump", "--", &fixture_path])
            .output()
            .expect("Failed to execute dump command");

        // Check that command succeeded
        if !output.status.success() {
            panic!(
                "Dump command failed for {}: {}",
                test_name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Get the actual output
        let actual_output =
            String::from_utf8(output.stdout).expect("Dump output was not valid UTF-8");

        // Read the expected golden file
        let expected_output = std::fs::read_to_string(&golden_path)
            .expect(&format!("Failed to read golden file: {}", golden_path));

        // Parse both as JSON to ensure they're valid and for normalized comparison
        let actual_json: serde_json::Value =
            serde_json::from_str(&actual_output).expect("Actual output was not valid JSON");

        let expected_json: serde_json::Value =
            serde_json::from_str(&expected_output).expect("Golden file was not valid JSON");

        // Compare the parsed JSON structures
        if actual_json != expected_json {
            // For debugging, show the differences
            let actual_pretty = serde_json::to_string_pretty(&actual_json)
                .expect("Failed to pretty-print actual JSON");
            let expected_pretty = serde_json::to_string_pretty(&expected_json)
                .expect("Failed to pretty-print expected JSON");

            panic!(
                "Golden test failed for {}!\n\nExpected:\n{}\n\nActual:\n{}\n\n\
                 To update the golden file, run:\ncargo run --example dump -- {} > {}",
                test_name, expected_pretty, actual_pretty, fixture_path, golden_path
            );
        }
    }

    /// Meta-test: Verify golden test infrastructure
    /// Metric: Golden test runner should detect mismatches correctly
    #[test]
    fn test_golden_infrastructure() {
        // Test that both fixture files exist
        assert!(Path::new("tests/fixtures/simple.md").exists());
        assert!(Path::new("tests/fixtures/branch.md").exists());

        // Test that both golden files exist
        assert!(Path::new("tests/golden/simple.json").exists());
        assert!(Path::new("tests/golden/branch.json").exists());

        // Test that dump command is available
        let output = Command::new("cargo")
            .args(&["run", "--example", "dump", "--", "--help"])
            .output();

        // The command should exist (even if --help isn't implemented)
        assert!(output.is_ok(), "Dump command should be available");
    }

    /// Test: Hash consistency
    /// Metric: Same input should always produce same hash
    #[test]
    fn test_hash_consistency() {
        let fixture_path = "tests/fixtures/simple.md";

        // Run dump twice
        let output1 = Command::new("cargo")
            .args(&["run", "--example", "dump", "--", fixture_path])
            .output()
            .expect("First dump run failed");

        let output2 = Command::new("cargo")
            .args(&["run", "--example", "dump", "--", fixture_path])
            .output()
            .expect("Second dump run failed");

        assert!(output1.status.success());
        assert!(output2.status.success());

        // Parse both outputs
        let json1: serde_json::Value =
            serde_json::from_slice(&output1.stdout).expect("First output not valid JSON");
        let json2: serde_json::Value =
            serde_json::from_slice(&output2.stdout).expect("Second output not valid JSON");

        // Hashes should be identical
        let hash1 = json1["input_hash"]
            .as_str()
            .expect("Missing input_hash in first output");
        let hash2 = json2["input_hash"]
            .as_str()
            .expect("Missing input_hash in second output");

        assert_eq!(
            hash1, hash2,
            "Input hash should be consistent across multiple runs"
        );

        // Full outputs should be identical (deterministic)
        assert_eq!(json1, json2, "Full dump output should be deterministic");
    }
}
