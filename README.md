tsumugai

概要

tsumugai（つむがい） は、
Markdown で書かれたノベルゲーム用シナリオを逐次実行する Rust 製ライブラリです。

シナリオを解釈し、
	•	次に表示すべきテキスト
	•	選択肢
	•	分岐先
	•	実行状態（セーブ／ロード対象）

といった 「何が起こるか」 を決定し、
その結果を 構造化されたデータとして返却します。

描画・音声再生・UI 制御は行いません。

⸻

tsumugai がやること／やらないこと

やること
	•	Markdown 形式のシナリオをパースする
	•	シナリオを上から順に逐次実行する
	•	選択肢・分岐・ジャンプを解釈する
	•	現在の実行状態を保持する
	•	セーブ／ロード可能な状態を提供する
	•	次に何を表示・選択すべきかをデータとして返す

やらないこと（重要）

tsumugai は以下を 一切 行いません。
	•	テキストの描画
	•	キャラ [ Markdown シナリオ ]
          ↓
      tsumugai
          ↓
[ 次に起こること（データ） ]
          ↓
   あなたのアプリケーション 	•	tsumugai：物語の進行を決める
	•	アプリ側：どう見せるか／どう鳴らすかを決める

という明確な分業を前提としています。

⸻

最小の使用例（CUI）

以下は、tsumugai を使って
コンソール上でノベルゲームを実行する最小例です。 use tsumugai::{Engine, UserEvent};

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
                let mut _buf = String::new();
                std::io::stdin().read_line(&mut _buf)?;
            }
            tsumugai::NextAction::WaitChoice => {
                engine.step(Some(UserEvent::Choose("go".into())))?;
            }
            tsumugai::NextAction::Halt => break,
        }
    }

    Ok(())
}  この例では：
	•	描画はすべて println!
	•	tsumugai は「何を出すか」しか決めていない

という点に注目してください。

⸻

シナリオ形式について
	•	シナリオは Markdown ファイルで記述します
	•	見出し・コメントを活用できます
	•	人間にも LLM にも読みやすい形式を重視しています

詳細な記法は docs/ を参照してください。

⸻

設計方針（簡潔版）
	•	テストファースト（cargo test）
	•	単一責任・関心の分離
	•	最小限の責務
	•	「ライブラリは静かに振る舞う」

tsumugai は
「これさえあればノベルゲームのロジックは動く」
という最小単位を目指しています。

⸻

ライセンス

MIT License

⸻

補足
	•	CLI / lint / GUI / サンプルアプリは ライブラリ外で提供される想定です
	•	tsumugai 自身は 純粋な Rust ライブラリとして完結します