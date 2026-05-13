# API — tsumugai Core ⇄ Host 契約

この文書は、Core が返す **Output** および **Event** の契約を定義します。

---

## 1. 実行サイクル（概要）

```text
parser::parse(markdown)
  → Ast
  → runtime::compile(&ast)
  → Program (IR命令列)
  → runtime::step(state, &program, input)
  → (State, Output)
```

`runtime::step` を繰り返すことでシナリオを進行させます。

- `Output.waiting_for` が `Some` のとき、ホスト側は入力を用意してから次の `step` を呼ぶ
- `Output.waiting_for` が `None` のとき、そのまま次の `step` を呼んでよい
- プログラム末尾に達すると `waiting_for = None` のまま返り続ける（終了判定はホスト側で行う）

---

## 2. Rust 型（公開API）

```rust
// runtime::Input — step への入力
pub enum Input {
    Advance,                    // 進行（Enterキー相当）
    SelectChoice(String),       // 選択肢を選ぶ（ChoiceOption.id を渡す）
}

// runtime::Output — step の戻り値
pub struct Output {
    pub events: Vec<Event>,                 // 発生したイベント列
    pub waiting_for: Option<WaitingType>,   // 入力待ち状態
}

// runtime::WaitingType — 入力待ちの種類
pub enum WaitingType {
    Advance,                        // Enter 待ち
    Choice(Vec<ChoiceOption>),      // 選択肢待ち
}

// runtime::ir::ChoiceOption — 選択肢の1項目
pub struct ChoiceOption {
    pub id: String,         // 選択肢ID（compile時に確定）
    pub label: String,      // 表示テキスト
    pub target_pc: usize,   // ジャンプ先IRインデックス（内部用）
}

// runtime::ir::Event — ホストに通知するイベント
pub enum Event {
    Say { speaker: String, text: String },
    SceneStart { name: String },
    ShowImage { layer: String, name: String },
    ClearLayer { layer: String },
    PlayBgm { name: String },
    PlaySe { name: String },
    PlayMovie { name: String },
    Wait { duration: f32 },
    Custom { tag: String, params: Vec<String> },
}

// types::State — シナリオの実行状態
pub struct State {
    pub pc: usize,                              // IR上の現在位置
    pub flags: HashMap<String, serde_json::Value>, // 変数・フラグ
}
```

---

## 3. 典型的な使い方

```rust
use tsumugai::{
    parser,
    runtime::{self, Input, WaitingType, ir::Event},
    types::State,
};

let ast = parser::parse(markdown)?;
let program = runtime::compile(&ast);
let mut state = State::new();
let mut input = None;

loop {
    let (new_state, output) = runtime::step(state, &program, input.take());
    state = new_state;

    for event in &output.events {
        if let Event::Say { speaker, text } = event {
            println!("{}: {}", speaker, text);
        }
    }

    match output.waiting_for {
        Some(WaitingType::Advance) => {
            input = Some(Input::Advance);
        }
        Some(WaitingType::Choice(options)) => {
            let choice_id = options[0].id.clone();
            input = Some(Input::SelectChoice(choice_id));
        }
        None => break,
    }
}
```

---

## 4. 静的解析 API

```rust
use tsumugai::analyzer::{self, Issue, Level};

let ast = parser::parse(markdown)?;
let result = analyzer::analyze(&ast);

for issue in &result.issues {
    match issue.level {
        Level::Error   => eprintln!("[ERROR] {}", issue.message),
        Level::Warning => eprintln!("[WARN]  {}", issue.message),
        Level::Info    => eprintln!("[INFO]  {}", issue.message),
    }
}

if result.has_errors() {
    // シナリオに問題あり
}
```

検査内容：
- 未定義ラベルへのジャンプ
- 到達不能なラベル（情報として報告）
- 選択肢に対応するラベルが存在しない
- 空または1択の BRANCH

---

## 5. JSON 出力例（serde でシリアライズした場合）

### Output（台詞ステップ）

```json
{
  "events": [
    { "Say": { "speaker": "Alice", "text": "こんにちは。" } }
  ],
  "waiting_for": { "Advance": null }
}
```

### Output（選択肢ステップ）

```json
{
  "events": [],
  "waiting_for": {
    "Choice": [
      { "id": "root_branch_0_choice_0", "label": "はい", "target_pc": 3 },
      { "id": "root_branch_0_choice_1", "label": "いいえ", "target_pc": 6 }
    ]
  }
}
```

### State

```json
{
  "pc": 4,
  "flags": {
    "score": 10,
    "player_name": "Alice"
  }
}
```

---

## 6. エラーと警告

- **パースエラー**：`parser::parse()` が `Err` を返す（`anyhow::Error`）
- **静的検査の問題**：`analyzer::analyze()` が `AnalysisResult` を返す（`Vec<Issue>`）
- **実行時エラー**：現時点では `step` は `Result` を返さない。変数演算の失敗は `Event::Custom { tag: "error", .. }` としてイベント列に積まれる

---

## 7. 互換性の考え方

- **後方互換の変更（許容）**：新 Event バリアント追加、`Event` / `ChoiceOption` へのオプショナルフィールド追加
- **破壊的変更（要調整）**：既存 Event バリアントの意味変更・削除、`Output` / `State` の必須フィールド変更

---

## 8. Roadmap（未実装）

以下は現時点では実装されていません。

- `runtime::step_with_trace` — Trace 付き実行
- `tsumugai check --json` — JSON 形式の診断出力
- 構造化 Diagnostic（`rule_id`, `span`, `suggestion` を持つ型）
- Dry Run / 全分岐探索
- エンディング到達検証
