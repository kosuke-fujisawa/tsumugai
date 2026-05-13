//! 抽象構文木（AST）の型定義

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// パース済みシナリオの AST
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ast {
    pub nodes: Vec<AstNode>,
    /// ラベル名 → ノードインデックスのマップ
    pub labels: HashMap<String, usize>,
    /// 宣言済み条件名（:::conditions ブロックから）
    pub conditions: std::collections::HashSet<String>,
}

impl Ast {
    pub fn new(nodes: Vec<AstNode>, labels: HashMap<String, usize>) -> Self {
        Self {
            nodes,
            labels,
            conditions: std::collections::HashSet::new(),
        }
    }

    pub fn with_conditions(
        nodes: Vec<AstNode>,
        labels: HashMap<String, usize>,
        conditions: std::collections::HashSet<String>,
    ) -> Self {
        Self {
            nodes,
            labels,
            conditions,
        }
    }

    pub fn get_label_index(&self, label: &str) -> Option<usize> {
        self.labels.get(label).copied()
    }

    pub fn get_node(&self, index: usize) -> Option<&AstNode> {
        self.nodes.get(index)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// AST の1ノード（コマンド）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AstNode {
    /// 台詞・ナレーション（speaker が空文字ならナレーション）
    Say { speaker: String, text: String },
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
    /// 時間待ち
    Wait { seconds: f32 },
    /// ユーザー選択肢
    Branch { choices: Vec<Choice> },
    /// 無条件ジャンプ
    Jump { label: String },
    /// 条件付きジャンプ（比較演算）
    JumpIf {
        var: String,
        cmp: Comparison,
        value: String,
        label: String,
    },
    /// 変数セット
    Set { name: String, value: String },
    /// 変数演算
    Modify {
        name: String,
        op: Operation,
        value: String,
    },
    /// ラベル定義（実行時 no-op）
    Label { name: String },
    /// シーン境界
    Scene { name: String },
    /// 条件付き実行ブロック
    WhenBlock { condition: Expr, body: Vec<AstNode> },
}

/// 選択肢の1項目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub target: String,
    pub condition: Option<String>,
}

/// 比較演算子
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Comparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// 変数演算の種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// 条件式（WhenBlock および Branch で使用）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Expr {
    Bool(bool),
    Number(i64),
    String(String),
    /// 変数参照
    Var(String),
    Equal(Box<Expr>, Box<Expr>),
    NotEqual(Box<Expr>, Box<Expr>),
    LessThan(Box<Expr>, Box<Expr>),
    LessThanOrEqual(Box<Expr>, Box<Expr>),
    GreaterThan(Box<Expr>, Box<Expr>),
    GreaterThanOrEqual(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
}
