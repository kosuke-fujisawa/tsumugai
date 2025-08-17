//! DFS scenario coverage tests
//! Ensures all branching paths are explored with depth limits to prevent infinite loops

use tsumugai::{parse};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod dfs_tests {
    use super::*;

    /// DFS test: Complex branching scenario
    /// Metric: All branches should be reachable and explored
    #[test]
    fn test_complex_branching_dfs() {
        let scenario = r#"
[SAY speaker=Start]
Beginning

[BRANCH choice=Path1 label=path1, choice=Path2 label=path2, choice=Path3 label=path3]

[LABEL name=path1]
[SAY speaker=A] Path 1
[BRANCH choice=Sub1A label=sub1a, choice=Sub1B label=sub1b]

[LABEL name=sub1a]
[SAY speaker=A] Sub 1A
[JUMP label=end]

[LABEL name=sub1b]  
[SAY speaker=A] Sub 1B
[JUMP label=end]

[LABEL name=path2]
[SAY speaker=B] Path 2
[JUMP label=end]

[LABEL name=path3]
[SAY speaker=C] Path 3
[JUMP label=end]

[LABEL name=end]
[SAY speaker=End] The End
"#;

        let program = parse(scenario).expect("Should parse successfully");
        
        let mut path_coverage = DfsPathExplorer::new();
        path_coverage.explore_all_paths(&program, 50); // Max depth 50
        
        // Verify all expected paths were covered
        let paths = path_coverage.get_discovered_paths();
        
        // Should have at least 4 distinct paths:
        // 1. Start -> path1 -> sub1a -> end
        // 2. Start -> path1 -> sub1b -> end  
        // 3. Start -> path2 -> end
        // 4. Start -> path3 -> end
        assert!(paths.len() >= 4, "Should discover at least 4 distinct paths, found {}", paths.len());
        
        // Verify specific labels were reached
        let reached_labels = path_coverage.get_reached_labels();
        let expected_labels = ["path1", "path2", "path3", "sub1a", "sub1b", "end"];
        
        for label in expected_labels.iter() {
            assert!(
                reached_labels.contains(*label),
                "Label '{}' should be reachable, reached labels: {:?}",
                label,
                reached_labels
            );
        }
    }

    /// DFS test: Loop prevention
    /// Metric: Infinite loops should be detected and prevented
    #[test]
    fn test_loop_prevention() {
        let scenario = r#"
[LABEL name=loop_start]
[SAY speaker=A] In loop
[JUMP label=loop_start]
"#;

        let program = parse(scenario).expect("Should parse successfully");
        
        let mut path_coverage = DfsPathExplorer::new();
        path_coverage.explore_all_paths(&program, 10); // Small depth limit
        
        // Should detect the loop and stop
        let paths = path_coverage.get_discovered_paths();
        assert!(paths.len() > 0, "Should discover at least one path");
        
        // Should not exceed depth limit
        let max_path_length = paths.iter().map(|p| p.len()).max().unwrap_or(0);
        assert!(max_path_length <= 15, "Path length should be bounded, got {}", max_path_length);
    }

    /// DFS test: Conditional branching
    /// Metric: Variable-dependent branches should be explored
    #[test]
    fn test_conditional_branching() {
        let scenario = r#"
[SET name=score value=0]

[SAY speaker=A] Start

[MODIFY name=score op=add value=10]

[JUMP_IF var=score cmp=ge value=10 label=high_score]

[SAY speaker=A] Low score path
[JUMP label=end]

[LABEL name=high_score]
[SAY speaker=A] High score path

[LABEL name=end]
[SAY speaker=A] End
"#;

        let program = parse(scenario).expect("Should parse successfully");
        
        let mut path_coverage = DfsPathExplorer::new();
        path_coverage.explore_all_paths(&program, 20);
        
        let reached_labels = path_coverage.get_reached_labels();
        
        // Should reach high_score label (since score becomes 10)
        assert!(
            reached_labels.contains("high_score"),
            "Should reach high_score label with score=10"
        );
        
        assert!(
            reached_labels.contains("end"),
            "Should reach end label"
        );
    }

    /// DFS test: Dead code detection
    /// Metric: Unreachable code should be identified
    #[test]
    fn test_dead_code_detection() {
        let scenario = r#"
[SAY speaker=A] Start
[JUMP label=reachable]

[SAY speaker=B] This is unreachable
[LABEL name=unreachable_label]

[LABEL name=reachable]
[SAY speaker=A] End
"#;

        let program = parse(scenario).expect("Should parse successfully");
        
        let mut path_coverage = DfsPathExplorer::new();
        path_coverage.explore_all_paths(&program, 20);
        
        let reached_labels = path_coverage.get_reached_labels();
        
        // Should reach 'reachable' but not 'unreachable_label'
        assert!(
            reached_labels.contains("reachable"),
            "Should reach reachable label"
        );
        
        assert!(
            !reached_labels.contains("unreachable_label"),
            "Should NOT reach unreachable_label"
        );
        
        // Verify dead code detection
        let dead_code = path_coverage.find_dead_code(&program);
        assert!(dead_code.len() > 0, "Should detect dead code");
    }
}

/// Path explorer using DFS to discover all reachable execution paths
struct DfsPathExplorer {
    discovered_paths: Vec<Vec<usize>>,
    reached_labels: HashSet<String>,
    visited_states: HashSet<String>,
}

impl DfsPathExplorer {
    fn new() -> Self {
        Self {
            discovered_paths: Vec::new(),
            reached_labels: HashSet::new(),
            visited_states: HashSet::new(),
        }
    }

