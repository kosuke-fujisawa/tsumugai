//! ランタイム実行エンジン
//!
//! # 責務
//! - AST → IR へのコンパイル（`compile`）
//! - IR + 状態 + 入力 → 次の状態 + 出力（`step`）
//!
//! # 実行モデル
//! ```text
//! Markdown → AST → IR(Program) → step(State, Input) → (State, Output)
//! ```
//!
//! # 設計原則
//! - runtime は IR のみを扱う（Markdown を直接解釈しない）
//! - `Emit` は Output.events に積むだけ（内容を解釈しない）
//! - `AwaitChoice` / `AwaitAdvance` で実行を停止し `waiting_for` をセット
//! - 状態は明示的（隠れた状態を持たない）

pub mod ir;

use crate::types::{
    ast::{Ast, AstNode, Comparison, Expr, Operation},
    state::State,
};
use ir::{ChoiceOption, Event, MathOp, Op, Program};
use std::collections::HashMap;

// ──────────────────────────────────────────────
// 公開 API 型定義
// ──────────────────────────────────────────────

/// runtime への入力
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// 進行（Enter キー相当）
    Advance,
    /// 選択肢を選ぶ（`ChoiceOption.id` を指定）
    SelectChoice(String),
}

/// runtime からの出力
#[derive(Debug, Clone)]
pub struct Output {
    /// この step で発生したイベント列（描画・音など）
    pub events: Vec<Event>,
    /// 入力待ち状態（None なら続けて step を呼んでよい）
    pub waiting_for: Option<WaitingType>,
}

/// 入力待ちの種類
#[derive(Debug, Clone, PartialEq)]
pub enum WaitingType {
    /// Enter 待ち
    Advance,
    /// 選択肢待ち（表示に必要な選択肢一覧を含む）
    Choice(Vec<ChoiceOption>),
    /// エンディング到達（シナリオ終了）
    Ended { id: String, name: String },
}

impl Output {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            waiting_for: None,
        }
    }
}

// ──────────────────────────────────────────────
// コンパイル
// ──────────────────────────────────────────────

/// AST を IR 命令列（Program）にコンパイルする
///
/// - ラベルを IR インデックスに解決する
/// - 選択肢 ID を `{scene_name}_choice_{index}` 形式で確定する
/// - 台詞・ナレーション直後に `AwaitAdvance` を自動挿入する
pub fn compile(ast: &Ast) -> Program {
    let mut compiler = Compiler::new();
    compiler.compile_nodes(&ast.nodes);
    compiler.resolve()
}

// ──────────────────────────────────────────────
// コンパイラ実装（内部）
// ──────────────────────────────────────────────

/// コンパイル中の未解決命令
enum PreOp {
    Resolved(Op),
    LabelDef(String),
    Jump(String),
    Branch { condition: Expr, label: String },
    AwaitChoice { choices: Vec<PreChoice> },
}

/// 未解決の選択肢項目
struct PreChoice {
    id: String,
    label: String,
    target_label: String,
    condition: Option<String>,
}

struct Compiler {
    ops: Vec<PreOp>,
    current_scene: String,
    /// WhenBlock のスキップラベル生成用グローバルカウンター
    label_counter: usize,
    /// BRANCH ごとの通し番号（同一シーン内の Choice ID 衝突を防ぐ）
    branch_counter: usize,
}

impl Compiler {
    fn new() -> Self {
        Self {
            ops: Vec::new(),
            current_scene: "root".to_string(),
            label_counter: 0,
            branch_counter: 0,
        }
    }

    fn compile_nodes(&mut self, nodes: &[AstNode]) {
        for node in nodes {
            self.compile_node(node);
        }
    }

