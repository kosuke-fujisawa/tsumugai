//! Intermediate Representation (IR) types for the tsumugai engine.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Label {
        name: String,
    },
    Jump {
        label: String,
    },
    Say {
        speaker: String,
        text: String,
    },
    PlayBgm {
        name: String,
    },
    PlaySe {
        name: String,
    },
    ShowImage {
        file: String,
    },
    PlayMovie {
        file: String,
    },
    Wait {
        secs: f32,
    },
    Branch {
        choices: Vec<Choice>,
    },
    Set {
        name: String,
        value: Value,
    },
    Modify {
        name: String,
        op: Op,
        value: Value,
    },
    JumpIf {
        var: String,
        cmp: Cmp,
        value: Value,
        label: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub choice: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Int(i32),
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Sub,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub cmds: Vec<Command>,
}

impl Program {
    pub fn new(cmds: Vec<Command>) -> Self {
        Self { cmds }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveData {
    pub pc: usize,
    pub vars: BTreeMap<String, Value>,
    pub seen: Option<Vec<u64>>,
    pub rng_seed: Option<u64>,
}

pub type VarStore = BTreeMap<String, Value>;

impl Value {
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}
