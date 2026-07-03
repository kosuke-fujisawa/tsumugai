//! tsumugai CLI エントリーポイント

use std::fs;
use std::path::Path;
use tsumugai::runtime::ir::Event;
use tsumugai::runtime::trace::RuntimeTrace;
use tsumugai::scenario;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let usage = concat!(
        "使い方: tsumugai <command> <path> [options]\n",
        "コマンド:\n",
        "  check <path>   シナリオの静的検査（ファイルまたはディレクトリ）\n",
        "      --format human|json|sarif  出力形式（既定: human）\n",
        "      --no-assets                background / bgm の実在チェックを省略\n",
        "  trace <file>   シナリオの実行トレース（旧記法。#77 で v1 記法に対応予定）\n",
        "      --json                     JSON 出力\n",
        "  play  <file>   シナリオの対話再生（旧記法）\n",
        "      --debug                    デバッグ情報付きで再生"
    );

    if args.len() < 3 {
        eprintln!("{}", usage);
        std::process::exit(1);
    }

    let command = &args[1];
    let file_path = &args[2];
    let debug_mode = args.contains(&"--debug".to_string());
    let json_mode = args.contains(&"--json".to_string());

    match command.as_str() {
        "play" => {
            let markdown = read_markdown(file_path)?;
            tsumugai::player::run(&markdown, debug_mode)?;
        }
        "check" => {
            let (format, options) = parse_check_args(&args[3..], usage);
            let result = scenario::check_path(Path::new(file_path), &options);
            let rendered = match format {
                CheckFormat::Human => scenario::render_human(&result),
                CheckFormat::Json => scenario::render_json(&result),
                CheckFormat::Sarif => scenario::render_sarif(&result),
            };
            println!("{}", rendered);
            if result.has_errors() {
                std::process::exit(1);
            }
        }
        "trace" => {
            let markdown = read_markdown(file_path)?;
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

fn read_markdown(file_path: &str) -> anyhow::Result<String> {
    fs::read_to_string(file_path)
        .map_err(|e| anyhow::anyhow!("ファイルを読み込めません '{}': {}", file_path, e))
}

enum CheckFormat {
    Human,
    Json,
    Sarif,
}

fn parse_check_args(rest: &[String], usage: &str) -> (CheckFormat, scenario::CheckOptions) {
    let mut format = CheckFormat::Human;
    let mut options = scenario::CheckOptions::default();
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--format" => {
                format = match iter.next().map(String::as_str) {
                    Some("human") => CheckFormat::Human,
                    Some("json") => CheckFormat::Json,
                    Some("sarif") => CheckFormat::Sarif,
                    other => {
                        eprintln!(
                            "--format には human / json / sarif を指定してください（指定: {}）",
                            other.unwrap_or("なし")
                        );
                        std::process::exit(1);
                    }
                };
            }
            "--no-assets" => options.check_assets = false,
            other => {
                eprintln!("不明なオプション: {}\n{}", other, usage);
                std::process::exit(1);
            }
        }
    }
    (format, options)
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
