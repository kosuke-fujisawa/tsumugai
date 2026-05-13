//! Runtime Trace — シナリオ実行の追跡
//!
//! `trace_linear` は選択肢を常に先頭から選ぶ自動実行を行い、
//! 各 step の入力・出力・状態変化を `RuntimeTrace` として返す。
//!
//! # 使い方
//! ```no_run
//! use tsumugai::{parser, runtime};
//! use tsumugai::runtime::trace::trace_linear;
//!
//! let md = "...";
//! let ast = parser::parse(md).unwrap();
//! let program = runtime::compile(&ast);
//! let trace = trace_linear(&program);
//! ```

use crate::runtime::ir::{Event, Program};
use crate::runtime::{Input, WaitingType, step};
use crate::types::state::State;
use serde::Serialize;
use std::collections::HashMap;

/// 無限ループ保護：これ以上 step したら打ち切る
const MAX_STEPS: usize = 1000;

// ──────────────────────────────────────────────
// 型定義
// ──────────────────────────────────────────────

/// 1変数の変化
#[derive(Debug, Clone, Serialize)]
pub struct VarChange {
    pub key: String,
    /// 変化前の値（新規追加の場合は None）
    pub before: Option<String>,
    pub after: String,
}

/// 1 step での状態差分
#[derive(Debug, Clone, Serialize)]
pub struct StateDiff {
    pub var_changes: Vec<VarChange>,
}

/// 1 step（`runtime::step()` 1回分）の記録
#[derive(Debug, Clone, Serialize)]
pub struct TraceStep {
    pub step_no: usize,
    /// step 呼び出し前の PC
    pub pc_before: usize,
    /// この step に与えた入力（最初の step は null）
    pub input: Option<String>,
    /// step 呼び出し後の PC
    pub pc_after: usize,
    /// この step で発生したイベント
    pub events: Vec<Event>,
    /// 変数の変化
    pub state_diff: StateDiff,
    /// 次に何を待っているか（null ならシナリオ終了）
    pub waiting_for: Option<String>,
}

/// シナリオ1回の実行トレース全体
#[derive(Debug, Serialize)]
pub struct RuntimeTrace {
    pub total_steps: usize,
    /// 無限ループ等で MAX_STEPS に達した場合は true
    pub truncated: bool,
    pub steps: Vec<TraceStep>,
}

/// `--json` 出力用ラッパー
#[derive(Debug, Serialize)]
pub struct TraceJsonOutput {
    pub status: &'static str,
    pub trace: RuntimeTrace,
}

// ──────────────────────────────────────────────
// 線形トレース（常に先頭選択肢を選ぶ自動実行）
// ──────────────────────────────────────────────

/// プログラムを自動実行して `RuntimeTrace` を返す
///
/// 選択肢は常に先頭（表示条件を満たすもの）を選ぶ。
/// `MAX_STEPS` を超えると打ち切り `truncated = true` になる。
pub fn trace_linear(program: &Program) -> RuntimeTrace {
    let mut state = State::new();
    let mut steps: Vec<TraceStep> = Vec::new();
    let mut input: Option<Input> = None;
    let mut truncated = false;

    for step_no in 0..MAX_STEPS {
        let pc_before = state.pc;
        let flags_before = state.flags.clone();
        let input_label = input.as_ref().map(describe_input);

        let (new_state, output) = step(state, program, input.take());
        state = new_state;

        let var_changes = diff_flags(&flags_before, &state.flags);
        let waiting_label = output.waiting_for.as_ref().map(describe_waiting);

        steps.push(TraceStep {
            step_no,
            pc_before,
            input: input_label,
            pc_after: state.pc,
            events: output.events,
            state_diff: StateDiff { var_changes },
            waiting_for: waiting_label,
        });

        input = match &output.waiting_for {
            None => break,
            Some(WaitingType::Advance) => Some(Input::Advance),
            Some(WaitingType::Choice(opts)) => {
                if let Some(first) = opts.first() {
                    Some(Input::SelectChoice(first.id.clone()))
                } else {
                    break;
                }
            }
            Some(WaitingType::Ended { .. }) => break,
        };

        if step_no + 1 == MAX_STEPS {
            truncated = true;
        }
    }

    let total = steps.len();
    RuntimeTrace {
        total_steps: total,
        truncated,
        steps,
    }
}

// ──────────────────────────────────────────────
// ヘルパー
// ──────────────────────────────────────────────

