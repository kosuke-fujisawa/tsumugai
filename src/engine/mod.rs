//! Engine module - step-by-step execution of Commands into Directives.

pub mod directive;

pub use directive::{Directive, ResId};

use crate::ir::{Cmp, Command, Op, Program, SaveData, Value, VarStore};
use crate::resolve::Resolver;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Next,
    Wait(WaitKind),
    Jump(String),
    Halt,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WaitKind {
    /// 台詞やWAITやムービー… Enter等で進む
    User,
    /// 分岐選択が入るまで進めない（選択肢payloadもここで渡す）
    Branch(Vec<crate::ir::Choice>),
    /// タイマー待ち（秒数指定）
    Timer(f32),
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Label not found: {0}")]
    LabelNotFound(String),
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    #[error("Type mismatch for variable {0}")]
    TypeMismatch(String),
    #[error("Program counter out of bounds: {0}")]
    PcOutOfBounds(usize),
}

pub struct Engine {
    program: Program,
    pc: usize,
    vars: VarStore,
    emitted: Vec<Directive>,
    resolver: Option<Box<dyn Resolver>>,
    labels: BTreeMap<String, usize>,
    branch_emitted: bool,
}

impl Engine {
    pub fn new(program: Program) -> Self {
        let labels = Self::build_label_map(&program);
        Self {
            program,
            pc: 0,
            vars: BTreeMap::new(),
            emitted: Vec::new(),
            resolver: None,
            labels,
            branch_emitted: false,
        }
    }

    pub fn with_resolver(program: Program, resolver: Box<dyn Resolver>) -> Self {
        let labels = Self::build_label_map(&program);
        Self {
            program,
            pc: 0,
            vars: BTreeMap::new(),
            emitted: Vec::new(),
            resolver: Some(resolver),
            labels,
            branch_emitted: false,
        }
    }

    fn build_label_map(program: &Program) -> BTreeMap<String, usize> {
        let mut labels = BTreeMap::new();
        for (idx, cmd) in program.cmds.iter().enumerate() {
            if let Command::Label { name } = cmd {
                labels.insert(name.clone(), idx);
            }
        }
        labels
    }

    pub fn step(&mut self) -> Step {
        if self.pc >= self.program.cmds.len() {
            return Step::Halt;
        }

        let cmd = &self.program.cmds[self.pc].clone();

        match cmd {
            Command::Say { speaker, text } => {
                self.emit(Directive::Say {
                    speaker: speaker.clone(),
                    text: text.clone(),
                });
                self.pc += 1;
                Step::Wait(WaitKind::User)
            }
            Command::PlayBgm { name } => {
                let res = self.resolve_bgm(name);
                self.emit(Directive::PlayBgm { res });
                self.pc += 1;
                Step::Next
            }
            Command::PlaySe { name } => {
                let res = self.resolve_se(name);
                self.emit(Directive::PlaySe { res });
                self.pc += 1;
                Step::Next
            }
            Command::ShowImage { file } => {
                let res = self.resolve_image(file);
                self.emit(Directive::ShowImage { res });
                self.pc += 1;
                Step::Next
            }
            Command::PlayMovie { file } => {
                let res = self.resolve_movie(file);
                self.emit(Directive::PlayMovie { res });
                self.pc += 1;
                Step::Wait(WaitKind::User)
            }
            Command::Wait { secs } => {
                self.emit(Directive::Wait { secs: *secs });
                self.pc += 1;
                Step::Wait(WaitKind::Timer(*secs))
            }
            Command::Branch { choices } => {
                // emit は1回だけにしたければフラグで制御（なくても致命ではない）
                if !self.branch_emitted {
                    self.emit(Directive::Branch {
                        choices: choices.clone(),
                    });
                    self.branch_emitted = true;
                }
                // ★ pc を進めない。ここで"分岐待ち"を明示して返す
                Step::Wait(WaitKind::Branch(choices.clone()))
            }
            Command::Jump { label } => {
                self.emit(Directive::Jump {
                    label: label.clone(),
                });
                self.pc += 1;
                Step::Jump(label.clone())
            }
            Command::Label { name } => {
                self.emit(Directive::Label { name: name.clone() });
                self.pc += 1;
                Step::Next
            }
            Command::Set { name, value } => {
                self.vars.insert(name.clone(), value.clone());
                self.pc += 1;
                Step::Next
            }
            Command::Modify { name, op, value } => {
                if let Some(Value::Int(current_val)) = self.vars.get_mut(name) {
                    if let &Value::Int(modify_val) = value {
                        match op {
                            Op::Add => *current_val += modify_val,
                            Op::Sub => *current_val -= modify_val,
                        }
                    }
                }
                self.pc += 1;
                Step::Next
            }
            Command::JumpIf {
                var,
                cmp,
                value,
                label,
            } => {
                if let Some(var_value) = self.vars.get(var) {
                    let should_jump = match (var_value, value) {
                        (Value::Int(a), Value::Int(b)) => match cmp {
                            Cmp::Eq => a == b,
                            Cmp::Ne => a != b,
                            Cmp::Lt => a < b,
                            Cmp::Le => a <= b,
                            Cmp::Gt => a > b,
                            Cmp::Ge => a >= b,
                        },
                        (Value::Bool(a), Value::Bool(b)) => match cmp {
                            Cmp::Eq => a == b,
                            Cmp::Ne => a != b,
                            _ => false,
                        },
                        (Value::Str(a), Value::Str(b)) => match cmp {
                            Cmp::Eq => a == b,
                            Cmp::Ne => a != b,
                            _ => false,
                        },
                        _ => false,
                    };

                    if should_jump {
                        self.pc += 1;
                        Step::Jump(label.clone())
                    } else {
                        self.pc += 1;
                        Step::Next
                    }
                } else {
                    self.pc += 1;
                    Step::Next
                }
            }
        }
    }

