//! Choice example using the new architecture

use std::env;
use std::io::{self, Write};
use tsumugai::{facade::SimpleEngine, types::event::Event};

fn main() -> anyhow::Result<()> {
    // コマンドライン引数から選択肢を取得
    let args: Vec<String> = env::args().collect();
    let auto_choice = if args.len() > 1 {
        args[1].parse::<usize>().ok()
    } else {
        None
    };
    println!("=== 選択肢機能デモ ===\n");

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

    let mut engine = SimpleEngine::from_markdown(scenario)?;
    let mut step_count = 0;

    println!("シナリオを開始します。\n");

    loop {
        step_count += 1;
        println!("--- ステップ {} ---", step_count);

        let (output, finished) = engine.step(None);

        // 台詞があれば表示
        for line in &output.lines {
            if let Some(speaker) = &line.speaker {
                println!("{}: {}", speaker, line.text);
            } else {
                println!("{}", line.text);
            }
        }

        // エフェクトがあれば表示
        for effect in &output.effects {
            println!("[エフェクト] {}: {:?}", effect.tag, effect.opts);
        }

        // 選択肢があれば表示
        if !output.choices.is_empty() {
            println!("\n選択してください:");
            for (i, choice) in output.choices.iter().enumerate() {
                println!("{}. {}", i + 1, choice.label);
            }

            let choice_num = if let Some(auto_num) = auto_choice {
                // 自動選択モード
                println!("自動選択: {}", auto_num);
                if auto_num > 0 && auto_num <= output.choices.len() {
                    auto_num
                } else {
                    println!("自動選択の値が無効です。手動入力に切り替えます。");
                    get_user_choice(output.choices.len())?
                }
            } else {
                // 手動入力モード
                get_user_choice(output.choices.len())?
            };

            if choice_num > 0 && choice_num <= output.choices.len() {
                let choice_id = format!("choice_{}", choice_num - 1);
                println!("選択: {}", output.choices[choice_num - 1].label);

                // 選択肢イベントで次のステップを実行
                let choice_event = Event::Choice { id: choice_id };
                let (next_output, next_finished) = engine.step(Some(choice_event));

                // 選択後の結果も表示
                for line in &next_output.lines {
                    if let Some(speaker) = &line.speaker {
                        println!("{}: {}", speaker, line.text);
                    } else {
                        println!("{}", line.text);
                    }
                }

                if next_finished {
                    println!("\n=== シナリオ完了 ===");
                    break;
                }
            } else {
                println!("無効な選択です。");
                continue;
            }
        }

        // 変数の状態を表示
        if let Some(path) = engine.state().get_var("path") {
            println!("選択した道: {}", path);
        }

        if finished {
            println!("\n=== シナリオ完了 ===");
            break;
        }

        // 選択肢がない場合はEnterで続行
        if output.choices.is_empty() {
            print!("Enter キーで続行...");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
        }
    }

    // 最終状態を表示
    println!("\n--- 最終状態 ---");
    if let Some(path) = engine.state().get_var("path") {
        println!("最終的に選択した道: {}", path);
    }
    println!("プログラムカウンタ: {}", engine.state().pc);

    Ok(())
}

/// 手動で選択肢の入力を取得する関数
fn get_user_choice(max_choices: usize) -> anyhow::Result<usize> {
    loop {
        print!("番号を入力 (1-{}): ", max_choices);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if let Ok(choice_num) = input.trim().parse::<usize>() {
            if choice_num > 0 && choice_num <= max_choices {
                return Ok(choice_num);
            } else {
                println!("無効な選択です。1から{}の間で入力してください。", max_choices);
            }
        } else {
            println!("数字を入力してください。");
        }
    }
}