fn describe_input(input: &Input) -> String {
    match input {
        Input::Advance => "Advance".to_string(),
        Input::SelectChoice(id) => format!("SelectChoice({})", id),
    }
}

fn describe_waiting(waiting: &WaitingType) -> String {
    match waiting {
        WaitingType::Advance => "Advance".to_string(),
        WaitingType::Choice(opts) => {
            let labels: Vec<&str> = opts.iter().map(|o| o.label.as_str()).collect();
            format!("Choice({})", labels.join(" / "))
        }
        WaitingType::Ended { id, name } => format!("Ended(id={}, name={})", id, name),
    }
}

fn describe_flag_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn diff_flags(
    before: &HashMap<String, serde_json::Value>,
    after: &HashMap<String, serde_json::Value>,
) -> Vec<VarChange> {
    let mut changes = Vec::new();

    for (key, after_val) in after {
        let before_val = before.get(key);
        match before_val {
            None => changes.push(VarChange {
                key: key.clone(),
                before: None,
                after: describe_flag_value(after_val),
            }),
            Some(b) if b != after_val => changes.push(VarChange {
                key: key.clone(),
                before: Some(describe_flag_value(b)),
                after: describe_flag_value(after_val),
            }),
            _ => {}
        }
    }

    // 削除されたキーも記録（現時点では State は変数を削除しないが念のため）
    for (key, before_val) in before {
        if !after.contains_key(key) {
            changes.push(VarChange {
                key: key.clone(),
                before: Some(describe_flag_value(before_val)),
                after: "(removed)".to_string(),
            });
        }
    }

    changes.sort_by(|a, b| a.key.cmp(&b.key));
    changes
}

// ──────────────────────────────────────────────
// テスト
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser::parse, runtime::compile};

    #[test]
    fn 台詞のトレース() {
        let md = "[SAY speaker=Alice]\nこんにちは！\n";
        let ast = parse(md).unwrap();
        let program = compile(&ast);
        let trace = trace_linear(&program);

        assert!(!trace.truncated);
        // step 0: None → Say + AwaitAdvance
        // step 1: Advance → プログラム末尾
        assert_eq!(trace.total_steps, 2);
        assert_eq!(trace.steps[0].input, None);
        assert_eq!(trace.steps[0].events.len(), 1);
        assert_eq!(trace.steps[0].waiting_for.as_deref(), Some("Advance"));
        assert_eq!(trace.steps[1].input.as_deref(), Some("Advance"));
        assert_eq!(trace.steps[1].waiting_for, None);
    }

    #[test]
    fn 選択肢のトレース_先頭選択() {
        let md = r#"
[BRANCH choice=はい label=yes, choice=いいえ label=no]

[LABEL name=yes]
[SAY speaker=Alice]
選んだ。

[LABEL name=no]
[SAY speaker=Alice]
選ばなかった。
"#;
        let ast = parse(md).unwrap();
        let program = compile(&ast);
        let trace = trace_linear(&program);

        assert!(!trace.truncated);
        // step 0: None → AwaitChoice
        assert!(
            trace.steps[0]
                .waiting_for
                .as_deref()
                .unwrap_or("")
                .starts_with("Choice(")
        );
        // step 1: SelectChoice → Say + AwaitAdvance
        assert!(
            trace.steps[1]
                .input
                .as_deref()
                .unwrap_or("")
                .starts_with("SelectChoice(")
        );
    }

    #[test]
    fn 変数変化がトレースに記録される() {
        let md = "[SET name=score value=10]\n[SAY speaker=Alice]\nテスト\n";
        let ast = parse(md).unwrap();
        let program = compile(&ast);
        let trace = trace_linear(&program);

        // SET は step 0 の中で実行される
        let changes = &trace.steps[0].state_diff.var_changes;
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].key, "score");
        assert_eq!(changes[0].before, None);
        assert_eq!(changes[0].after, "10");
    }

    #[test]
    fn エンディング到達で終了() {
        let md = "[SAY speaker=Alice]\n終わり\n\n[ENDING id=end1]\n";
        let ast = parse(md).unwrap();
        let program = compile(&ast);
        let trace = trace_linear(&program);

        let last = trace.steps.last().unwrap();
        assert!(
            last.waiting_for
                .as_deref()
                .unwrap_or("")
                .starts_with("Ended(")
        );
    }
}
