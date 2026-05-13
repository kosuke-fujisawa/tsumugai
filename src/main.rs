//! tsumugai CLI エントリーポイント

use std::fs;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let usage = concat!(
        "使い方: tsumugai <command> <file> [--json] [--debug]\n",
        "コマンド:\n",
        "  check <file>         シナリオの静的検証（人間向け出力）\n",
        "  check <file> --json  シナリオの静的検証（JSON出力）\n",
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
        _ => {
            eprintln!("不明なコマンド: {}\n{}", command, usage);
            std::process::exit(1);
        }
    }

    Ok(())
}
