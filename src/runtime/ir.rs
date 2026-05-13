//! IR（中間表現）定義
//!
//! Markdown → AST → IR の変換で生成される命令列。
//! IR は以下の性質を持つ：
//! - ラベル解決済み（ラベル名 → インデックス）
//! - 分岐先確定済み
//! - 実行時に構造解釈が不要
//!
//! 命令は「進行制御」「状態変更」「イベント生成」の3種類に分類される。

use crate::types::ast::Expr;
use serde::{Deserialize, Serialize};

/// コンパイル済みプログラム（IR命令列）
pub type Program = Vec<Op>;

/// IR 命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    // ──────────────────────────────
    // 進行制御
    // ──────────────────────────────
    /// 無条件ジャンプ（ラベル解決済み）
    Jump { target: usize },

    /// 条件分岐：condition が真なら target へジャンプ、偽なら次の命令へ
    Branch { condition: Expr, target: usize },

    /// 選択肢入力待ち。ここで実行を停止し、プレイヤーに選択肢を提示する
    AwaitChoice { options: Vec<ChoiceOption> },

    /// 進行入力待ち（Enter 待ち）。台詞・ナレーション直後に挿入される
    AwaitAdvance,

    // ──────────────────────────────
    // 状態変更
    // ──────────────────────────────
    /// 変数を値にセット
    Set { key: String, value: String },

    /// 変数を演算で更新
    Modify {
        key: String,
        op: MathOp,
        value: String,
    },

    /// シナリオ終了（エンディング到達後に実行を停止する）
    Halt,

    // ──────────────────────────────
    // イベント生成
    // runtime はこれを Output に積むだけで内容を解釈しない
    // ──────────────────────────────
    /// イベントを Output に積む
    Emit(Event),
}

/// プレイヤー側に通知するイベント
///
/// runtime は Emit(event) を見たら Output.events に積むだけ。
/// 表示・再生の判断はプレイヤー（アダプター）側が行う。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Event {
    /// 台詞またはナレーション（speaker が空文字ならナレーション）
    Say { speaker: String, text: String },

    /// シーン開始
    SceneStart { name: String },

    /// 画像表示
    ShowImage { layer: String, name: String },

    /// レイヤークリア
    ClearLayer { layer: String },

    /// BGM 再生
    PlayBgm { name: String },

    /// SE 再生
    PlaySe { name: String },

    /// ムービー再生
    PlayMovie { name: String },

    /// 時間待ち（実際に sleep するかはプレイヤー側の判断）
    Wait { duration: f32 },

    /// エンディング到達
    Ending { id: String, name: String },

    /// 拡張コマンド
    Custom { tag: String, params: Vec<String> },
}

/// 選択肢の1項目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChoiceOption {
    /// 選択肢ID（`{scene_name}_branch_{branch_index}_choice_{choice_index}` 形式、compile 時に確定）
    pub id: String,
    /// 表示テキスト
    pub label: String,
    /// ジャンプ先の IR インデックス（ラベル解決済み）
    pub target_pc: usize,
}

/// 変数演算の種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
}
