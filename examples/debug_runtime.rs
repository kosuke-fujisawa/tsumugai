//! Debug runtime behavior with branch

use tsumugai::{parser, runtime, types::{state::State, event::Event}};

fn main() -> anyhow::Result<()> {
    let scenario = r#"
[BRANCH choice=森の道 choice=山の道]

[LABEL name=森の道]
[SAY speaker=ガイド]
森の道を選びました。

[LABEL name=山の道]
[SAY speaker=ガイド]
山の道を選びました。
"#;

    println!("=== RUNTIME デバッグ ===\n");

    let ast = parser::parse(scenario)?;
    let mut state = State::new();

    println!("初期状態:");
    println!("  PC: {}", state.pc);
    println!("  waiting_for_choice: {}", state.waiting_for_choice);

    // Step 1: BRANCH を実行
    println!("\n--- Step 1: BRANCH 実行 ---");
    let (new_state, output) = runtime::step(state, &ast, None);
    state = new_state;

    println!("実行後の状態:");
    println!("  PC: {}", state.pc);
    println!("  waiting_for_choice: {}", state.waiting_for_choice);
    println!("  pending_choices: {:?}", state.pending_choices);

    println!("AST labels: {:?}", ast.labels);
    for (i, choice_target) in state.pending_choices.iter().enumerate() {
        if let Some(label_index) = ast.get_label_index(choice_target) {
            println!("  choice_{} -> '{}' -> label_index {}", i, choice_target, label_index);
        }
    }

    println!("出力:");
    println!("  choices.len(): {}", output.choices.len());
    for (i, choice) in output.choices.iter().enumerate() {
        println!("    {}: id='{}', label='{}'", i, choice.id, choice.label);
    }

    // Step 2: choice_0 を選択
    println!("\n--- Step 2: choice_0 を選択 ---");
    let choice_event = Event::Choice { id: "choice_0".to_string() };
    let (new_state, output) = runtime::step(state, &ast, Some(choice_event));
    state = new_state;

    println!("選択後の状態:");
    println!("  PC: {}", state.pc);
    println!("  waiting_for_choice: {}", state.waiting_for_choice);
    println!("  pending_choices: {:?}", state.pending_choices);

    println!("出力:");
    println!("  lines.len(): {}", output.lines.len());
    for line in &output.lines {
        println!("    {}: {}", line.speaker.as_ref().unwrap_or(&"None".to_string()), line.text);
    }

    // Step 3: 続行
    println!("\n--- Step 3: 続行 ---");
    let (new_state, output) = runtime::step(state, &ast, None);
    state = new_state;

    println!("続行後の状態:");
    println!("  PC: {}", state.pc);
    println!("  waiting_for_choice: {}", state.waiting_for_choice);

    println!("出力:");
    println!("  lines.len(): {}", output.lines.len());
    for line in &output.lines {
        println!("    {}: {}", line.speaker.as_ref().unwrap_or(&"None".to_string()), line.text);
    }

    Ok(())
}