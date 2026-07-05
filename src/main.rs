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
        "  routes <file>  全分岐を探索し到達可能性を報告（SPEC 5.2）\n",
        "      --format human|json        出力形式（既定: human）\n",
        "      --no-assets                background / bgm の実在チェックを省略\n",
        "  fmt   <file>   よくある書き方を推測して v1 記法へ整形する（SPEC 7章）\n",
        "      --write                    整形結果をファイルに書き戻す（既定は表示のみ）\n",
        "      --format human|json        出力形式（既定: human）"
    );

    if args.len() < 3 {
        eprintln!("{}", usage);
        std::process::exit(1);
    }

    let command = &args[1];
    let file_path = &args[2];

    match command.as_str() {
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
        "routes" => {
            let (json, options) = parse_routes_args(&args[3..], usage);
            let result = scenario::routes_path(Path::new(file_path), &options);
            let rendered = if json {
                scenario::render_routes_json(&result)
            } else {
                scenario::render_routes_human(&result)
            };
            println!("{}", rendered);
            if result.has_errors() {
                std::process::exit(1);
            }
        }
        "fmt" => {
            let (json, write) = parse_fmt_args(&args[3..], usage);
            let result = scenario::fmt_path(Path::new(file_path));
            let rendered = if json {
                scenario::render_fmt_json(&result)
            } else {
                scenario::render_fmt_human(&result)
            };
            println!("{}", rendered);
            if write && result.has_changes() {
                fs::write(file_path, &result.formatted).map_err(|e| {
                    anyhow::anyhow!("ファイルに書き戻せません '{}': {}", file_path, e)
                })?;
            }
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

/// routes の引数を解釈する。返り値は (JSON 出力か, オプション)
fn parse_routes_args(rest: &[String], usage: &str) -> (bool, scenario::RoutesOptions) {
    let mut json = false;
    let mut options = scenario::RoutesOptions::default();
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--format" => match iter.next().map(String::as_str) {
                Some("human") => json = false,
                Some("json") => json = true,
                other => {
                    eprintln!(
                        "routes の --format には human / json を指定してください（指定: {}）",
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

/// fmt の引数を解釈する。返り値は (JSON 出力か, --write が指定されたか)
fn parse_fmt_args(rest: &[String], usage: &str) -> (bool, bool) {
    let mut json = false;
    let mut write = false;
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--write" => write = true,
            "--json" => json = true,
            "--format" => match iter.next().map(String::as_str) {
                Some("human") => json = false,
                Some("json") => json = true,
                other => {
                    eprintln!(
                        "fmt の --format には human / json を指定してください（指定: {}）",
                        other.unwrap_or("なし")
                    );
                    std::process::exit(1);
                }
            },
            other => {
                eprintln!("不明なオプション: {}\n{}", other, usage);
                std::process::exit(1);
            }
        }
    }
    (json, write)
}
