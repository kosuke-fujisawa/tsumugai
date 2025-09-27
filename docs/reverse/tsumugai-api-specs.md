# tsumugai API仕様書（逆生成）

## 分析日時
2025-09-27

## API概要
tsumugaiは、ライブラリAPIとして公開されるRustクレートです。HTTPエンドポイントは提供せず、Rust関数呼び出しベースのAPIを提供します。

## 公開API契約（破壊変更禁止）

### Engine API

#### `Engine::from_markdown(src: &str) -> Result<Self, ApiError>`
**説明**: Markdown台本からEngineインスタンスを作成

**パラメータ**:
```rust
src: &str  // Markdown形式の台本文字列
```

**戻り値**:
```rust
Result<Engine, ApiError>
```

**使用例**:
```rust
let markdown = r#"
[SAY speaker=Ayumi]
Hello, world!
"#;
let engine = Engine::from_markdown(markdown)?;
```

**エラー**:
- `ApiError::Parse` - Markdown構文エラー、未定義ラベル等

---

#### `Engine::from_markdown_with_resolver(src: &str, resolver: Box<dyn Resolver>) -> Result<Self, ApiError>`
**説明**: アセットリゾルバー付きでEngineを作成

**パラメータ**:
```rust
src: &str                    // Markdown台本
resolver: Box<dyn Resolver>  // アセット解決戦略
```

**戻り値**:
```rust
Result<Engine, ApiError>
```

---

#### `Engine::step(&mut self) -> Result<StepResult, ApiError>`
**説明**: 台本の次のステップを実行

**戻り値**:
```rust
Result<StepResult, ApiError>
```

**StepResult構造**:
```rust
pub struct StepResult {
    pub next: NextAction,        // 次に取るべきアクション
    pub directives: Vec<Directive>, // 実行すべき指令リスト
}
```

**実行パターン**:
```rust
loop {
    let result = engine.step()?;

    // Directiveを処理
    for directive in &result.directives {
        match directive {
            Directive::Say { speaker, text } => {
                println!("{}: {}", speaker, text);
            }
            // その他の処理...
        }
    }

    // NextActionに応じた制御
    match result.next {
        NextAction::Next => continue,
        NextAction::WaitUser => {
            // Enterキー等の入力待ち
            wait_for_input();
        }
        NextAction::WaitBranch => {
            // 分岐選択肢表示、choose()呼び出し
            break;
        }
        NextAction::Halt => break,
    }
}
```

---

#### `Engine::choose(&mut self, index: usize) -> Result<(), ApiError>`
**説明**: 分岐選択肢を選択

**パラメータ**:
```rust
index: usize  // 選択肢のインデックス（0始まり）
```

**戻り値**:
```rust
Result<(), ApiError>
```

**エラー**:
- `ApiError::Invalid` - 選択肢が利用できない、インデックス範囲外

**使用例**:
```rust
// BRANCH実行後
let result = engine.step()?;
if result.next == NextAction::WaitBranch {
    // ユーザーに選択肢を表示
    engine.choose(0)?;  // 最初の選択肢を選択
}
```

---

#### `Engine::get_var(&self, name: &str) -> Option<String>`
**説明**: 台本変数の値を取得

**パラメータ**:
```rust
name: &str  // 変数名
```

**戻り値**:
```rust
Option<String>  // 変数値（文字列形式）、未定義の場合None
```

---

#### `Engine::set_var(&mut self, name: &str, value: &str)`
**説明**: 台本変数の値を設定

**パラメータ**:
```rust
name: &str   // 変数名
value: &str  // 設定値（自動型推論）
```

## データ型定義

### NextAction
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NextAction {
    /// 即座に次のステップへ進む
    Next,
    /// ユーザー入力（Enterキー等）を待つ
    WaitUser,
    /// ユーザーの分岐選択を待つ
    WaitBranch,
    /// 台本実行終了
    Halt,
}
```

### Directive
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "args")]
#[non_exhaustive]
pub enum Directive {
    /// セリフ表示
    Say {
        speaker: String,  // 話者名
        text: String      // セリフ内容
    },

    /// 画像表示
    ShowImage {
        layer: String,           // レイヤー名
        path: Option<String>     // 画像パス（未解決の場合None）
    },

    /// BGM再生
    PlayBgm {
        path: Option<String>     // BGMパス（未解決の場合None）
    },

    /// 効果音再生
    PlaySe {
        path: Option<String>     // SEパス（未解決の場合None）
    },

    /// 動画再生
    PlayMovie {
        path: Option<String>     // 動画パス（未解決の場合None）
    },

    /// 待機
    Wait {
        seconds: f32             // 待機秒数
    },

    /// 分岐選択肢提示
    Branch {
        choices: Vec<String>     // 選択肢文字列リスト
    },

    /// レイヤークリア
    ClearLayer {
        layer: String            // クリア対象レイヤー
    },

    /// 変数設定
    SetVar {
        name: String,            // 変数名
        value: String            // 設定値
    },

    /// ラベルジャンプ
    JumpTo {
        label: String            // ジャンプ先ラベル名
    },

    /// ラベル到達通知
    ReachedLabel {
        label: String            // 到達したラベル名
    },
}
```

