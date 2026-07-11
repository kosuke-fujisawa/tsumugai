//! 実行トレース（SPEC 5.1「経路の再現」）
//!
//! [`trace_path`] は SPEC 5章の実行モデルに従ってシーンを 1 経路ぶん
//! 自動実行し、通った場所と発生したイベントを [`TraceStep`] の列として返す。
//!
//! - 実行前に check と同じ検査を行い、error があれば実行しない（SPEC 6.1）
//! - 選択肢は `--choices` の選択番号（ブロック内の並び順、1 始まり）を
//!   先頭から消費して進む。尽きたら入力待ちとして停止する
//! - [`check_path`] と同じく infallible。入出力エラーも Diagnostic として
//!   [`TraceResult`] に含め、JSON 出力の形式を崩さない

use super::Block;
use super::check::CheckResult;
use super::exec::{Cursor, GotoResult, goto, segment_blocks, target_string};
use super::project::{LoadedScene, load_checked_project};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// 無限ループ保護：これ以上ステップを記録したら打ち切る（SPEC 5.1）
const MAX_STEPS: usize = 10_000;

/// trace の動作オプション
#[derive(Debug, Clone)]
pub struct TraceOptions {
    /// 選択肢ブロックで消費する選択番号（1 始まり、`--choices`）
    pub choices: Vec<usize>,
    /// background / bgm の実在チェック（`--no-assets` で false）
    pub check_assets: bool,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            choices: Vec::new(),
            check_assets: true,
        }
    }
}

/// トレースに記録される 1 ステップ
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TraceStep {
    /// シーンファイルに進入した（開始時・ファイルをまたぐ移動時）
    SceneEnter {
        file: PathBuf,
        id: Option<String>,
        title: Option<String>,
        background: Option<String>,
        bgm: Option<String>,
    },
    /// セクション（H2）に進入した（フォールスルー・リンク着地）
    SectionEnter {
        file: PathBuf,
        line: usize,
        heading: String,
        anchor: String,
    },
    Narration {
        file: PathBuf,
        line: usize,
        text: String,
    },
    Dialogue {
        file: PathBuf,
        line: usize,
        speaker: String,
        text: String,
    },
    /// 選択肢ブロックに到達した。`selected` は消費した選択番号
    /// （1 始まり）。None は入力待ちで停止したことを表す
    Choice {
        file: PathBuf,
        line: usize,
        options: Vec<TraceChoice>,
        selected: Option<usize>,
    },
    Jump {
        file: PathBuf,
        line: usize,
        label: String,
        target: String,
    },
    Ending {
        file: PathBuf,
        line: usize,
        id: String,
    },
}

/// 選択肢ブロックの 1 項目（表示用）
#[derive(Debug, Clone, Serialize)]
pub struct TraceChoice {
    pub label: String,
    pub target: String,
}

/// トレースの終わり方
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum TraceEnd {
    /// `<!-- ending: id -->` に到達した
    Ending { id: String },
    /// ファイル末尾に到達した（暗黙の終了、SPEC 5章）
    EndOfFile,
    /// 選択番号が尽きて入力待ちで停止した
    AwaitingChoice,
    /// 選択番号がブロックの項目数を超えている（error）
    InvalidChoice { given: usize, available: usize },
    /// ステップ数が上限に達して打ち切った（error）
    Truncated { max_steps: usize },
}

/// 1 経路の実行記録
#[derive(Debug, Serialize)]
pub struct Trace {
    pub steps: Vec<TraceStep>,
    pub end: TraceEnd,
    /// `--choices` に与えられた選択番号
    pub choices_requested: Vec<usize>,
    /// 実際に消費した個数（残りは未使用）
    pub choices_used: usize,
}

/// trace の結果。実行前検査の結果（check）を必ず含む
#[derive(Debug)]
pub struct TraceResult {
    /// 開始シーンとして指定されたパス
    pub file: PathBuf,
    /// 実行前検査（check と同じ規則）の結果
    pub check: CheckResult,
    /// 実行記録。check が error のときは None
    pub trace: Option<Trace>,
}

impl TraceResult {
    /// exit code を 1 にすべきか（check エラー、または経路再現の失敗）
    pub fn has_errors(&self) -> bool {
        self.check.has_errors()
            || matches!(
                self.trace.as_ref().map(|t| &t.end),
                Some(TraceEnd::InvalidChoice { .. } | TraceEnd::Truncated { .. })
            )
    }
}

/// シーンファイルを実行前検査してからトレースする（SPEC 5.1）。
///
/// パスが存在しない・ディレクトリ・検査 error の場合も panic や Err にせず、
/// Diagnostic 入りの [`TraceResult`] を返す（trace は None になる）。
pub fn trace_path(path: &Path, options: &TraceOptions) -> TraceResult {
    let project = match load_checked_project(path, "trace", options.check_assets) {
        Ok(project) => project,
        Err(check) => {
            return TraceResult {
                file: path.to_path_buf(),
                check,
                trace: None,
            };
        }
    };

    let trace = run(&project.scenes, options);
    TraceResult {
        file: path.to_path_buf(),
        check: project.check,
        trace: Some(trace),
    }
}

