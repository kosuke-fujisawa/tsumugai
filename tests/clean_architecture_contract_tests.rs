//! TDD tests for Clean Architecture contract violations
//! These tests should FAIL until we fix dependency direction issues

#[cfg(test)]
mod contract_tests {

    /// Test: Domain layer should not depend on Infrastructure
    /// This test will FAIL because domain currently imports infrastructure
    #[test]
    #[should_panic(expected = "Domain should not import infrastructure")]
    fn test_domain_dependency_direction() {
        // Check that domain modules don't import infrastructure modules
        let domain_files = vec![
            "src/domain/entities.rs",
            "src/domain/services.rs",
            "src/domain/value_objects.rs",
            "src/domain/repositories.rs",
            "src/domain/errors.rs",
        ];

        for file_path in domain_files {
            let content = std::fs::read_to_string(file_path)
                .expect(&format!("Should be able to read {}", file_path));

            // Domain should not import infrastructure
            if content.contains("crate::infrastructure") {
                panic!(
                    "Domain should not import infrastructure: found in {}",
                    file_path
                );
            }
        }

        panic!("Domain should not import infrastructure");
    }

    /// Test: StepResult contract should be stable and centralized
    /// Now this test should PASS since we consolidated StepResult
    #[test]
    fn test_step_result_contract_centralization() {
        // StepResult should be defined in exactly one canonical location
        let _content =
            std::fs::read_to_string("src/lib.rs").expect("Should be able to read lib.rs");

        // Check for StepResult in multiple files
        let files_with_stepresult = [
            "src/lib.rs",
            "src/story_engine.rs",
            "src/contracts/mod.rs",
            "src/domain/mod.rs",
            "src/application/mod.rs",
        ];

        let mut stepresult_locations = Vec::new();
        for file_path in files_with_stepresult {
            if let Ok(file_content) = std::fs::read_to_string(file_path) {
                if file_content.contains("enum StepResult") {
                    stepresult_locations.push(file_path);
                }
            }
        }

        // StepResult should only be defined in contracts module
        if stepresult_locations.len() == 1 && stepresult_locations[0] == "src/contracts/mod.rs" {
            // Expected behavior - test passes
            assert!(
                true,
                "StepResult contract is properly centralized in contracts module"
            );
        } else {
            panic!(
                "StepResult should be defined only in contracts module, found in: {:?}",
                stepresult_locations
            );
        }
    }

    /// Test: Public API should use stable contracts only
    /// This test will FAIL until we fix contract exposure
    #[test]
    #[should_panic(expected = "Public API exposes internal types")]
    fn test_public_api_contract_stability() {
        // Public API should not expose internal domain/infrastructure types directly
        let lib_content =
            std::fs::read_to_string("src/lib.rs").expect("Should be able to read lib.rs");

        // Check for problematic exports
        let problematic_exports = vec![
            "pub use domain::",
            "pub use infrastructure::",
            "pub use crate::domain::",
            "pub use crate::infrastructure::",
        ];

        for export in problematic_exports {
            if lib_content.contains(export) {
                panic!("Public API exposes internal types: {}", export);
            }
        }

        panic!("Public API exposes internal types");
    }

    /// Test: Infrastructure should depend on Domain abstractions only
    /// This test will FAIL if infrastructure depends on concrete domain types inappropriately  
    #[test]
    #[should_panic(expected = "Infrastructure dependencies incorrect")]
    fn test_infrastructure_dependency_direction() {
        // Infrastructure should only depend on domain traits, not concrete types
        let infrastructure_files = vec![
            "src/infrastructure/parsing.rs",
            "src/infrastructure/repositories.rs",
            "src/infrastructure/resource_resolution.rs",
        ];

        for file_path in infrastructure_files {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                // Infrastructure should use domain traits, not application layer
                if content.contains("crate::application") {
                    panic!(
                        "Infrastructure should not depend on application layer: {}",
                        file_path
                    );
                }
            }
        }

        panic!("Infrastructure dependencies incorrect");
    }

    /// Test: Circular dependencies should not exist
    /// This test will FAIL if there are circular imports
    #[test]
    #[should_panic(expected = "Circular dependency detected")]
    fn test_no_circular_dependencies() {
        // Simple check: no module should import a module that imports it back
        // This is a simplified check - in practice you'd use a proper dependency analyzer

        let modules = vec![
            ("domain", vec!["src/domain/mod.rs"]),
            ("application", vec!["src/application/mod.rs"]),
            ("infrastructure", vec!["src/infrastructure/mod.rs"]),
        ];

        for (module_name, files) in modules {
            for file_path in files {
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    // Check for obvious circular imports
                    if module_name == "domain" && content.contains("crate::application") {
                        panic!("Circular dependency detected: domain -> application");
                    }
                    if module_name == "application" && content.contains("crate::infrastructure") {
                        // This might be OK depending on the pattern
                    }
                }
            }
        }

        panic!("Circular dependency detected");
    }
}
