//! CUI プレイヤー（参照実装）
//!
//! # 操作
//! - Enter → 進行
//! - 数字キー → 選択肢を選ぶ
//! - b → 戻る（Undo）
//! - q → 終了
//!
//! # Undo の仕組み
//! step を呼ぶ前に `(State, ViewState)` のペアをスタックに積む。
//! Undo 時はそのスタックから復元する。
//! ViewState も一緒に復元するため、背景・BGM が消えない。

pub mod view_state;

use crate::{
    parser,
    runtime::{self, Input, WaitingType, ir::Event},
    types::state::State,
};
use std::io::{self, Write};
use view_state::ViewState;

/// プレイヤーを起動する
pub fn run(markdown: &str, debug_mode: bool) -> anyhow::Result<()> {
    let ast = parser::parse(markdown)?;
    let program = runtime::compile(&ast);

    let mut state = State::new();
    let mut view = ViewState::new();

    // Undo 用の履歴スタック：step を呼ぶ前のスナップショットを積む
    let mut history: Vec<(State, ViewState)> = Vec::new();

    print_header();
    print_prompt("Enter で開始...");
    read_line()?;

    'main: loop {
        // ─── step を呼ぶ前にスナップショットを保存 ───
        history.push((state.clone(), view.clone()));

        let (new_state, output) = runtime::step(state, &program, None);
        state = new_state;

        // エフェクトを ViewState に適用して差分表示
        let delta = view.apply(&output.events);
        print_delta(&delta);

        // Say イベントを表示
        for event in &output.events {
            if let Event::Say { speaker, text } = event {
                print_say(speaker, text);
            }
        }

        if debug_mode {
            print_debug(&state);
        }

        match output.waiting_for {
            None => {
                // プログラム末尾
                history.pop(); // 末尾のスナップショットは不要
                print_ending();
                break;
            }

            Some(WaitingType::Advance) => {
                // Enter 待ち
                loop {
                    let input = read_input("")?;

                    if input == "q" {
                        print_quit();
                        return Ok(());
                    }

                    if input == "b" {
                        match undo(&mut history, &mut state, &mut view) {
                            UndoResult::Success => {
                                // 外側ループで step(None) を再実行して正しく再表示する
                                // undo() が history.last() を復元済みなので、もう1つ pop して
                                // 外側ループの push と相殺し、履歴の重複を防ぐ
                                redisplay_current(&state, &view, debug_mode);
                                history.pop();
                                continue 'main;
                            }
                            UndoResult::NothingToUndo => {
                                println!("これ以上戻れません。");
                                continue;
                            }
                        }
                    }

                    if input.is_empty() {
                        // step(Advance) で AwaitAdvance を解消
                        let (new_state, output2) =
                            runtime::step(state.clone(), &program, Some(Input::Advance));
                        state = new_state;
                        let delta = view.apply(&output2.events);
                        print_delta(&delta);
                        for event in &output2.events {
                            if let Event::Say { speaker, text } = event {
                                print_say(speaker, text);
                            }
                        }
                        if debug_mode {
                            print_debug(&state);
                        }
                        // メインループへ戻る（次の step を呼ぶ）
                        break;
                    }

                    println!("  Enter で進む / b で戻る / q で終了");
                }
            }

            Some(WaitingType::Ended { ref id, ref name }) => {
                history.pop();
                print_scenario_ending(id, name);
                break;
            }

            Some(WaitingType::Choice(ref options)) => {
                // 選択肢待ち
                print_choices(options);

                loop {
                    let input = read_input(&format!("選択 (1-{}): ", options.len()))?;

                    if input == "q" {
                        print_quit();
                        return Ok(());
                    }

                    if input == "b" {
                        match undo(&mut history, &mut state, &mut view) {
                            UndoResult::Success => {
                                redisplay_current(&state, &view, debug_mode);
                                history.pop();
                                continue 'main;
                            }
                            UndoResult::NothingToUndo => {
                                println!("これ以上戻れません。");
                                continue;
                            }
                        }
                    }

                    // 数字入力を ChoiceOption.id に変換
                    if let Ok(n) = input.parse::<usize>() {
                        if n >= 1 && n <= options.len() {
                            let choice_id = options[n - 1].id.clone();
                            let (new_state, output2) = runtime::step(
                                state.clone(),
                                &program,
                                Some(Input::SelectChoice(choice_id)),
                            );
                            state = new_state;
                            let delta = view.apply(&output2.events);
                            print_delta(&delta);
                            for event in &output2.events {
                                if let Event::Say { speaker, text } = event {
                                    print_say(speaker, text);
                                }
                            }
                            if debug_mode {
                                print_debug(&state);
                            }
                            break;
                        } else {
                            println!("  1〜{} の数字を入力してください。", options.len());
                        }
                    } else {
                        println!("  数字を入力してください。");
                    }
                }
            }
        }
    }

    Ok(())
}

