//! Application Engine - High-level API for markdown execution
//!
//! This module provides the main entry point for external users.

use crate::application::api::{ApiError, Directive, NextAction, StepResult};
use crate::engine::{Engine as CoreEngine, Step, WaitKind};
use crate::infrastructure::md_parser::parse_markdown;
use crate::resolve::Resolver;

/// High-level engine that provides the public API
pub struct Engine {
    core: CoreEngine,
    current_choices: Option<Vec<String>>,
    last_branch_choices: Option<Vec<crate::ir::Choice>>,
}

impl Engine {
    /// Create engine from markdown source
    pub fn from_markdown(src: &str) -> Result<Self, ApiError> {
        let program = parse_markdown(src)?;

        let core = CoreEngine::new(program);
        Ok(Self {
            core,
            current_choices: None,
            last_branch_choices: None,
        })
    }

    /// Create engine from markdown with resolver
    pub fn from_markdown_with_resolver(
        src: &str,
        resolver: Box<dyn Resolver>,
    ) -> Result<Self, ApiError> {
        let program = parse_markdown(src)?;

        let core = CoreEngine::with_resolver(program, resolver);
        Ok(Self {
            core,
            current_choices: None,
            last_branch_choices: None,
        })
    }

    /// Execute next step
    pub fn step(&mut self) -> Result<StepResult, ApiError> {
        let step = self.core.step();
        let directives = self.core.take_emitted();

        let api_directives = directives
            .into_iter()
            .map(|d| self.convert_directive(d))
            .collect::<Result<Vec<_>, _>>()?;

        match step {
            Step::Next => Ok(StepResult {
                next: NextAction::Next,
                directives: api_directives,
            }),
            Step::Wait(WaitKind::User) => Ok(StepResult {
                next: NextAction::WaitUser,
                directives: api_directives,
            }),
            Step::Wait(WaitKind::Branch(choices)) => {
                let choice_texts: Vec<String> = choices.iter().map(|c| c.choice.clone()).collect();
                self.current_choices = Some(choice_texts.clone());
                self.last_branch_choices = Some(choices); // Cache for choose()
                Ok(StepResult {
                    next: NextAction::WaitBranch,
                    directives: vec![Directive::Branch {
                        choices: choice_texts,
                    }],
                })
            }
            Step::Wait(WaitKind::Timer(secs)) => Ok(StepResult {
                next: NextAction::WaitUser,
                directives: vec![Directive::Wait { seconds: secs }],
            }),
            Step::Jump(label) => {
                self.core
                    .jump_to(&label)
                    .map_err(|e| ApiError::engine(e.to_string()))?;
                // Return immediately without recursion, preserving collected directives
                Ok(StepResult {
                    next: NextAction::Next,
                    directives: api_directives,
                })
            }
            Step::Halt => Ok(StepResult {
                next: NextAction::Halt,
                directives: api_directives,
            }),
        }
    }

    /// Choose option by index
    pub fn choose(&mut self, index: usize) -> Result<(), ApiError> {
        if self.current_choices.is_none() {
            return Err(ApiError::invalid("No choices available"));
        }

        // Use cached branch choices instead of calling step() again
        if let Some(ref branch_choices) = self.last_branch_choices {
            if index >= branch_choices.len() {
                return Err(ApiError::invalid(format!(
                    "Choice index {} out of range (0-{})",
                    index,
                    branch_choices.len().saturating_sub(1)
                )));
            }

            let label = &branch_choices[index].label;
            self.core
                .jump_to(label)
                .map_err(|e| ApiError::engine(format!("{e:?}")))?;
            self.current_choices = None;
            self.last_branch_choices = None; // Clear cache after use
            Ok(())
        } else {
            Err(ApiError::invalid("No cached branch choices available"))
        }
    }

    /// Get current variable value
    pub fn get_var(&self, name: &str) -> Option<String> {
        self.core.vars().get(name).map(|v| {
            use crate::ir::Value;
            match v {
                Value::Int(i) => i.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Str(s) => s.clone(),
            }
        })
    }

    /// Set variable value
    pub fn set_var(&mut self, name: &str, value: &str) {
        // This would need proper Value parsing in a real implementation
        use crate::ir::Value;
        if let Ok(int_val) = value.parse::<i32>() {
            self.core
                .vars_mut()
                .insert(name.to_string(), Value::Int(int_val));
        } else if let Ok(bool_val) = value.parse::<bool>() {
            self.core
                .vars_mut()
                .insert(name.to_string(), Value::Bool(bool_val));
        } else {
            self.core
                .vars_mut()
                .insert(name.to_string(), Value::Str(value.to_string()));
        }
    }

    fn convert_directive(&self, dir: crate::engine::Directive) -> Result<Directive, ApiError> {
        use crate::engine::Directive as CoreDirective;

        match dir {
            CoreDirective::Say { speaker, text } => Ok(Directive::Say { speaker, text }),
            CoreDirective::PlayBgm { res } => Ok(Directive::PlayBgm {
                path: res.resolved.map(|p| p.to_string_lossy().to_string()),
            }),
            CoreDirective::PlaySe { res } => Ok(Directive::PlaySe {
                path: res.resolved.map(|p| p.to_string_lossy().to_string()),
            }),
            CoreDirective::ShowImage { res } => Ok(Directive::ShowImage {
                layer: "main".to_string(), // Default layer
                path: res.resolved.map(|p| p.to_string_lossy().to_string()),
            }),
            CoreDirective::PlayMovie { res } => Ok(Directive::PlayMovie {
                path: res.resolved.map(|p| p.to_string_lossy().to_string()),
            }),
            CoreDirective::Wait { secs } => Ok(Directive::Wait { seconds: secs }),
            CoreDirective::Branch { choices } => {
                let choice_texts = choices.iter().map(|c| c.choice.clone()).collect();
                Ok(Directive::Branch {
                    choices: choice_texts,
                })
            }
            CoreDirective::Jump { label } => Ok(Directive::JumpTo { label }),
            CoreDirective::Label { name } => Ok(Directive::ReachedLabel { label: name }),
        }
    }
}