// ---------------------------------------------------------------- 実行

fn run(scenes: &[LoadedScene], options: &TraceOptions) -> Trace {
    let mut steps: Vec<TraceStep> = Vec::new();
    let mut next_choice = 0usize;
    let mut cursor = Cursor {
        scene: 0,
        seg: 0,
        block: 0,
    };
    push_scene_enter(&mut steps, &scenes[0]);

    let end = loop {
        if steps.len() >= MAX_STEPS {
            break TraceEnd::Truncated {
                max_steps: MAX_STEPS,
            };
        }
        let loaded = &scenes[cursor.scene];
        let scene = &loaded.parsed.scene;
        let blocks = segment_blocks(scene, cursor.seg);

        // セグメント末尾: 次のセクションへフォールスルー、なければ暗黙の終了
        if cursor.block >= blocks.len() {
            if cursor.seg < scene.sections.len() {
                cursor.seg += 1;
                cursor.block = 0;
                push_section_enter(&mut steps, loaded, cursor.seg - 1);
                continue;
            }
            break TraceEnd::EndOfFile;
        }

        match &blocks[cursor.block] {
            Block::Narration { text, line } => {
                steps.push(TraceStep::Narration {
                    file: loaded.path.clone(),
                    line: *line,
                    text: text.clone(),
                });
                cursor.block += 1;
            }
            Block::Dialogue {
                speaker,
                text,
                line,
            } => {
                steps.push(TraceStep::Dialogue {
                    file: loaded.path.clone(),
                    line: *line,
                    speaker: speaker.clone(),
                    text: text.clone(),
                });
                cursor.block += 1;
            }
            Block::Ending { id, line } => {
                steps.push(TraceStep::Ending {
                    file: loaded.path.clone(),
                    line: *line,
                    id: id.clone(),
                });
                break TraceEnd::Ending { id: id.clone() };
            }
            Block::Jump {
                label,
                target,
                line,
            } => {
                steps.push(TraceStep::Jump {
                    file: loaded.path.clone(),
                    line: *line,
                    label: label.clone(),
                    target: target_string(target),
                });
                let result = goto(&mut cursor, scenes, target);
                push_goto(&mut steps, scenes, &cursor, result);
            }
            Block::Choices { items, line } => {
                let shown: Vec<TraceChoice> = items
                    .iter()
                    .map(|item| TraceChoice {
                        label: item.label.clone(),
                        target: target_string(&item.target),
                    })
                    .collect();
                let Some(&given) = options.choices.get(next_choice) else {
                    steps.push(TraceStep::Choice {
                        file: loaded.path.clone(),
                        line: *line,
                        options: shown,
                        selected: None,
                    });
                    break TraceEnd::AwaitingChoice;
                };
                if given == 0 || given > items.len() {
                    steps.push(TraceStep::Choice {
                        file: loaded.path.clone(),
                        line: *line,
                        options: shown,
                        selected: None,
                    });
                    break TraceEnd::InvalidChoice {
                        given,
                        available: items.len(),
                    };
                }
                next_choice += 1;
                let target = items[given - 1].target.clone();
                steps.push(TraceStep::Choice {
                    file: loaded.path.clone(),
                    line: *line,
                    options: shown,
                    selected: Some(given),
                });
                let result = goto(&mut cursor, scenes, &target);
                push_goto(&mut steps, scenes, &cursor, result);
            }
        }
    };

    Trace {
        steps,
        end,
        choices_requested: options.choices.clone(),
        choices_used: next_choice,
    }
}

/// [`goto`] の結果を trace のステップ列に反映する
fn push_goto(
    steps: &mut Vec<TraceStep>,
    scenes: &[LoadedScene],
    cursor: &Cursor,
    result: GotoResult,
) {
    if let Some(idx) = result.entered_scene {
        push_scene_enter(steps, &scenes[idx]);
    }
    if let Some(section_idx) = result.entered_section {
        push_section_enter(steps, &scenes[cursor.scene], section_idx);
    }
}

fn push_scene_enter(steps: &mut Vec<TraceStep>, loaded: &LoadedScene) {
    let scene = &loaded.parsed.scene;
    steps.push(TraceStep::SceneEnter {
        file: loaded.path.clone(),
        id: scene.id.clone(),
        title: scene.title.clone(),
        background: scene.background.clone(),
        bgm: scene.bgm.clone(),
    });
}

fn push_section_enter(steps: &mut Vec<TraceStep>, loaded: &LoadedScene, section_idx: usize) {
    let section = &loaded.parsed.scene.sections[section_idx];
    steps.push(TraceStep::SectionEnter {
        file: loaded.path.clone(),
        line: section.line,
        heading: section.heading.clone(),
        anchor: section.anchor.clone(),
    });
}
