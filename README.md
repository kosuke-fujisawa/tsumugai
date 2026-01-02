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

- 指定漏れ（背景未指定など）を検出
- 到達不能な分岐の検出
- 条件式・選択肢定義の不整合チェック
- ワーニング付きで最後まで流せる

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

最小の使用例（CUI / ドライラン）
```
use tsumugai::{Engine, UserEvent};

fn main() -> anyhow::Result<()> {
    let scenario = r#"
[SAY speaker=Alice]
こんにちは。

:::choices
- 進む @go
:::

# scene: next
[SAY speaker=Alice]
おしまい。
"#;

    let mut engine = Engine::from_markdown(scenario)?;

    loop {
        let step = engine.step(None)?;

        // tsumugai が返す「指示」をそのまま出力
        for directive in step.directives {
            println!("{:?}", directive);
        }

        match step.next {
            tsumugai::NextAction::WaitUser => {
                // Enter待ち
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf)?;
            }
            tsumugai::NextAction::WaitChoice => {
                // 選択肢を選ぶ（ここでは固定）
                engine.step(Some(UserEvent::Choose("go".into())))?;
            }
            tsumugai::NextAction::Halt => break,
        }
    }

    Ok(())
}
```

	•	描画はすべて println!
	•	tsumugai は 「何をすべきか」だけを返す
	•	実際の表示・演出はアプリ側の責務

⸻

シナリオ形式
	•	標準的な Markdown ファイル
	•	見出し・コメントが使える
	•	人間にも LLM にも読みやすい
	•	独自タグは最小限

👉 「ライターが書きやすいこと」を最優先

詳細な記法は docs/ を参照してください。

⸻

設計方針
	•	テストファースト（TDD）
	•	最小責務・単純な構造
	•	シナリオ解釈と検証に特化
	•	ライブラリは静かに振る舞う
	•	実装より「意味」を返す

⸻

tsumugai が目指すもの
	•	これさえあれば シナリオの正しさは確認できる
	•	実行にもチェックにも使える
	•	LLM フレンドリーなノベルゲーム開発基盤

⸻

ライセンス

MIT License