    fn compile_node(&mut self, node: &AstNode) {
        match node {
            AstNode::Say { speaker, text } => {
                self.ops.push(PreOp::Resolved(Op::Emit(Event::Say {
                    speaker: speaker.clone(),
                    text: text.clone(),
                })));
                // 台詞・ナレーション直後は必ず Enter 待ち
                self.ops.push(PreOp::Resolved(Op::AwaitAdvance));
            }

            AstNode::Scene { name } => {
                self.current_scene = name.clone();
                self.ops.push(PreOp::Resolved(Op::Emit(Event::SceneStart {
                    name: name.clone(),
                })));
            }

            AstNode::ShowImage { layer, name } => {
                self.ops.push(PreOp::Resolved(Op::Emit(Event::ShowImage {
                    layer: layer.clone(),
                    name: name.clone(),
                })));
            }

            AstNode::ClearLayer { layer } => {
                self.ops.push(PreOp::Resolved(Op::Emit(Event::ClearLayer {
                    layer: layer.clone(),
                })));
            }

            AstNode::PlayBgm { name } => {
                self.ops.push(PreOp::Resolved(Op::Emit(Event::PlayBgm {
                    name: name.clone(),
                })));
            }

            AstNode::PlaySe { name } => {
                self.ops.push(PreOp::Resolved(Op::Emit(Event::PlaySe {
                    name: name.clone(),
                })));
            }

            AstNode::PlayMovie { name } => {
                self.ops.push(PreOp::Resolved(Op::Emit(Event::PlayMovie {
                    name: name.clone(),
                })));
            }

            AstNode::Wait { seconds } => {
                self.ops.push(PreOp::Resolved(Op::Emit(Event::Wait {
                    duration: *seconds,
                })));
            }

            AstNode::Label { name } => {
                self.ops.push(PreOp::LabelDef(name.clone()));
            }

            AstNode::Jump { label } => {
                self.ops.push(PreOp::Jump(label.clone()));
            }

            AstNode::JumpIf {
                var,
                cmp,
                value,
                label,
            } => {
                let condition = make_cmp_expr(var, cmp, value);
                self.ops.push(PreOp::Branch {
                    condition,
                    label: label.clone(),
                });
            }

            AstNode::Set { name, value } => {
                self.ops.push(PreOp::Resolved(Op::Set {
                    key: name.clone(),
                    value: value.clone(),
                }));
            }

            AstNode::Modify { name, op, value } => {
                let math_op = match op {
                    Operation::Add => MathOp::Add,
                    Operation::Subtract => MathOp::Sub,
                    Operation::Multiply => MathOp::Mul,
                    Operation::Divide => MathOp::Div,
                };
                self.ops.push(PreOp::Resolved(Op::Modify {
                    key: name.clone(),
                    op: math_op,
                    value: value.clone(),
                }));
            }

            AstNode::Branch { choices } => {
                let scene = self.current_scene.clone();
                let branch_idx = self.branch_counter;
                self.branch_counter += 1;
                let pre_choices = choices
                    .iter()
                    .enumerate()
                    .map(|(i, c)| PreChoice {
                        id: format!("{}_branch_{}_choice_{}", scene, branch_idx, i),
                        label: c.label.clone(),
                        target_label: c.target.clone(),
                        condition: c.condition.clone(),
                    })
                    .collect();
                self.ops.push(PreOp::AwaitChoice {
                    choices: pre_choices,
                });
            }

            AstNode::Ending { id, name } => {
                let display_name = name.clone().unwrap_or_else(|| id.clone());
                self.ops.push(PreOp::Resolved(Op::Emit(Event::Ending {
                    id: id.clone(),
                    name: display_name,
                })));
                self.ops.push(PreOp::Resolved(Op::Halt));
            }

            AstNode::WhenBlock { condition, body } => {
                // 条件が偽のときボディをスキップするジャンプを挿入
                // ops.len() ではなくグローバルカウンターを使って衝突を防ぐ
                let skip_label = format!("__when_end_{}", self.label_counter);
                self.label_counter += 1;
                self.ops.push(PreOp::Branch {
                    condition: Expr::Not(Box::new(condition.clone())),
                    label: skip_label.clone(),
                });
                self.compile_nodes(body);
                self.ops.push(PreOp::LabelDef(skip_label));
            }
        }
    }

    /// ラベルを解決して最終的な Program を生成する
    fn resolve(self) -> Program {
        // パス1: ラベル位置マップを構築（LabelDef を除いたインデックスで計算）
        let mut label_map: HashMap<String, usize> = HashMap::new();
        let mut ir_index: usize = 0;
        for pre_op in &self.ops {
            match pre_op {
                PreOp::LabelDef(name) => {
                    label_map.insert(name.clone(), ir_index);
                }
                _ => {
                    ir_index += 1;
                }
            }
        }

        // パス2: 解決して Program を生成
        let mut program: Program = Vec::new();
        for pre_op in self.ops {
            match pre_op {
                PreOp::LabelDef(_) => {}
                PreOp::Resolved(op) => {
                    program.push(op);
                }
                PreOp::Jump(label) => {
                    let target = label_map.get(&label).copied().unwrap_or(program.len());
                    program.push(Op::Jump { target });
                }
                PreOp::Branch { condition, label } => {
                    let target = label_map.get(&label).copied().unwrap_or(program.len());
                    program.push(Op::Branch { condition, target });
                }
                PreOp::AwaitChoice { choices } => {
                    let options = choices
                        .into_iter()
                        .map(|c| {
                            let target_pc = label_map
                                .get(&c.target_label)
                                .copied()
                                .unwrap_or(program.len());
                            ChoiceOption {
                                id: c.id,
                                label: c.label,
                                target_pc,
                                condition: c.condition.map(Expr::Var),
                            }
                        })
                        .collect();
                    program.push(Op::AwaitChoice { options });
                }
            }
        }

        program
    }
}

