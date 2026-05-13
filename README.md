# tsumugai

tsumugai（つむがい）は、  
**Markdown で書かれたノベルゲーム用シナリオを解析し、  
実行前に意味を確定し、検証し、再利用可能にする  
Rust 製の semantic runtime / シナリオチェッカーライブラリ**です。

描画・音声再生・UI 操作は行いません。  
代わりに、

> **「次に何が起こるのか」「分岐や条件は正しいのか」**

を、構造化されたデータとして返します。

---

## これは何か？（一言で）

> **ノベルゲーム用シナリオのための  
> 検証可能な意味実行基盤**

- ゲームエンジンではありません
- UI や演出は含みません
- Markdown を AST / IR に変換し、意味を確定してから実行します
- 実行・静的検証・将来的なドライラン検証に使えるコアです

詳しい思想と責務境界は [docs/CONCEPT.md](docs/CONCEPT.md) を参照してください。

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

### 2. 「次に起こること」を Event として返却

tsumugai は、  
**描画や再生を行う代わりに「Event（意味イベント）」を返します。**

例：

- このテキストを表示してほしい
- ここで選択肢を出してほしい
- 次はこのラベルにジャンプする
- ユーザー入力を待つ

### 3. チェック・検証

**静的検証（parser::check）**:
- 条件の宣言と使用の整合性チェック
- 未定義条件の使用を警告
- 宣言されたが未使用の条件を警告

**静的解析（analyzer）**:
- 未定義ラベルの検出
- 未参照ラベルの検出
- 選択肢の基本検査

**今後強化する検証**:
- 全分岐ドライラン
- エンディング到達検証
- 無限ループ・最大深度超過の検出
- 修正候補付き Diagnostic

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
[ Event / Diagnostic / DryRunReport ]
↓
あなたのアプリケーション
```

---

## 想定している利用者

- Rust でノベルゲームを作りたいエンジニア
- UI・演出は自分で制御したい開発者
- シナリオを Markdown / Git で管理したいチーム
- LLM でシナリオ生成・修正・レビューを回したい人
- 書いたシナリオが正しいかを自動で確認したい人

完成済みノベルゲームエンジンの代替ではありません。

---

## 最小の使用例

### ライブラリ API

```rust
use tsumugai::{
    parser,
    runtime::{self, Input, WaitingType, ir::Event},
    types::State,
};

fn main() -> anyhow::Result<()> {
    let scenario = r#"
[SAY speaker=Alice]
こんにちは。

[BRANCH choice=進む label=go, choice=戻る label=back]

[LABEL name=go]
[SAY speaker=Alice]
前に進みましょう。

[LABEL name=back]
[SAY speaker=Alice]
戻りましょう。
"#;

    let ast = parser::parse(scenario)?;
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

    Ok(())
}
```

### CLI

```bash
cargo run -- play assets/scenarios/strange_encounter.md
cargo run -- check assets/scenarios/strange_encounter.md
```

- `play`: CUI プレイヤーで実行
- `check`: analyzer で静的検査

---

## シナリオ形式

- 標準的な Markdown ファイル
- 見出し・コメントが使える
- 人間にも LLM にも読みやすい
- 独自タグは最小限

👉 「ライターが書きやすいこと」を最優先

詳細な記法は docs/ を参照してください。

---

## ドキュメント

- [Concept](docs/CONCEPT.md): 存在意義、設計思想、責務境界
- [Architecture](docs/ARCHITECTURE.md): アーキテクチャとデータフロー
- [API](docs/API.md): Core と Host の契約
- [Development Workflow](docs/DEVELOPMENT_WORKFLOW.md): 開発ワークフロー

## 設計方針

- テストファースト（TDD）
- 最小責務・単純な構造
- シナリオ解釈と検証に特化
- ライブラリは静かに振る舞う
- 実装より「意味」を返す

---

## tsumugai が目指すもの

- これさえあればシナリオの正しさを確認できる
- 実行にもチェックにも使える
- LLM フレンドリーなノベルゲーム開発基盤
- Bevy / iOS / Web / CUI などに接続できる意味コア

---

## 開発・CI

### ローカル確認コマンド

PR 前に以下を実行してください。

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI（GitHub Actions）も同じコマンドを実行します。ローカルで通れば CI も通ります。

詳細な開発ワークフローは [docs/DEVELOPMENT_WORKFLOW.md](docs/DEVELOPMENT_WORKFLOW.md) を参照してください。

---

## ライセンス

MIT License
