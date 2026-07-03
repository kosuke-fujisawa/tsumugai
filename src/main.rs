//! tsumugai CLI エントリーポイント

use std::fs;
use std::path::Path;
use tsumugai::scenario;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let usage = concat!(
        "使い方: tsumugai <command> <path> [options]\n",
        "コマンド:\n",
        "  check <path>   シナリオの静的検査（ファイルまたはディレクトリ）\n",
        "      --format human|json|sarif  出力形式（既定: human）\n",
        "      --no-assets                background / bgm の実在チェックを省略\n",
        "  trace <file>   シナリオを 1 経路ぶん自動実行して表示（SPEC 5.1）\n",
        "      --choices 1,3,1            選択肢で選ぶ番号（ブロック内の並び順、1 始まり）\n",
        "      --format human|json        出力形式（既定: human）。--json は --format json と同じ\n",
        "      --no-assets                background / bgm の実在チェックを省略\n",
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
            let (json, options) = parse_trace_args(&args[3..], usage);
            let result = scenario::trace_path(Path::new(file_path), &options);
            let rendered = if json {
                scenario::render_trace_json(&result)
            } else {
                scenario::render_trace_human(&result)
            };
            println!("{}", rendered);
            if result.has_errors() {
                std::process::exit(1);
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

/// trace の引数を解釈する。返り値は (JSON 出力か, オプション)
fn parse_trace_args(rest: &[String], usage: &str) -> (bool, scenario::TraceOptions) {
    let mut json = false;
    let mut options = scenario::TraceOptions::default();
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--choices" => {
                let Some(list) = iter.next() else {
                    eprintln!(
                        "--choices には選択番号をカンマ区切りで指定してください（例: --choices 1,3,1）"
                    );
                    std::process::exit(1);
                };
                options.choices = list
                    .split(',')
                    .map(|s| match s.trim().parse::<usize>() {
                        Ok(n) if n >= 1 => n,
                        _ => {
                            eprintln!(
                                "--choices の「{}」が選択番号として読めません。選択肢ブロック内の並び順を 1 始まりの数字で指定してください（例: --choices 1,3,1）",
                                s.trim()
                            );
                            std::process::exit(1);
                        }
                    })
                    .collect();
            }
            "--json" => json = true,
            "--format" => match iter.next().map(String::as_str) {
                Some("human") => json = false,
                Some("json") => json = true,
                other => {
                    eprintln!(
                        "trace の --format には human / json を指定してください（指定: {}）",
                        other.unwrap_or("なし")
                    );
                    std::process::exit(1);
                }
            },
            "--no-assets" => options.check_assets = false,
            other => {
                eprintln!("不明なオプション: {}\n{}", other, usage);
                std::process::exit(1);
            }
        }
    }
    (json, options)
}
