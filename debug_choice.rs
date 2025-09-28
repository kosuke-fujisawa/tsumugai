use tsumugai::{facade::SimpleEngine, types::event::Event};

fn main() -> anyhow::Result<()> {
    let scenario = r#"
[BRANCH choice=選択肢A choice=選択肢B]

[LABEL name=選択肢A]
[SAY speaker=ナレーター]
Aを選択しました。

[LABEL name=選択肢B]
[SAY speaker=ナレーター]
Bを選択しました。
"#;

    let mut engine = SimpleEngine::from_markdown(scenario)?;

    // ステップ1: BRANCHを実行
    let (output1, finished1) = engine.step(None);
    println!("=== ステップ1 ===");
    println!("finished: {}", finished1);
    println!("choices: {:?}", output1.choices);
    println!("state.waiting_for_choice: {}", engine.state().waiting_for_choice);
    println!("state.pending_choices: {:?}", engine.state().pending_choices);
    println!("state.pc: {}", engine.state().pc);

    // 選択肢を選択
    if !output1.choices.is_empty() {
        println!("\n=== 選択肢を選択 ===");
        let choice_event = Event::Choice { id: "choice_0".to_string() };
        let (output2, finished2) = engine.step(Some(choice_event));
        println!("finished: {}", finished2);
        println!("lines: {:?}", output2.lines);
        println!("state.pc: {}", engine.state().pc);
        println!("state.waiting_for_choice: {}", engine.state().waiting_for_choice);
    }

    Ok(())
}