    pub fn take_emitted(&mut self) -> Vec<Directive> {
        std::mem::take(&mut self.emitted)
    }

    pub fn jump_to(&mut self, label: &str) -> Result<(), EngineError> {
        if let Some(&pc) = self.labels.get(label) {
            self.pc = pc;
            self.branch_emitted = false; // 同じ分岐に戻るケースで再emitしたいなら
            Ok(())
        } else {
            Err(EngineError::LabelNotFound(label.to_string()))
        }
    }

    pub fn vars(&self) -> &VarStore {
        &self.vars
    }

    pub fn vars_mut(&mut self) -> &mut VarStore {
        &mut self.vars
    }

    pub fn snapshot(&self) -> SaveData {
        SaveData {
            pc: self.pc,
            vars: self.vars.clone(),
            seen: None,
            rng_seed: None,
        }
    }

    pub fn restore(&mut self, save: SaveData) -> Result<(), EngineError> {
        if save.pc > self.program.cmds.len() {
            return Err(EngineError::PcOutOfBounds(save.pc));
        }

        self.pc = save.pc;
        self.vars = save.vars;
        Ok(())
    }

    fn emit(&mut self, directive: Directive) {
        self.emitted.push(directive);
    }

    fn resolve_bgm(&self, name: &str) -> ResId {
        let resolved = self.resolver.as_ref().and_then(|r| r.resolve_bgm(name));
        ResId {
            logical: name.to_string(),
            resolved,
        }
    }

    fn resolve_se(&self, name: &str) -> ResId {
        let resolved = self.resolver.as_ref().and_then(|r| r.resolve_se(name));
        ResId {
            logical: name.to_string(),
            resolved,
        }
    }

    fn resolve_image(&self, name: &str) -> ResId {
        let resolved = self.resolver.as_ref().and_then(|r| r.resolve_image(name));
        ResId {
            logical: name.to_string(),
            resolved,
        }
    }

    fn resolve_movie(&self, name: &str) -> ResId {
        let resolved = self.resolver.as_ref().and_then(|r| r.resolve_movie(name));
        ResId {
            logical: name.to_string(),
            resolved,
        }
    }

    pub fn pc(&self) -> usize {
        self.pc
    }
}