### ApiError
```rust
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    /// パースエラー（位置情報付き）
    #[error("parse error at {line}:{column}: {message}")]
    Parse {
        line: usize,     // エラー行
        column: usize,   // エラー列
        message: String, // エラーメッセージ
    },

    /// 無効な操作
    #[error("invalid operation: {0}")]
    Invalid(String),

    /// エンジン実行エラー
    #[error("engine error: {0}")]
    Engine(String),

    /// I/Oエラー
    #[error("I/O error: {0}")]
    Io(String),
}
```

### StepResult
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StepResult {
    /// 次に取るべきアクション
    pub next: NextAction,
    /// 実行すべき指令リスト（順序保証）
    pub directives: Vec<Directive>,
}
```

## Resolver拡張API

### Resolver trait
```rust
pub trait Resolver {
    /// BGMファイルのパス解決
    fn resolve_bgm(&self, logical: &str) -> Option<PathBuf>;

    /// 効果音ファイルのパス解決
    fn resolve_se(&self, logical: &str) -> Option<PathBuf> {
        None  // デフォルト実装
    }

    /// 画像ファイルのパス解決
    fn resolve_image(&self, logical: &str) -> Option<PathBuf> {
        None  // デフォルト実装
    }

    /// 動画ファイルのパス解決
    fn resolve_movie(&self, logical: &str) -> Option<PathBuf> {
        None  // デフォルト実装
    }
}
```

**実装例**:
```rust
struct CustomResolver;

impl Resolver for CustomResolver {
    fn resolve_bgm(&self, logical: &str) -> Option<PathBuf> {
        match logical {
            "intro" => Some(PathBuf::from("assets/bgm/intro.mp3")),
            "battle" => Some(PathBuf::from("assets/bgm/battle.ogg")),
            _ => None,
        }
    }
}

let engine = Engine::from_markdown_with_resolver(
    markdown,
    Box::new(CustomResolver)
)?;
```

## JSON シリアライゼーション形式

### StepResult JSON例
```json
{
  "next": "wait_user",
  "directives": [
    {
      "type": "say",
      "args": {
        "speaker": "Ayumi",
        "text": "Hello, world!"
      }
    }
  ]
}
```

### 分岐選択例
```json
{
  "next": "wait_branch",
  "directives": [
    {
      "type": "branch",
      "args": {
        "choices": ["左へ行く", "右へ行く", "戻る"]
      }
    }
  ]
}
```

### 複数Directive例
```json
{
  "next": "wait_user",
  "directives": [
    {
      "type": "play_bgm",
      "args": {
        "path": "assets/bgm/intro.mp3"
      }
    },
    {
      "type": "show_image",
      "args": {
        "layer": "background",
        "path": "assets/images/classroom.png"
      }
    },
    {
      "type": "say",
      "args": {
        "speaker": "Teacher",
        "text": "授業を始めます。"
      }
    }
  ]
}
```

## エラーレスポンス例

### パースエラー
```rust
ApiError::Parse {
    line: 5,
    column: 12,
    message: "Undefined label 'unknown_label'".to_string()
}
```

### 実行エラー
```rust
ApiError::Invalid("No choices available".to_string())
```

## API使用パターン

### 基本的な実行ループ
```rust
let mut engine = Engine::from_markdown(markdown)?;

loop {
    match engine.step() {
        Ok(result) => {
            // Directive処理
            process_directives(&result.directives);

            match result.next {
                NextAction::Next => continue,
                NextAction::WaitUser => {
                    wait_for_enter();
                }
                NextAction::WaitBranch => {
                    let choice = show_choices_and_wait(&result.directives);
                    engine.choose(choice)?;
                }
                NextAction::Halt => break,
            }
        }
        Err(e) => {
            eprintln!("Engine error: {}", e);
            break;
        }
    }
}
```

### 分岐処理パターン
```rust
if result.next == NextAction::WaitBranch {
    // 分岐Directiveから選択肢を抽出
    if let Some(Directive::Branch { choices }) = result.directives.first() {
        println!("選択してください:");
        for (i, choice) in choices.iter().enumerate() {
            println!("{}. {}", i + 1, choice);
        }

        let input = read_user_input();
        if let Ok(index) = input.parse::<usize>() {
            if index > 0 && index <= choices.len() {
                engine.choose(index - 1)?;
            }
        }
    }
}
```

### 変数操作パターン
```rust
// 変数設定
engine.set_var("score", "100");
engine.set_var("player_name", "太郎");

// 変数取得
if let Some(score) = engine.get_var("score") {
    println!("現在のスコア: {}", score);
}
```

## 決定性保証

tsumugai APIは**決定的実行**を保証します：
- 同一のMarkdown入力に対して、同一のStepResult列を出力
- 変数状態、分岐選択の順序が実行結果に影響
- アセット解決結果は非決定的要素（ファイル存在状況等）に依存

この特性により、台本の動作をテスト・検証・再現可能です。