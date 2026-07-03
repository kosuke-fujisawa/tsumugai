//! `scenario::check_path` の統合テスト
//!
//! 入力は tests/fixtures/check/ 配下のミニプロジェクト。各ディレクトリが
//! SPEC 6章の意味論ルール 1 つに対応し、対象ルール以外の Diagnostic が
//! 出ないことも同時に確認する。examples/spring/ は仕様網羅サンプルであり
//! Diagnostic 0 件が #76 の完了条件。

use serde_json::Value;
use std::path::{Path, PathBuf};
use tsumugai::scenario::{
    CheckOptions, CheckResult, Severity, check_path, render_human, render_json, render_sarif,
};

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/check")
        .join(rel)
}

fn check(rel: &str) -> CheckResult {
    check_path(&fixture(rel), &CheckOptions::default())
}

fn rule_ids(result: &CheckResult) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.rule_id).collect()
}

// ---------------------------------------------------------- 完了条件の確認

#[test]
fn examples_springはディレクトリ検査でdiagnostic0件() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/spring");
    let result = check_path(&dir, &CheckOptions::default());
    assert_eq!(result.diagnostics, vec![], "仕様網羅サンプルは 0 件のはず");
    assert_eq!(result.files.len(), 2);
}

#[test]
fn examples_springは単一ファイル起点でもdiagnostic0件() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/spring/scenario/spring_001.md");
    let result = check_path(&file, &CheckOptions::default());
    assert_eq!(result.diagnostics, vec![]);
    // リンクで辿れる spring_002.md も検査対象に入る
    assert_eq!(result.files.len(), 2);
}

// -------------------------------------------------------------- 意味論ルール

#[test]
fn シーンidの重複はduplicate_scene_idになる() {
    let result = check("duplicate_id");
    assert_eq!(rule_ids(&result), vec!["duplicate-scene-id"]);
    let diag = &result.diagnostics[0];
    assert!(
        diag.file.ends_with("b.md"),
        "後に読んだ側に出る: {:?}",
        diag.file
    );
    assert_eq!(diag.span.as_ref().map(|s| s.line), Some(2));
    assert!(
        diag.message.contains("a.md"),
        "重複相手を案内する: {}",
        diag.message
    );
}

#[test]
fn アンカーの書き間違いはbroken_linkと類似見出しの提案になる() {
    let result = check("typo_anchor/scene.md");
    assert_eq!(rule_ids(&result), vec!["broken-link"]);
    let diag = &result.diagnostics[0];
    assert_eq!(diag.span.as_ref().map(|s| s.line), Some(9));
    assert!(diag.message.contains("## run-together"), "{}", diag.message);
    assert_eq!(
        diag.suggestion.as_deref(),
        Some("[一緒に走る](#run-together)"),
        "ユーザーが書いたラベルをそのまま使った書き換え例を出す"
    );
}

#[test]
fn 存在しないファイルへのリンクはbroken_linkと類似ファイルの提案になる() {
    let result = check("missing_file/scene.md");
    assert_eq!(rule_ids(&result), vec!["broken-link"]);
    let diag = &result.diagnostics[0];
    assert!(diag.message.contains("spring_999.md"), "{}", diag.message);
    assert_eq!(diag.suggestion.as_deref(), Some("[翌朝へ](spring_002.md)"));
    // リンク切れのファイルは検査対象に加わらない
    assert_eq!(result.files.len(), 1);
}

#[test]
fn 絶対パスの参照はリンクもアセットも拒否される() {
    // SPEC 2章: tsumugai が読むのは相対パスで参照されるファイルだけ。
    // 絶対パスは実在の有無にかかわらず error に倒す
    let result = check("absolute_path/scene.md");
    assert_eq!(rule_ids(&result), vec!["missing-asset", "broken-link"]);
    for diag in &result.diagnostics {
        assert!(diag.message.contains("絶対パス"), "{}", diag.message);
        assert!(diag.message.contains("相対パス"), "{}", diag.message);
    }
    // 絶対パスのリンク先は検査対象（閉包）にも加えない
    assert_eq!(result.files.len(), 1);
}

#[test]
fn h1タイトルへのリンクはbroken_linkでh2だけが分岐先だと案内される() {
    let result = check("h1_link/scene.md");
    assert_eq!(rule_ids(&result), vec!["broken-link"]);
    let diag = &result.diagnostics[0];
    assert!(diag.message.contains("H2"), "{}", diag.message);
}

#[test]
fn 別ファイルの存在しないアンカーはbroken_linkで使える見出しが案内される() {
    let result = check("cross_file");
    assert_eq!(rule_ids(&result), vec!["broken-link"]);
    let diag = &result.diagnostics[0];
    assert!(diag.file.ends_with("a.md"));
    assert!(diag.message.contains("b.md"), "{}", diag.message);
    assert!(diag.message.contains("#evening"), "{}", diag.message);
}

#[test]
fn 実在しないアセットはmissing_assetと類似ファイルの提案になる() {
    let result = check("missing_asset/scene.md");
    assert_eq!(rule_ids(&result), vec!["missing-asset"]);
    let diag = &result.diagnostics[0];
    assert_eq!(
        diag.span.as_ref().map(|s| s.line),
        Some(3),
        "front matter の background の行を指す"
    );
    assert_eq!(
        diag.suggestion.as_deref(),
        Some("background: assets/bg/school_gate.png")
    );
}

#[test]
fn no_assetsオプションでアセット検査を省略できる() {
    let result = check_path(
        &fixture("missing_asset/scene.md"),
        &CheckOptions {
            check_assets: false,
        },
    );
    assert_eq!(result.diagnostics, vec![]);
}

