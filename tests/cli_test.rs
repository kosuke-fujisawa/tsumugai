//! tsumugai CLI バイナリの外部結合レベルの統合テスト
//!
//! check_test.rs / trace_test.rs / routes_test.rs は `scenario::check_path` 等の
//! ライブラリ関数を直接呼び出しており、`main.rs` の引数パース・exit code・
//! stdout 整形・ファイル I/O（`fmt --write`）は経由しない。
//!
//! tsumugai は外部ツールとして CLI サブプロセス + JSON（stdout / `compile
//! --output`）で消費される契約（docs/ARCHITECTURE.md 8章）なので、ここでは
//! 実バイナリを `CARGO_BIN_EXE_tsumugai` で起動し、check / trace / routes /
//! fmt の 4 コマンドについてその契約をブラックボックスに検証する。
//! `compile` の同種のテストは tests/compile_test.rs にある。

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tsumugai"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("tsumugai バイナリを起動できる")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

// ---------------------------------------------------------------------- check

#[test]
fn check_examplesはexit0で問題なしと表示する() {
    let out = run(&["check", "examples/spring"]);
    assert!(out.status.success(), "stdout: {}", stdout(&out));
    assert!(stdout(&out).contains("問題は見つかりませんでした"));
}

#[test]
fn check_jsonはexit0でstatus_okの有効なjsonを返す() {
    let out = run(&["check", "examples/spring", "--format", "json"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["files"].as_array().unwrap().len(), 2);
}

#[test]
fn check_sarifは有効なsarif_2_1_0を返す() {
    let out = run(&["check", "examples/spring", "--format", "sarif"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(json["version"], "2.1.0");
    assert!(json["runs"].is_array());
}

#[test]
fn checkでbroken_linkがあるとexit1になりファイル出力はしない() {
    let out = run(&["check", "tests/fixtures/trace/broken/scenario.md"]);
    assert!(!out.status.success());
    assert!(stdout(&out).contains("broken-link"));
}

#[test]
fn checkで存在しないパスはexit1でio_errorになる() {
    let out = run(&["check", "no/such/path.md"]);
    assert!(!out.status.success());
    assert!(stdout(&out).contains("io-error"));
}

// ---------------------------------------------------------------------- trace

#[test]
fn trace_choices無しは選択肢の入力待ちで停止しexit0() {
    let out = run(&["trace", "examples/spring/scenario/spring_001.md"]);
    assert!(out.status.success(), "stdout: {}", stdout(&out));
    assert!(stdout(&out).contains("入力待ちで停止しました"));
}

#[test]
fn trace_choicesを与えるとendingに到達しexit0() {
    let out = run(&[
        "trace",
        "examples/spring/scenario/spring_001.md",
        "--choices",
        "1",
    ]);
    assert!(out.status.success(), "stdout: {}", stdout(&out));
    assert!(stdout(&out).contains("childhood_route"));
}

#[test]
fn trace_jsonは有効なjsonを返す() {
    let out = run(&[
        "trace",
        "examples/spring/scenario/spring_001.md",
        "--choices",
        "1",
        "--format",
        "json",
    ]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(json["status"], "ok");
}

#[test]
fn trace_範囲外の選択番号はexit1になる() {
    let out = run(&[
        "trace",
        "examples/spring/scenario/spring_001.md",
        "--choices",
        "99",
    ]);
    assert!(!out.status.success());
    assert!(stdout(&out).contains("選択番号 99"));
}

// --------------------------------------------------------------------- routes

#[test]
fn routesは全経路を探索しexit0で報告する() {
    let out = run(&["routes", "examples/spring/scenario/spring_001.md"]);
    assert!(out.status.success(), "stdout: {}", stdout(&out));
    assert!(stdout(&out).contains("発見した経路数: 4"));
}

#[test]
fn routes_jsonは有効なjsonを返す() {
    let out = run(&[
        "routes",
        "examples/spring/scenario/spring_001.md",
        "--format",
        "json",
    ]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(json["status"], "ok");
}

#[test]
fn routesで循環があるとexit1になる() {
    let out = run(&["routes", "tests/fixtures/trace/loop/scenario.md"]);
    assert!(!out.status.success());
    assert!(stdout(&out).contains("circular-route"));
}

#[test]
fn routesでendingを宣言しないシナリオはwarningを表示しつつexit0になる() {
    let out = run(&["routes", "tests/fixtures/trace/eof/scenario.md"]);
    assert!(out.status.success(), "stdout: {}", stdout(&out));
    assert!(stdout(&out).contains("route-without-ending"));
}

// ------------------------------------------------------------------------ fmt

#[test]
fn fmt_writeなしでは入力ファイルを変更しない() {
    let before = std::fs::read_to_string("examples/fmt/before.md").unwrap();

    // [SHOW_IMAGE ...] が legacy-command の error になるため exit 1 だが、
    // --write を付けない限りファイルには一切触れない
    let out = run(&["fmt", "examples/fmt/before.md"]);
    assert!(!out.status.success());

    let after_run = std::fs::read_to_string("examples/fmt/before.md").unwrap();
    assert_eq!(
        before, after_run,
        "--write なしでは入力ファイルを変更しない"
    );
}

#[test]
fn fmt_writeで整形結果がafter_mdと一致する() {
    // fmt --write は入力ファイルへ直接書き戻すため、リポジトリのサンプルを
    // 汚さないよう一時ディレクトリにコピーしてから実行する。
    // fmt-paren-dialogue の判定には同階層の characters.yaml が要るため、
    // before.md と一緒にコピーする
    let dir = std::env::temp_dir().join(format!(
        "tsumugai-cli-test-fmt-write-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("一時ディレクトリを作成できる");
    let target = dir.join("before.md");
    std::fs::copy("examples/fmt/before.md", &target).unwrap();
    std::fs::copy("examples/fmt/characters.yaml", dir.join("characters.yaml")).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_tsumugai"))
        .args(["fmt", target.to_str().unwrap(), "--write"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("tsumugai バイナリを起動できる");

    // legacy-command の error は変換できないため残り、exit code は 1 のまま。
    // ただし --write の書き戻し自体は変更可能な箇所に対して行われる
    assert!(!out.status.success());

    let written = std::fs::read_to_string(&target).expect("書き戻されたファイルを読める");
    let expected = std::fs::read_to_string("examples/fmt/after.md").unwrap();
    assert_eq!(written, expected);

    let _ = std::fs::remove_dir_all(&dir);
}
