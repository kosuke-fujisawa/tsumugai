//! Facade module providing a simple interface over the new architecture
//!
//! This module demonstrates how the new parse → step → save/load API works
//! and provides backward compatibility wrappers.

use crate::types::{Ast, Event, Output, State};
use crate::{parser, runtime, storage};

/// A simple facade engine that wraps the new architecture
#[derive(Debug)]
pub struct SimpleEngine {
    ast: Ast,
    state: State,
}

impl SimpleEngine {
    /// Create a new engine from markdown content
    pub fn from_markdown(markdown: &str) -> anyhow::Result<Self> {
        let ast = parser::parse(markdown)?;
        let state = State::new();
        Ok(Self { ast, state })
    }

    /// Execute one step of the scenario
    pub fn step(&mut self, event: Option<Event>) -> (Output, bool) {
        let (new_state, output) = runtime::step(self.state.clone(), &self.ast, event);
        self.state = new_state;

        // Check if we've reached the end
        let is_finished = self.state.pc >= self.ast.len();
        (output, is_finished)
    }

    /// Save the current state to bytes
    pub fn save_state(&self) -> anyhow::Result<Vec<u8>> {
        storage::save(&self.state)
    }

    /// Load state from bytes
    pub fn load_state(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.state = storage::load(bytes)?;
        Ok(())
    }

    /// Get the current state
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Get the AST
    pub fn ast(&self) -> &Ast {
        &self.ast
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_simple_scenario() {
        let markdown = r#"
[SAY speaker=Alice]
Hello, world!

[SET name=score value=100]

[SAY speaker=Bob]
Your score is 100!
"#;

        let mut engine = SimpleEngine::from_markdown(markdown).expect("Failed to parse");

        // Step 1: SAY Alice
        let (output1, finished1) = engine.step(None);
        assert!(!finished1);
        assert_eq!(output1.lines.len(), 1);
        assert_eq!(output1.lines[0].speaker, Some("Alice".to_string()));
        assert_eq!(output1.lines[0].text, "Hello, world!");

        // Step 2: SET score (should continue and execute both SET and SAY)
        let (output2, finished2) = engine.step(None);
        assert_eq!(engine.state().get_var("score"), Some("100".to_string()));
        assert_eq!(output2.lines.len(), 1);
        assert_eq!(output2.lines[0].speaker, Some("Bob".to_string()));
        assert_eq!(output2.lines[0].text, "Your score is 100!");

        // After executing the last SAY, we should be finished
        assert!(finished2);
    }

    #[test]
    fn facade_save_load() {
        let markdown = r#"
[SET name=progress value=checkpoint1]
[SAY speaker=Narrator]
Checkpoint reached!
"#;

        let mut engine = SimpleEngine::from_markdown(markdown).expect("Failed to parse");

        // Execute first step
        let (output1, _) = engine.step(None);
        assert_eq!(output1.lines[0].text, "Checkpoint reached!");
        assert_eq!(
            engine.state().get_var("progress"),
            Some("checkpoint1".to_string())
        );

        // Save state
        let saved_bytes = engine.save_state().expect("Failed to save");

        // Modify state (simulate progression)
        engine
            .state
            .set_var("progress".to_string(), "checkpoint2".to_string());
        assert_eq!(
            engine.state().get_var("progress"),
            Some("checkpoint2".to_string())
        );

        // Load saved state
        engine.load_state(&saved_bytes).expect("Failed to load");
        assert_eq!(
            engine.state().get_var("progress"),
            Some("checkpoint1".to_string())
        );
    }

    #[test]
    fn choice_example_integration_test() {
        let scenario = r#"
[SAY speaker=ガイド]
冒険の始まりです。

[SAY speaker=ガイド]
どちらの道を選びますか？

[BRANCH choice=森の道 choice=山の道]

[LABEL name=森の道]
[SAY speaker=ガイド]
森の道を選びました。緑豊かな風景が広がります。

[SET name=path value=forest]

[SAY speaker=ガイド]
森で美しい花を見つけました！

[JUMP label=結末]

[LABEL name=山の道]
[SAY speaker=ガイド]
山の道を選びました。険しい道のりですが景色は絶景です。

[SET name=path value=mountain]

[SAY speaker=ガイド]
山頂で素晴らしい景色を見ることができました！

[LABEL name=結末]
[SAY speaker=ガイド]
冒険が完了しました。お疲れさまでした！
"#;

        let mut engine = SimpleEngine::from_markdown(scenario).expect("Failed to parse");

        // ステップ1: 最初の台詞
        let (output1, finished1) = engine.step(None);
        assert!(!finished1);
        assert_eq!(output1.lines.len(), 1);
        assert_eq!(output1.lines[0].text, "冒険の始まりです。");

        // ステップ2: 2番目の台詞
        let (output2, finished2) = engine.step(None);
        assert!(!finished2);
        assert_eq!(output2.lines.len(), 1);
        assert_eq!(output2.lines[0].text, "どちらの道を選びますか？");

        // ステップ3: 選択肢表示
        let (output3, finished3) = engine.step(None);
        assert!(!finished3);
        assert_eq!(output3.choices.len(), 2);
        assert_eq!(output3.choices[0].label, "森の道");
        assert_eq!(output3.choices[1].label, "山の道");

        // 森の道を選択
        let choice_event = Event::Choice {
            id: "choice_0".to_string(),
        };
        let (output4, finished4) = engine.step(Some(choice_event));
        assert!(!finished4);
        assert_eq!(output4.lines.len(), 1);
        assert_eq!(
            output4.lines[0].text,
            "森の道を選びました。緑豊かな風景が広がります。"
        );

        // 変数が設定されているか確認（デバッグ用）
        println!("Current PC: {}", engine.state().pc);
        println!("Variables: {:?}", engine.state().flags);

        // 次のステップを実行してSETコマンドを処理
        let (_output4_5, _) = engine.step(None);
        println!("After SET command: Variables: {:?}", engine.state().flags);

        assert_eq!(engine.state().get_var("path"), Some("forest".to_string()));

        // JUMPで結末に移動し、最終台詞が表示される
        let (output5, finished5) = engine.step(None);
        println!("Output5: lines={:?}, finished={}", output5.lines, finished5);
        assert!(finished5);
        assert_eq!(
            output5.lines[0].text,
            "冒険が完了しました。お疲れさまでした！"
        );
    }
}
