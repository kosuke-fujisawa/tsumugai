# tsumugai

tsumugai（つむがい）は、  
**Markdown で書かれたノベルゲーム用シナリオを解析し、  
「物語がどう進行するか」を順に解釈・検証する  
Rust 製のシナリオパース＆チェッカーライブラリ**です。

描画・音声再生・UI 操作は行いません。  
代わりに、

> **「次に何が起こるのか」「指定は足りているのか」**

を、構造化されたデータとして返します。

---

## これは何か？（一言で）

> **ノベルゲーム用シナリオのための  
> シナリオチェッカー付き多機能パースエンジン**

- ゲームエンジンではありません
- UI や演出は含みません
- シナリオを **上から順に解釈するステートマシン**です
- 実行もでき、**ドライラン（検証）にも使えます**

---

## tsumugai がやること

tsumugai は Markdown 形式のシナリオを読み取り、  
以下を **順次・一貫して処理**します。

### 1. シナリオの解釈（パース）

- テキスト（セリフ／ナレーション）
- 選択肢
- 分岐・ジャンプ
- 条件付き分岐（フラグ・論理式）
- シナリオ進行上の状態

### 2. 「次に起こること」を Directive として返却

tsumugai は、  
**描画や再生を行う代わりに「Directive（指示）」を返します。**

例：

- このテキストを表示してほしい
- ここで選択肢を出してほしい
- 次はこのラベルにジャンプする
- ユーザー入力を待つ

### 3. チェック・検証（ドライラン）

**静的検証（parser::check）**:
- 条件の宣言と使用の整合性チェック
- 未定義条件の使用を警告
- 宣言されたが未使用の条件を警告

**包括的 Linter（lint モジュール）**:
- **構文チェック**: コマンドパラメータの検証
- **参照整合性**: 未定義ラベル・条件の検出
- **品質チェック**: 連続WAIT、重複BGM、長文テキストの警告
- **フロー分析**: 到達不能コード、無限ループの検出

**デバッグログ（runtime::step_with_debug）**:
- プログラムカウンタ、実行ステップの詳細ログ
- 分岐・ジャンプの追跡
- 変数状態の監視
- 環境変数 `TSUMUGAI_DEBUG` で有効化

👉 **「動かさなくても問題点が分かる」**のが特徴です。

---

## tsumugai がやらないこと（重要）

tsumugai は以下を **一切行いません**。

- テキスト・画像の描画
- 音声・BGM・SE の再生
- UI 操作・演出制御
- アニメーション

tsumugai の責務は、あくまで  
**「物語の意味と進行を決め、検証すること」**です。

```text
[ Markdown シナリオ ]
↓
tsumugai
↓
[ 解釈結果 / Directive / ワーニング ]
↓
あなたのアプリケーション
```

---

想定している利用者
	•	Rust でノベルゲームを作りたいエンジニア
	•	UI・演出は自分で制御したい
	•	シナリオを Markdown / Git 管理したい
	•	LLM（ChatGPT / Claude 等）で
シナリオ生成・修正・レビューを回したい
	•	「書いたシナリオが正しいか」を自動で確認したい

完成済みノベルゲームエンジンの代替ではありません。

⸻

## 最小の使用例（新API）

### シンプルAPI（推奨）

```rust
use tsumugai::{parser, runtime, types::{State, Event}};

fn main() -> anyhow::Result<()> {
    let scenario = r#"
[SAY speaker=Alice]
こんにちは。

[BRANCH choice=進む choice=戻る]

[LABEL name=進む]
[SAY speaker=Alice]
前に進みましょう。

[LABEL name=戻る]
[SAY speaker=Alice]
戻りましょう。
"#;

    // 1. パース
    let ast = parser::parse(scenario)?;

    // 2. 初期状態
    let mut state = State::new();

    // 3. ステップ実行
    loop {
        let (new_state, output) = runtime::step(state, &ast, None);
        state = new_state;

        // セリフを表示
        for line in &output.lines {
            if let Some(speaker) = &line.speaker {
                println!("{}: {}", speaker, line.text);
            } else {
                println!("{}", line.text);
            }
        }

        // 選択肢がある場合
        if !output.choices.is_empty() {
            for (i, choice) in output.choices.iter().enumerate() {
                println!("  {}. {}", i + 1, choice.label);
            }

            // ユーザー入力（ここでは固定で最初を選択）
            let event = Event::Choice { id: "choice_0".to_string() };
            let (new_state, _) = runtime::step(state, &ast, Some(event));
            state = new_state;
        }

        // プログラム終了チェック
        if state.pc >= ast.len() {
            break;
        }
    }

    Ok(())
}
```

### 従来API（互換性維持）

```rust
use tsumugai::application::engine::Engine;

fn main() -> anyhow::Result<()> {
    let scenario = r#"
[SAY speaker=Alice]
こんにちは。
"#;

    let mut engine = Engine::from_markdown(scenario)?;

    let result = engine.step()?;

    for directive in &result.directives {
        println!("{:?}", directive);
    }

    Ok(())
}
```

	•	描画はすべて println!
	•	tsumugai は 「何をすべきか」だけを返す
	•	実際の表示・演出はアプリ側の責務

⸻

## シナリオ形式
	•	標準的な Markdown ファイル
	•	見出し・コメントが使える
	•	人間にも LLM にも読みやすい
	•	独自タグは最小限

👉 「ライターが書きやすいこと」を最優先

詳細な記法は docs/ を参照してください。

⸻

## 設計方針
	•	テストファースト（TDD）
	•	最小責務・単純な構造
	•	シナリオ解釈と検証に特化
	•	ライブラリは静かに振る舞う
	•	実装より「意味」を返す

⸻

## tsumugai が目指すもの
	•	これさえあれば シナリオの正しさは確認できる
	•	実行にもチェックにも使える
	•	LLM フレンドリーなノベルゲーム開発基盤

⸻

ライセンス

MIT License