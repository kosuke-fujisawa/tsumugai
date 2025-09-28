# API — tsumugai Core ⇄ Host 契約

この文書は、Core が返す **StepResult(JSON)** の契約を定義します。  
**互換性はこの文書と `schemas/stepresult.schema.json` を基準**に保ちます。

---

## 1. 実行サイクル（概要）

- `Engine::step()` が 1 ステップ進め、`StepResult` を返す  
- `next` によりホスト側の待機/選択/終了を指示  
- 分岐時は `Engine::choose(index)` を呼び戻す

---

## 2. Rust 型（参考）

```rust
pub enum NextAction {
    Next,        // 次ステップへ（Enter等）
    WaitUser,    // ユーザー入力待ち（Enter等）
    WaitBranch,  // 分岐選択待ち（choose() 必須）
    Halt,        // 終了
}

#[serde(tag = "type", content = "args")]
pub enum Directive {
    Say { speaker: String, text: String },
    ShowImage { layer: String, path: Option<String> }, // path=None は未解決
    PlayBgm { path: Option<String> },
    Wait { seconds: f32 },
    Branch { choices: Vec<String> },                   // labels は Engine 内部で保持
    ClearLayer { layer: String },
}

pub struct StepResult {
    pub next: NextAction,
    pub directives: Vec<Directive>,
}
```
**禁止**：既存 Directive の意味変更／フィールド削除。  
**許容**：新 Directive の追加、オプショナルフィールド追加。

## 3. JSON 例（dump 出力サンプル）

```json
{
  "next": "WaitBranch",
  "directives": [
    { "type": "Say", "args": { "speaker": "ハル", "text": "どっちに行く？" } },
    { "type": "Branch", "args": { "choices": ["右へ", "左へ"] } }
  ]
}
```

## 4. JSON Schema（抜粋：schemas/stepresult.schema.json）

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "StepResult",
  "type": "object",
  "required": ["next", "directives"],
  "properties": {
    "next": {
      "type": "string",
      "enum": ["Next", "WaitUser", "WaitBranch", "Halt"]
    },
    "directives": {
      "type": "array",
      "items": {
        "oneOf": [
          {
            "type": "object",
            "required": ["type", "args"],
            "properties": {
              "type": { "const": "Say" },
              "args": {
                "type": "object",
                "required": ["speaker", "text"],
                "properties": {
                  "speaker": { "type": "string" },
                  "text": { "type": "string" }
                },
                "additionalProperties": false
              }
            },
            "additionalProperties": false
          },
          {
            "type": "object",
            "required": ["type", "args"],
            "properties": {
              "type": { "const": "ShowImage" },
              "args": {
                "type": "object",
                "required": ["layer"],
                "properties": {
                  "layer": { "type": "string" },
                  "path": { "type": ["string", "null"] }
                },
                "additionalProperties": false
              }
            },
            "additionalProperties": false
          }
          /* … PlayBgm / Wait / Branch / ClearLayer を同様に定義 … */
        ]
      }
    }
  },
  "additionalProperties": false
}
```
**Schema は 後方互換を守る**。新 Directive を追加する場合は oneOf を追加する。

## 5. エラーと警告
- **エラー（パース不能・致命的な構文矛盾）**：`Engine::step()` がエラー型で返す
- **警告（未解決アセット・未定義ラベル）**：`Directive.path=null` などに反映、処理は継続

**必ず 行/列番号 と 修正候補 をログに含めること**

## 6. 受け入れ条件（Core ⇄ Host）
- Core は **同一 .md から同一 JSON を生成する**（決定性）。
- Host は **StepResult を 変換なしに反映できる**（画像/BGM/分岐/待機）。
- API 変更は **Schema/文書→実装→ゴールデン更新** の順で行う。

## 7. 互換性の扱い
- **マイナー**：後方互換の追加（新 Directive 追加、オプショナル項目の追加）
- **メジャー**：既存の意味変更や削除（原則禁止。どうしても必要なら移行ガイドと2段リリース）

## 8. 例：分岐の往復
1.  **Core**: `step()` → `next=WaitBranch`, `Directive::Branch { choices }`
2.  **Host**: UI で選択 → `choose(index)`
3.  **Core**: 次の `step()` で分岐先の `Say`/`ShowImage`/… を返す

---

## 9. 簡易アーキテクチャ APIリファレンス

簡易アーキテクチャは `tsumugai::facade::Facade` 構造体を通じて利用します。
JSONではなく、Rustの構造体を直接やり取りするのが特徴です。

### 主な構造体

- **`Facade`**: `Facade::new(markdown: &str)` で初期化し、`next(&mut State)` でシナリオを1ステップ進めます。
- **`State`**: シナリオの実行状態を保持します。`State::new()` で初期化し、`facade.next()`に渡します。
- **`Output`**: `facade.next()` の戻り値。次にホストアプリが実行すべきことを示します。
- **`Event`**: `Output` に含まれる、発生したイベントのリスト（台詞、BGM再生など）。

### `Output` 構造体 (抜粋)

```rust
pub enum Output {
    /// 次のステップへ進む
    Next,
    /// ユーザーの入力を待つ
    Wait,
    /// シナリオの終端
    Halt,
    /// 選択肢を提示し、ユーザーの選択を待つ
    Branch(Vec<String>),
}
```

### `Event` 構造体 (抜粋)

```rust
pub enum Event {
    /// 台詞を表示
    Say(String, String), // speaker, text
    /// BGMを再生
    PlayBgm(String), // path
    /// 画像を表示
    ShowImage(String, String), // layer, path
    // ... 他のイベント
}
```

### `State` 構造体 (抜粋)

```rust
pub struct State {
    /// 現在の実行位置
    current_index: usize,
    /// ユーザーフラグ
    flags: HashMap<String, String>,
    // ... 他の状態
}
```

### 利用例

```rust
use tsumugai::facade::{Facade, State};
use tsumugai::types::output::Output;

let markdown = "# scene: start\n[SAY speaker=A] こんにちは";
let facade = Facade::new(markdown).unwrap();
let mut state = State::new();

loop {
    let (output, events) = facade.next(&mut state);
    
    // events を元に描画処理...
    
    match output {
        Output::Next | Output::Wait => { /* ユーザー入力待ち */ },
        Output::Branch(choices) => {
            let choice_index = get_user_choice(&choices);
            state.set_choice(choice_index);
        },
        Output::Halt => break,
    }
}
```
