//! tsumugai CLI エントリーポイント

use std::fs;
use tsumugai::runtime::ir::Event;
use tsumugai::runtime::trace::RuntimeTrace;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let usage = concat!(
        "使い方: tsumugai <command> <file> [--json] [--debug]\n",
        "コマンド:\n",
        "  check <file>         シナリオの静的検証（人間向け出力）\n",
        "  check <file> --json  シナリオの静的検証（JSON出力）\n",
        "  trace <file>         シナリオの実行トレース（先頭選択肢を自動選択）\n",
        "  trace <file> --json  シナリオの実行トレース（JSON出力）\n",
        "  play  <file>         シナリオの対話再生\n",
        "  play  <file> --debug デバッグ情報付きで再生"
    );

    if args.len() < 3 {
        eprintln!("{}", usage);
        std::process::exit(1);
    }

    let command = &args[1];
    let file_path = &args[2];
    let debug_mode = args.contains(&"--debug".to_string());
    let json_mode = args.contains(&"--json".to_string());

    let markdown = fs::read_to_string(file_path)
        .map_err(|e| anyhow::anyhow!("ファイルを読み込めません '{}': {}", file_path, e))?;

    match command.as_str() {
        "play" => {
            tsumugai::player::run(&markdown, debug_mode)?;
        }
        "check" => {
            let ast = match tsumugai::parser::parse(&markdown) {
                Ok(ast) => ast,
                Err(e) => {
                    if json_mode {
                        let output =
                            tsumugai::analyzer::CheckJsonOutput::parse_error(e.to_string());
                        println!("{}", serde_json::to_string_pretty(&output)?);
                        std::process::exit(1);
                    } else {
                        return Err(e);
                    }
                }
            };

            let result = tsumugai::analyzer::analyze(&ast);

            if json_mode {
                let output = tsumugai::analyzer::CheckJsonOutput::from(&result);
                println!("{}", serde_json::to_string_pretty(&output)?);
                if result.has_errors() {
                    std::process::exit(1);
                }
            } else if result.is_clean() {
                println!("✓ 問題は見つかりませんでした。");
            } else {
                for issue in &result.issues {
                    let level = match issue.level {
                        tsumugai::analyzer::Level::Error => "エラー",
                        tsumugai::analyzer::Level::Warning => "警告",
                        tsumugai::analyzer::Level::Info => "情報",
                    };
                    println!("[{}][{}] {}", level, issue.rule_id, issue.message);
                    if let Some(span) = &issue.span {
                        println!("  位置: {}行目", span.line);
                    }
                    if let Some(suggestion) = &issue.suggestion {
                        println!("  提案: {}", suggestion);
                    }
                }
                println!(
                    "\nエラー: {}件  警告: {}件  情報: {}件",
                    result.error_count(),
                    result.warning_count(),
                    result.info_count()
                );
                if result.has_errors() {
                    std::process::exit(1);
                }
            }
        }
        "trace" => {
            let ast = match tsumugai::parser::parse(&markdown) {
                Ok(ast) => ast,
                Err(e) => {
                    if json_mode {
                        let trace = tsumugai::runtime::trace::RuntimeTrace {
                            total_steps: 0,
                            truncated: false,
                            steps: vec![],
                        };
                        let output = tsumugai::runtime::trace::TraceJsonOutput {
                            status: "error",
                            trace,
                        };
                        println!("{}", serde_json::to_string_pretty(&output)?);
                        std::process::exit(1);
                    } else {
                        return Err(e);
                    }
                }
            };

            let program = tsumugai::runtime::compile(&ast);
            let trace = tsumugai::runtime::trace::trace_linear(&program);

            if json_mode {
                let output = tsumugai::runtime::trace::TraceJsonOutput {
                    status: "ok",
                    trace,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                print_trace_human(&trace);
                if trace.truncated {
                    eprintln!("警告: ステップ数の上限に達したため打ち切られました。");
                }
            }
        }
        _ => {
            eprintln!("不明なコマンド: {}\n{}", command, usage);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_trace_human(trace: &RuntimeTrace) {
    println!("=== Runtime Trace ({} steps) ===\n", trace.total_steps);
    for step in &trace.steps {
        println!(
            "Step {} (pc {} → {})",
            step.step_no, step.pc_before, step.pc_after
        );
        if let Some(input) = &step.input {
            println!("  Input   : {}", input);
        } else {
            println!("  Input   : (初回実行)");
        }
        if step.events.is_empty() {
            println!("  Events  : (なし)");
        } else {
            for event in &step.events {
                println!("  Event   : {}", describe_event(event));
            }
        }
        if step.state_diff.var_changes.is_empty() {
            println!("  State   : (変化なし)");
        } else {
            for change in &step.state_diff.var_changes {
                match &change.before {
                    None => println!("  State   : {} = {} (新規)", change.key, change.after),
                    Some(before) => {
                        println!("  State   : {} {} → {}", change.key, before, change.after)
                    }
                }
            }
        }
        match &step.waiting_for {
            None => println!("  Waiting : (終了)"),
            Some(w) => println!("  Waiting : {}", w),
        }
        println!();
    }
}

fn describe_event(event: &Event) -> String {
    match event {
        Event::Say { speaker, text } if speaker.is_empty() => format!("Narration: {}", text),
        Event::Say { speaker, text } => format!("Say({}): {}", speaker, text),
        Event::SceneStart { name } => format!("SceneStart: {}", name),
        Event::ShowImage { layer, name } => format!("ShowImage: {} / {}", layer, name),
        Event::ClearLayer { layer } => format!("ClearLayer: {}", layer),
        Event::PlayBgm { name } => format!("PlayBgm: {}", name),
        Event::PlaySe { name } => format!("PlaySe: {}", name),
        Event::PlayMovie { name } => format!("PlayMovie: {}", name),
        Event::Wait { duration } => format!("Wait: {}s", duration),
        Event::Ending { id, name } => format!("Ending: {} ({})", name, id),
        Event::Custom { tag, params } => format!("Custom[{}]: {}", tag, params.join(", ")),
    }
}