// ──────────────────────────────────────────────
// Undo
// ──────────────────────────────────────────────

enum UndoResult {
    Success,
    NothingToUndo,
}

fn undo(
    history: &mut Vec<(State, ViewState)>,
    state: &mut State,
    view: &mut ViewState,
) -> UndoResult {
    // 直前のスナップショット（現在位置）を捨てて、その前を復元する
    // history には「このステップを始める前の状態」が積まれている
    // Undo = 現在のスナップショットも捨てて、さらに前へ
    if history.len() <= 1 {
        return UndoResult::NothingToUndo;
    }
    history.pop(); // 今いる位置のスナップショットを捨てる
    if let Some((prev_state, prev_view)) = history.last() {
        *state = prev_state.clone();
        *view = prev_view.clone();
        println!("（戻りました）");
        UndoResult::Success
    } else {
        UndoResult::NothingToUndo
    }
}

/// Undo 後の現在状態を再表示する
fn redisplay_current(state: &State, view: &ViewState, debug_mode: bool) {
    if let Some(scene) = &view.scene {
        println!("\n━━ シーン: {} ━━", scene);
    }
    for (layer, name) in &view.images {
        println!("  [画像: {} / {}]", name, layer);
    }
    if let Some(bgm) = &view.bgm {
        println!("  [BGM: {}]", bgm);
    }
    if debug_mode {
        print_debug(state);
    }
}

// ──────────────────────────────────────────────
// 表示ヘルパー
// ──────────────────────────────────────────────

fn print_header() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  tsumugai シナリオプレイヤー");
    println!("  Enter: 進む  b: 戻る  q: 終了");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn print_prompt(msg: &str) {
    println!("{}", msg);
}

fn print_delta(delta: &view_state::RenderDelta) {
    if let Some(scene) = &delta.scene_changed {
        println!("\n━━ シーン: {} ━━", scene);
    }
    for effect in &delta.effects {
        println!("  [{}]", effect);
    }
}

fn print_say(speaker: &str, text: &str) {
    if speaker.is_empty() {
        println!("  {}", text);
    } else {
        println!("【{}】{}", speaker, text);
    }
}

fn print_choices(options: &[crate::runtime::ir::ChoiceOption]) {
    println!();
    for (i, opt) in options.iter().enumerate() {
        println!("  {}. {}", i + 1, opt.label);
    }
}

fn print_ending() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("          THE END");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn print_scenario_ending(id: &str, name: &str) {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  エンディング: {}", name);
    println!("  (id: {})", id);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn print_quit() {
    println!("\nシナリオを終了します。");
}

fn print_debug(state: &State) {
    let vars: Vec<String> = state
        .flags
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    println!("  [DEBUG] pc={} vars={{{}}}", state.pc, vars.join(", "));
}

// ──────────────────────────────────────────────
// 入力ヘルパー
// ──────────────────────────────────────────────

fn read_line() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

fn read_input(prompt: &str) -> io::Result<String> {
    if !prompt.is_empty() {
        print!("{}", prompt);
        io::stdout().flush()?;
    }
    read_line()
}
