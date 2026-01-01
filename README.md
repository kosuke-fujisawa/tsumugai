# tsumugai

tsumugai（つむがい）は、  
**Markdown で書かれたノベルゲームのシナリオを「順に解釈・実行する」ための  
Rust 製シナリオ実行ライブラリ**です。

描画や音声再生は行わず、  
**「次に何が起こるか」だけを決定してデータとして返します。**

---

## これは何か？（一言で）

> **ノベルゲームの「シナリオ進行ロジック」だけを担当するライブラリ**

- エンジンではありません
- UI や演出は含みません
- シナリオを **上から順に実行する状態機械**です

---

## tsumugai が提供するもの

tsumugai は、Markdown シナリオを読み取り、  
次のような情報を **構造化されたデータ**として返します。

- 表示すべきテキスト
- プレイヤーに提示すべき選択肢
- 分岐・ジャンプ先
- 現在の実行状態（セーブ／ロード対象）

---

## tsumugai が提供しないもの（重要）

tsumugai は以下を **一切行いません**。

- テキスト・画像の描画
- 音声の再生
- UI 操作
- 演出・アニメーション

tsumugai の責務は、あくまで **「物語の進行を決めること」** だけです。

```text
[ Markdown シナリオ ]
↓
tsumugai
↓
[ 次に起こること（データ） ]
↓
あなたのアプリケーション
```

---

## 想定している使い方

- Rust でノベルゲームを作りたい
- UI や演出は自分で実装したい
- シナリオを Markdown / Git 管理したい
- LLM（ChatGPT / Claude 等）に  
  シナリオ生成・レビューをさせたい

**完成済みエンジンの代替ではありません。**

---

## 最小の使用例（CUI）

```rust
use tsumugai::{Engine, UserEvent};

fn main() -> anyhow::Result<()> {
    let scenario = r#"
[SAY speaker=Alice]
こんにちは。

[CHOICE id=go label="進む" goto=next]

# scene: next
[SAY speaker=Alice]
おしまい。
"#;

    let mut engine = Engine::from_markdown(scenario)?;

    loop {
        let step = engine.step(None)?;

        for directive in step.directives {
            println!("{:?}", directive);
        }

        match step.next {
            tsumugai::NextAction::WaitUser => {
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf)?;
            }
            tsumugai::NextAc 	•	描画はすべて println!
	•	tsumugai は 「次に何が起こるか」だけを返す

⸻

シナリオ形式
	•	標準的な Markdown ファイル
	•	見出し・コメントが使える
	•	人間にも LLM にも読みやすい形式

詳細は docs/ を参照してください。

⸻

設計方針
	•	テストファースト（cargo test）
	•	最小責務・単純な構造
	•	シナリオ実行に特化
	•	ライブラリは静かに振る舞う

⸻

ライセンス

MIT License