/// 比較演算子から Expr を生成するヘルパー
fn make_cmp_expr(var: &str, cmp: &Comparison, value: &str) -> Expr {
    let lhs = Box::new(Expr::Var(var.to_string()));
    let rhs = Box::new(Expr::String(value.to_string()));
    match cmp {
        Comparison::Equal => Expr::Equal(lhs, rhs),
        Comparison::NotEqual => Expr::NotEqual(lhs, rhs),
        Comparison::LessThan => Expr::LessThan(lhs, rhs),
        Comparison::LessThanOrEqual => Expr::LessThanOrEqual(lhs, rhs),
        Comparison::GreaterThan => Expr::GreaterThan(lhs, rhs),
        Comparison::GreaterThanOrEqual => Expr::GreaterThanOrEqual(lhs, rhs),
    }
}

// ──────────────────────────────────────────────
// 実行エンジン
// ──────────────────────────────────────────────

/// 1ステップ実行する
///
/// # 動作
/// 1. 入力がある場合、現在位置の `Await*` 命令を解決して PC を進める
/// 2. `Await*` に到達するか、プログラム末尾に達するまで Op を処理し続ける
pub fn step(mut state: State, program: &Program, input: Option<Input>) -> (State, Output) {
    let mut output = Output::new();

    // 入力処理
    if let Some(ref input) = input {
        match input {
            Input::Advance => {
                if let Some(Op::AwaitAdvance) = program.get(state.pc) {
                    state.pc += 1;
                }
            }
            Input::SelectChoice(id) => {
                if let Some(Op::AwaitChoice { options }) = program.get(state.pc)
                    && let Some(opt) = options.iter().find(|o| {
                        &o.id == id
                            && o.condition
                                .as_ref()
                                .is_none_or(|cond| eval_expr(cond, &state))
                    })
                {
                    state.pc = opt.target_pc;
                }
            }
        }
    }

    // Op 処理ループ
    while let Some(op) = program.get(state.pc) {
        match op {
            Op::Emit(event) => {
                output.events.push(event.clone());
                state.pc += 1;
            }

            Op::AwaitAdvance => {
                output.waiting_for = Some(WaitingType::Advance);
                break;
            }

            Op::AwaitChoice { options } => {
                let visible: Vec<ChoiceOption> = options
                    .iter()
                    .filter(|o| {
                        o.condition
                            .as_ref()
                            .is_none_or(|cond| eval_expr(cond, &state))
                    })
                    .cloned()
                    .collect();
                output.waiting_for = Some(WaitingType::Choice(visible));
                break;
            }

            Op::Jump { target } => {
                state.pc = *target;
            }

            Op::Branch { condition, target } => {
                if eval_expr(condition, &state) {
                    state.pc = *target;
                } else {
                    state.pc += 1;
                }
            }

            Op::Halt => {
                let ended = output.events.iter().find_map(|e| {
                    if let Event::Ending { id, name } = e {
                        Some((id.clone(), name.clone()))
                    } else {
                        None
                    }
                });
                let (id, name) = ended.unwrap_or_default();
                output.waiting_for = Some(WaitingType::Ended { id, name });
                break;
            }

            Op::Set { key, value } => {
                state.set_var(key.clone(), value.clone());
                state.pc += 1;
            }

            Op::Modify { key, op, value } => {
                let ast_op = match op {
                    MathOp::Add => Operation::Add,
                    MathOp::Sub => Operation::Subtract,
                    MathOp::Mul => Operation::Multiply,
                    MathOp::Div => Operation::Divide,
                };
                if let Err(e) = state.modify_var(key, ast_op, value) {
                    output.events.push(Event::Custom {
                        tag: "error".to_string(),
                        params: vec![format!("modify_var failed for '{}': {}", key, e)],
                    });
                }
                state.pc += 1;
            }
        }
    }

    (state, output)
}

// ──────────────────────────────────────────────
// 式評価
// ──────────────────────────────────────────────

