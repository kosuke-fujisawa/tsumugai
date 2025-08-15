//! Directive types - what the engine tells the application to do.

use crate::ir::Choice;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Directive {
    Say { speaker: String, text: String },
    PlayBgm { res: ResId },
    PlaySe { res: ResId },
    ShowImage { res: ResId },
    PlayMovie { res: ResId },
    Wait { secs: f32 },
    Branch { choices: Vec<Choice> },
    Label { name: String },
    Jump { label: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResId {
    pub logical: String,
    pub resolved: Option<PathBuf>,
}