    /// Explore all possible execution paths up to max_depth
    fn explore_all_paths(&mut self, program: &tsumugai::Program, max_depth: usize) {
        let initial_path = Vec::new();
        self.dfs_explore(program, 0, HashMap::new(), initial_path, max_depth);
    }

    /// Recursive DFS exploration
    fn dfs_explore(
        &mut self,
        program: &tsumugai::Program,
        pc: usize,
        mut variables: HashMap<String, tsumugai::Value>,
        mut current_path: Vec<usize>,
        remaining_depth: usize,
    ) {
        if remaining_depth == 0 || pc >= program.cmds.len() {
            if !current_path.is_empty() {
                self.discovered_paths.push(current_path);
            }
            return;
        }

        // Simple cycle detection within current path
        if current_path.contains(&pc) {
            // Found a cycle in current path - save it and return
            current_path.push(pc);
            self.discovered_paths.push(current_path);
            return;
        }

        current_path.push(pc);

        let cmd = &program.cmds[pc];
        match cmd {
            tsumugai::Command::Say { .. } => {
                self.dfs_explore(program, pc + 1, variables, current_path, remaining_depth - 1);
            }
            tsumugai::Command::Wait { .. } => {
                self.dfs_explore(program, pc + 1, variables, current_path, remaining_depth - 1);
            }
            tsumugai::Command::PlayBgm { .. } |
            tsumugai::Command::PlaySe { .. } |
            tsumugai::Command::ShowImage { .. } |
            tsumugai::Command::PlayMovie { .. } => {
                self.dfs_explore(program, pc + 1, variables, current_path, remaining_depth - 1);
            }
            tsumugai::Command::Label { name } => {
                self.reached_labels.insert(name.clone());
                self.dfs_explore(program, pc + 1, variables, current_path, remaining_depth - 1);
            }
            tsumugai::Command::Jump { label } => {
                if let Some(target_pc) = self.find_label_pc(program, label) {
                    self.dfs_explore(program, target_pc, variables, current_path, remaining_depth - 1);
                }
            }
            tsumugai::Command::Branch { choices } => {
                // Explore each branch choice
                for choice in choices {
                    if let Some(target_pc) = self.find_label_pc(program, &choice.label) {
                        self.dfs_explore(program, target_pc, variables.clone(), current_path.clone(), remaining_depth - 1);
                    }
                }
            }
            tsumugai::Command::Set { name, value } => {
                variables.insert(name.clone(), value.clone());
                self.dfs_explore(program, pc + 1, variables, current_path, remaining_depth - 1);
            }
            tsumugai::Command::Modify { name, op, value } => {
                if let Some(current_val) = variables.get(name) {
                    if let (tsumugai::Value::Int(current), tsumugai::Value::Int(modify)) = (current_val, value) {
                        let new_val = match op {
                            tsumugai::Op::Add => current + modify,
                            tsumugai::Op::Sub => current - modify,
                        };
                        variables.insert(name.clone(), tsumugai::Value::Int(new_val));
                    }
                }
                self.dfs_explore(program, pc + 1, variables, current_path, remaining_depth - 1);
            }
            tsumugai::Command::JumpIf { var, cmp, value, label } => {
                let should_jump = if let Some(var_value) = variables.get(var) {
                    self.evaluate_condition(var_value, cmp, value)
                } else {
                    false
                };

                if should_jump {
                    if let Some(target_pc) = self.find_label_pc(program, label) {
                        self.dfs_explore(program, target_pc, variables, current_path, remaining_depth - 1);
                    }
                } else {
                    self.dfs_explore(program, pc + 1, variables, current_path, remaining_depth - 1);
                }
            }
        }
    }

    fn find_label_pc(&self, program: &tsumugai::Program, label: &str) -> Option<usize> {
        for (i, cmd) in program.cmds.iter().enumerate() {
            if let tsumugai::Command::Label { name } = cmd {
                if name == label {
                    return Some(i);
                }
            }
        }
        None
    }

    fn evaluate_condition(&self, var_value: &tsumugai::Value, cmp: &tsumugai::Cmp, value: &tsumugai::Value) -> bool {
        match (var_value, value) {
            (tsumugai::Value::Int(a), tsumugai::Value::Int(b)) => match cmp {
                tsumugai::Cmp::Eq => a == b,
                tsumugai::Cmp::Ne => a != b,
                tsumugai::Cmp::Lt => a < b,
                tsumugai::Cmp::Le => a <= b,
                tsumugai::Cmp::Gt => a > b,
                tsumugai::Cmp::Ge => a >= b,
            },
            (tsumugai::Value::Bool(a), tsumugai::Value::Bool(b)) => match cmp {
                tsumugai::Cmp::Eq => a == b,
                tsumugai::Cmp::Ne => a != b,
                _ => false,
            },
            (tsumugai::Value::Str(a), tsumugai::Value::Str(b)) => match cmp {
                tsumugai::Cmp::Eq => a == b,
                tsumugai::Cmp::Ne => a != b,
                _ => false,
            },
            _ => false,
        }
    }

    fn get_discovered_paths(&self) -> &Vec<Vec<usize>> {
        &self.discovered_paths
    }

    fn get_reached_labels(&self) -> &HashSet<String> {
        &self.reached_labels
    }

    fn find_dead_code(&self, program: &tsumugai::Program) -> Vec<usize> {
        let mut reachable_pcs: HashSet<usize> = HashSet::new();
        
        for path in &self.discovered_paths {
            for &pc in path {
                reachable_pcs.insert(pc);
            }
        }

        let mut dead_code = Vec::new();
        for i in 0..program.cmds.len() {
            if !reachable_pcs.contains(&i) {
                dead_code.push(i);
            }
        }

        dead_code
    }
}