fn eval_expr(expr: &Expr, state: &State) -> bool {
    match expr {
        Expr::Bool(b) => *b,
        Expr::Number(n) => *n != 0,
        Expr::String(s) => !s.is_empty(),
        Expr::Var(name) => match state.get_var(name) {
            Some(v) => !v.is_empty() && v != "false" && v != "0",
            None => false,
        },
        Expr::Equal(lhs, rhs) => eval_str(lhs, state) == eval_str(rhs, state),
        Expr::NotEqual(lhs, rhs) => eval_str(lhs, state) != eval_str(rhs, state),
        Expr::LessThan(lhs, rhs) => eval_num(lhs, state) < eval_num(rhs, state),
        Expr::LessThanOrEqual(lhs, rhs) => eval_num(lhs, state) <= eval_num(rhs, state),
        Expr::GreaterThan(lhs, rhs) => eval_num(lhs, state) > eval_num(rhs, state),
        Expr::GreaterThanOrEqual(lhs, rhs) => eval_num(lhs, state) >= eval_num(rhs, state),
        Expr::And(lhs, rhs) => eval_expr(lhs, state) && eval_expr(rhs, state),
        Expr::Or(lhs, rhs) => eval_expr(lhs, state) || eval_expr(rhs, state),
        Expr::Not(inner) => !eval_expr(inner, state),
    }
}

fn eval_str(expr: &Expr, state: &State) -> String {
    match expr {
        Expr::Bool(b) => b.to_string(),
        Expr::Number(n) => n.to_string(),
        Expr::String(s) => s.clone(),
        Expr::Var(name) => state.get_var(name).unwrap_or_default(),
        _ => {
            if eval_expr(expr, state) {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
    }
}

fn eval_num(expr: &Expr, state: &State) -> f64 {
    eval_str(expr, state).parse::<f64>().unwrap_or(0.0)
}

// ──────────────────────────────────────────────
// テスト
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser::parse, types::state::State};

    #[test]
    fn 台詞のコンパイルと実行() {
        let md = "[SAY speaker=Alice]\nこんにちは！\n";
        let ast = parse(md).unwrap();
        let program = compile(&ast);

        // Say + AwaitAdvance = 2 命令
        assert_eq!(program.len(), 2);
        assert!(matches!(program[0], Op::Emit(Event::Say { .. })));
        assert!(matches!(program[1], Op::AwaitAdvance));

        // step(None) → Say イベントが来て AwaitAdvance で停止
        let state = State::new();
        let (state, output) = step(state, &program, None);
        assert_eq!(output.events.len(), 1);
        assert!(matches!(output.waiting_for, Some(WaitingType::Advance)));
        assert_eq!(state.pc, 1);

        // step(Advance) → AwaitAdvance を抜けてプログラム末尾へ
        let (state, output) = step(state, &program, Some(Input::Advance));
        assert!(output.waiting_for.is_none());
        assert_eq!(state.pc, 2);
    }

    #[test]
    fn 選択肢のコンパイルと実行() {
        let md = r#"
[BRANCH choice=はい label=yes, choice=いいえ label=no]

[LABEL name=yes]
[SAY speaker=Alice]
選んだね。

[LABEL name=no]
[SAY speaker=Alice]
選ばなかったね。
"#;
        let ast = parse(md).unwrap();
        let program = compile(&ast);

        // AwaitChoice で停止する
        let state = State::new();
        let (state, output) = step(state, &program, None);
        assert!(matches!(output.waiting_for, Some(WaitingType::Choice(_))));

        // 最初の選択肢を選ぶ
        let choice_id = if let Some(WaitingType::Choice(opts)) = &output.waiting_for {
            opts[0].id.clone()
        } else {
            panic!("選択肢が返されなかった");
        };

        let (_, output2) = step(state, &program, Some(Input::SelectChoice(choice_id)));
        assert!(
            output2
                .events
                .iter()
                .any(|e| matches!(e, Event::Say { .. }))
        );
    }

    #[test]
    fn 変数セットと条件分岐() {
        let md = r#"
[SET name=score value=10]
[JUMP_IF var=score cmp=ge value=5 label=pass]
[SAY speaker=Alice]
失敗。

[LABEL name=pass]
[SAY speaker=Alice]
合格！
"#;
        let ast = parse(md).unwrap();
        let program = compile(&ast);

        let state = State::new();
        let (_, output) = step(state, &program, None);

        // score >= 5 なので「合格！」の台詞が来るはず
        assert!(output.events.iter().any(|e| {
            if let Event::Say { text, .. } = e {
                text.contains("合格")
            } else {
                false
            }
        }));
    }
}
