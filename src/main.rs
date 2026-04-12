//! tsumugai CLI エントリーポイント

use std::fs;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let usage = "使い方: tsumugai <command> <file> [--debug]\n\
                 コマンド: play, check";

    if args.len() < 3 {
        eprintln!("{}", usage);
        std::process::exit(1);
    }

    let command = &args[1];
    let file_path = &args[2];
    let debug_mode = args.contains(&"--debug".to_string());

    let markdown = fs::read_to_string(file_path)
        .map_err(|e| anyhow::anyhow!("ファイルを読み込めません '{}': {}", file_path, e))?;

    match command.as_str() {
        "play" => {
            tsumugai::player::run(&markdown, debug_mode)?;
        }
        "check" => {
            let ast = tsumugai::parser::parse(&markdown)?;
            let result = tsumugai::analyzer::analyze(&ast);

            if result.is_clean() {
                println!("✓ 問題は見つかりませんでした。");
            } else {
                for issue in &result.issues {
                    let level = match issue.level {
                        tsumugai::analyzer::Level::Error => "エラー",
                        tsumugai::analyzer::Level::Warning => "警告",
                        tsumugai::analyzer::Level::Info => "情報",
                    };
                    println!("[{}] {}", level, issue.message);
                }
                println!(
                    "\nエラー: {}件  警告: {}件",
                    result.error_count(),
                    result.warning_count()
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