#[test]
fn 未宣言の話者はundefined_characterのwarningになる() {
    let result = check("undefined_character/scene.md");
    assert_eq!(rule_ids(&result), vec!["undefined-character"]);
    let diag = &result.diagnostics[0];
    assert_eq!(diag.severity, Severity::Warning);
    assert!(!result.has_errors(), "warning のみなら終了コード 0 相当");
    // 最初の出現に warning、2 回目以降は related_spans
    assert_eq!(diag.span.as_ref().map(|s| s.line), Some(7));
    assert_eq!(diag.related_spans.len(), 1);
    assert_eq!(diag.related_spans[0].line, 9);
    // 「幼馴染」→ 宣言済みの「幼なじみ」を提案する
    assert!(diag.message.contains("幼なじみ"), "{}", diag.message);
    assert_eq!(diag.suggestion.as_deref(), Some("幼なじみ: おはよう。"));
}

#[test]
fn characters_yamlがないとmissing_characters_fileだけになる() {
    let result = check("missing_characters/scene.md");
    // undefined-character は報告しない（SPEC 2.1）
    assert_eq!(rule_ids(&result), vec!["missing-characters-file"]);
    assert_eq!(result.diagnostics[0].severity, Severity::Warning);
}

#[test]
fn 壊れたcharacters_yamlはinvalid_characters_fileになる() {
    let result = check("invalid_characters/scene.md");
    // undefined-character は報告しない（SPEC 2.1）
    assert_eq!(rule_ids(&result), vec!["invalid-characters-file"]);
    let diag = &result.diagnostics[0];
    assert_eq!(diag.severity, Severity::Error);
    assert!(diag.file.ends_with("characters.yaml"), "{:?}", diag.file);
}

#[test]
fn 終端のないセクションはimplicit_fallthroughになる() {
    let result = check("fallthrough/scene.md");
    assert_eq!(rule_ids(&result), vec!["implicit-fallthrough"]);
    let diag = &result.diagnostics[0];
    assert_eq!(diag.span.as_ref().map(|s| s.line), Some(9));
    assert!(diag.message.contains("二日目"), "{}", diag.message);
    assert_eq!(diag.related_spans.first().map(|s| s.line), Some(11));
}

#[test]
fn どこからも到達しないセクションはunreachable_sectionになる() {
    let result = check("unreachable/scene.md");
    assert_eq!(rule_ids(&result), vec!["unreachable-section"]);
    let diag = &result.diagnostics[0];
    assert_eq!(diag.span.as_ref().map(|s| s.line), Some(15));
    assert!(diag.message.contains("orphan"), "{}", diag.message);
}

#[test]
fn 存在しないパスはio_errorのdiagnosticになる() {
    let result = check("no_such_path.md");
    assert_eq!(rule_ids(&result), vec!["io-error"]);
    assert!(result.has_errors());
}

// ------------------------------------------------------------------ 出力形式

#[test]
fn 人間向け出力はrustc風で位置と入力行と提案を示す() {
    let rendered = render_human(&check("typo_anchor/scene.md"));
    assert!(rendered.contains("error[broken-link]:"), "{rendered}");
    assert!(rendered.contains("scene.md:9"), "{rendered}");
    assert!(
        rendered.contains("| - [一緒に走る](#run-togather)"),
        "入力 Markdown の該当行を引用する: {rendered}"
    );
    assert!(rendered.contains("= help: "), "{rendered}");
    assert!(rendered.contains("エラー: 1件  警告: 0件"), "{rendered}");
}

#[test]
fn 問題がなければ人間向け出力はチェックマークだけになる() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/spring");
    let rendered = render_human(&check_path(&dir, &CheckOptions::default()));
    assert!(rendered.contains("✓"), "{rendered}");
}

#[test]
fn json出力は安定したスキーマを持つ() {
    let value: Value =
        serde_json::from_str(&render_json(&check("undefined_character/scene.md"))).unwrap();
    assert_eq!(value["status"], "ok", "warning のみなら ok");
    assert_eq!(value["error_count"], 0);
    assert_eq!(value["warning_count"], 1);
    let diag = &value["diagnostics"][0];
    assert_eq!(diag["rule_id"], "undefined-character");
    assert_eq!(diag["severity"], "warning");
    assert_eq!(diag["span"]["line"], 7);
    assert!(diag["file"].as_str().unwrap().ends_with("scene.md"));
}

#[test]
fn json出力はエラー時も形式が崩れない() {
    let value: Value = serde_json::from_str(&render_json(&check("no_such_path.md"))).unwrap();
    assert_eq!(value["status"], "error");
    assert_eq!(value["diagnostics"][0]["rule_id"], "io-error");
    assert_eq!(value["diagnostics"][0]["span"], Value::Null);
}

#[test]
fn sarif出力はcode_scanningが要求する構造を持つ() {
    let value: Value = serde_json::from_str(&render_sarif(&check("typo_anchor/scene.md"))).unwrap();
    assert_eq!(value["version"], "2.1.0");
    let driver = &value["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "tsumugai");
    assert!(
        driver["rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "broken-link")
    );
    let result = &value["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "broken-link");
    assert_eq!(result["level"], "error");
    let location = &result["locations"][0]["physicalLocation"];
    assert_eq!(location["region"]["startLine"], 9);
    let uri = location["artifactLocation"]["uri"].as_str().unwrap();
    assert!(!uri.contains('\\'), "uri は / 区切り: {uri}");
    assert!(uri.ends_with("scene.md"), "{uri}");
    assert!(
        result["message"]["text"].as_str().unwrap().contains("提案"),
        "suggestion は message に併記する"
    );
}
