//! `scenario::compile_path`（#128）と `tsumugai compile`（#135）の統合テスト
//!
//! StoryBundle JSON 生成の仕様を検証する。examples/spring を全ブロック種別
//! （narration / dialogue / choice / jump / ending）を含む正常系サンプルとして
//! 使い、tests/fixtures/ 配下の check 用フィクスチャを失敗系に流用する。
//!
//! Golden JSON（tests/fixtures/compile/golden/spring_001.json）と CLI
//! プロセスを実際に起動するテストは #135 で追加した。前者は意図しない出力
//! 変化を検出し、後者は「エラー時に出力ファイルを書き出さない」という
//! main.rs 側の制御フローを、compile_path の戻り値だけでなく実際の
//! ファイルシステムへの影響で確認する。

use std::path::{Path, PathBuf};
use std::process::Command;
use tsumugai::scenario::{BundleAsset, BundleStep, CompileOptions, StepTarget, compile_path};

fn spring() -> &'static Path {
    Path::new("examples/spring/scenario/spring_001.md")
}

// -------------------------------------------------------------- 正常系

#[test]
fn spring例からstorybundleを生成できる() {
    let result = compile_path(spring(), &CompileOptions::default());
    assert!(!result.has_errors());
    let bundle = result
        .bundle
        .as_ref()
        .expect("check が通れば bundle が返る");

    assert_eq!(bundle.schema_version, "1");
    assert_eq!(bundle.entry_scene_id, "spring_001");
    assert_eq!(bundle.title, "春・出会い");
    assert!(!bundle.story_build_id.is_empty());

    let mut scene_ids: Vec<&str> = bundle.scenes.iter().map(|s| s.id.as_str()).collect();
    scene_ids.sort();
    assert_eq!(scene_ids, vec!["spring_001", "spring_002"]);
}

#[test]
fn 全ブロック種別がstepに変換される() {
    let result = compile_path(spring(), &CompileOptions::default());
    let bundle = result.bundle.as_ref().unwrap();
    let spring_001 = bundle
        .scenes
        .iter()
        .find(|s| s.id == "spring_001")
        .expect("spring_001 が bundle に含まれる");

    assert_eq!(spring_001.steps.len(), 11);
    assert!(
        spring_001
            .steps
            .iter()
            .any(|s| matches!(s, BundleStep::Narration { .. }))
    );
    assert!(
        spring_001
            .steps
            .iter()
            .any(|s| matches!(s, BundleStep::Dialogue { .. }))
    );
    assert!(
        spring_001
            .steps
            .iter()
            .any(|s| matches!(s, BundleStep::Choice { .. }))
    );
    assert!(
        spring_001
            .steps
            .iter()
            .any(|s| matches!(s, BundleStep::Jump { .. }))
    );
    assert!(
        spring_001
            .steps
            .iter()
            .any(|s| matches!(s, BundleStep::Ending { .. }))
    );
}

#[test]
fn ファイルをまたぐジャンプはscene_idとstep_indexに解決される() {
    let result = compile_path(spring(), &CompileOptions::default());
    let bundle = result.bundle.as_ref().unwrap();
    let spring_001 = bundle.scenes.iter().find(|s| s.id == "spring_001").unwrap();
    let spring_002 = bundle.scenes.iter().find(|s| s.id == "spring_002").unwrap();

    let jump_target = spring_001
        .steps
        .iter()
        .find_map(|s| match s {
            BundleStep::Jump { target, .. } if target.scene_id == "spring_002" => Some(target),
            _ => None,
        })
        .expect("spring_002 への jump がある（walk-together → after-school）");

    assert_eq!(
        jump_target,
        &StepTarget {
            scene_id: "spring_002".to_string(),
            step_index: 8,
        }
    );
    // after-school セクションの最初のステップは Dialogue
    assert!(matches!(
        spring_002.steps[jump_target.step_index],
        BundleStep::Dialogue { .. }
    ));
}

#[test]
fn 同一ファイル内の選択肢は自身のscene_idとstep_indexに解決される() {
    let result = compile_path(spring(), &CompileOptions::default());
    let bundle = result.bundle.as_ref().unwrap();
    let spring_001 = bundle.scenes.iter().find(|s| s.id == "spring_001").unwrap();

    let choice_items = spring_001
        .steps
        .iter()
        .find_map(|s| match s {
            BundleStep::Choice { items, .. } => Some(items),
            _ => None,
        })
        .expect("選択肢ステップがある");
    assert_eq!(choice_items.len(), 3);

    let run_together = choice_items
        .iter()
        .find(|i| i.label == "一緒に走る")
        .unwrap();
    assert_eq!(run_together.target.scene_id, "spring_001");
    assert!(matches!(
        spring_001.steps[run_together.target.step_index],
        BundleStep::Dialogue { .. }
    ));

    let leave_first = choice_items
        .iter()
        .find(|i| i.label == "先に行ってもらう")
        .unwrap();
    assert_eq!(leave_first.target.scene_id, "spring_002");
    assert_eq!(leave_first.target.step_index, 0);
}

#[test]
fn assetはファイル横断で重複排除して収集される() {
    let result = compile_path(spring(), &CompileOptions::default());
    let bundle = result.bundle.as_ref().unwrap();

    assert_eq!(bundle.assets.len(), 3);
    assert!(bundle.assets.contains(&BundleAsset::Background {
        path: "../assets/bg/school_gate.png".to_string()
    }));
    assert!(bundle.assets.contains(&BundleAsset::Background {
        path: "../assets/bg/classroom.png".to_string()
    }));
    assert!(bundle.assets.contains(&BundleAsset::Bgm {
        path: "../assets/bgm/spring.ogg".to_string()
    }));
}

#[test]
fn 同じ入力からは同じstory_build_idが生成される() {
    let a = compile_path(spring(), &CompileOptions::default());
    let b = compile_path(spring(), &CompileOptions::default());
    assert_eq!(
        a.bundle.unwrap().story_build_id,
        b.bundle.unwrap().story_build_id
    );
}

// -------------------------------------------------------------- 異常系

#[test]
fn checkエラーがあるとbundleを生成せず診断を返す() {
    let result = compile_path(
        Path::new("tests/fixtures/trace/broken/scenario.md"),
        &CompileOptions::default(),
    );

    assert!(result.check.has_errors());
    assert!(result.bundle.is_none());
    assert!(result.has_errors());
}

#[test]
fn 存在しないassetがあるとbundleを生成しない() {
    let result = compile_path(
        Path::new("tests/fixtures/check/missing_asset/scene.md"),
        &CompileOptions::default(),
    );

    assert!(result.check.has_errors());
    assert!(result.bundle.is_none());
    assert!(
        result
            .check
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "missing-asset")
    );
}

#[test]
fn ディレクトリを渡すとio_errorになる() {
    let result = compile_path(Path::new("examples/spring"), &CompileOptions::default());
    assert!(result.check.has_errors());
    assert!(result.bundle.is_none());
    assert!(
        result
            .check
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "io-error")
    );
}

// -------------------------------------------------------------- routes相当の検証（#144）

#[test]
fn 循環するシナリオはcheckを通ってもcompileが失敗する() {
    // check だけでは検出できず routes の全分岐探索で初めて分かる不具合
    // （エンディングに一切到達しない無限ループ）を compile が見逃さないことを確認する
    let result = compile_path(
        Path::new("tests/fixtures/trace/loop/scenario.md"),
        &CompileOptions::default(),
    );

    assert!(result.bundle.is_none());
    assert!(result.has_errors());
    assert!(
        result
            .check
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "circular-route"),
        "diagnostics: {:?}",
        result.check.diagnostics
    );
}

#[test]
fn 到達不能endingがあってもcompileは成功しつつ警告を報告する() {
    // circular-route（error）と違い、到達不能は warning 相当なので
    // StoryBundle の生成自体は妨げない
    let result = compile_path(
        Path::new("tests/fixtures/routes/unreachable/entry.md"),
        &CompileOptions::default(),
    );

    assert!(!result.has_errors());
    assert!(result.bundle.is_some());
    assert!(
        result
            .check
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "unreachable-ending"),
        "diagnostics: {:?}",
        result.check.diagnostics
    );
}

#[test]
fn cliで循環するシナリオがあると出力ファイルを生成せず失敗する() {
    let output = unique_output_path("circular-route");
    let _ = std::fs::remove_file(&output);

    let result = run_compile("tests/fixtures/trace/loop/scenario.md", &output);
    assert!(!result.status.success());
    assert!(
        !output.exists(),
        "circular-route（error）がある場合は出力ファイルを書き出さない"
    );
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("circular-route"),
        "stdout: {}",
        String::from_utf8_lossy(&result.stdout)
    );
}

#[test]
fn エンディング未宣言のシナリオはcompileが成功しつつ警告を報告する() {
    // #147: routes_pathがroute-without-endingを報告するようになったことを
    // compile側にも波及していることを確認する（#144でroutesを組み込み済み）
    let result = compile_path(
        Path::new("tests/fixtures/trace/eof/scenario.md"),
        &CompileOptions::default(),
    );

    assert!(!result.has_errors());
    assert!(result.bundle.is_some());
    assert!(
        result
            .check
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "route-without-ending"),
        "diagnostics: {:?}",
        result.check.diagnostics
    );
}

#[test]
fn cliで到達不能endingがあってもwarningを表示しつつ出力ファイルを書き出す() {
    let output = unique_output_path("unreachable-ending");
    let _ = std::fs::remove_file(&output);

    let result = run_compile("tests/fixtures/routes/unreachable/entry.md", &output);
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        output.exists(),
        "warning のみなら出力ファイルは書き出される"
    );
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("unreachable-ending"),
        "stdout: {}",
        String::from_utf8_lossy(&result.stdout)
    );

    let _ = std::fs::remove_file(&output);
}

// -------------------------------------------------------------- choiceからendingまでのroute

/// [`StepTarget`] だけを頼りに、選択肢ブロックで指定した項目を選びながら
/// ending に到達するまで歩く。routes.rs 自身の探索アルゴリズム
/// （Cursor / goto、パース結果の Scene を直接辿る）とは別に、
/// **compile が出力する StoryBundle 単体で経路を再現できること**を確認する
fn walk_to_ending(bundle: &tsumugai::scenario::StoryBundle, choose: &[&str]) -> String {
    let mut scene_id = bundle.entry_scene_id.clone();
    let mut step_index = 0usize;
    let mut choose = choose.iter();
    loop {
        let scene = bundle
            .scenes
            .iter()
            .find(|s| s.id == scene_id)
            .unwrap_or_else(|| panic!("scene {scene_id} が bundle にある"));
        match &scene.steps[step_index] {
            BundleStep::Narration { .. } | BundleStep::Dialogue { .. } => step_index += 1,
            BundleStep::Ending { id, .. } => return id.clone(),
            BundleStep::Jump { target, .. } => {
                scene_id = target.scene_id.clone();
                step_index = target.step_index;
            }
            BundleStep::Choice { items, .. } => {
                let label = choose.next().expect("choose に選択肢の数だけラベルを渡す");
                let item = items
                    .iter()
                    .find(|i| i.label == *label)
                    .unwrap_or_else(|| panic!("選択肢「{label}」が見つかる"));
                scene_id = item.target.scene_id.clone();
                step_index = item.target.step_index;
            }
        }
    }
}

#[test]
fn choiceからendingまでの経路をbundle単体で再現できる() {
    let result = compile_path(spring(), &CompileOptions::default());
    let bundle = result.bundle.as_ref().unwrap();

    assert_eq!(walk_to_ending(bundle, &["一緒に走る"]), "childhood_route");
    assert_eq!(
        walk_to_ending(bundle, &["先に行ってもらう", "休み時間に話しかける"]),
        "sprint_route"
    );
    assert_eq!(
        walk_to_ending(bundle, &["先に行ってもらう", "放課後まで待つ"]),
        "calm_route"
    );
}

// -------------------------------------------------------------- Golden JSON

fn golden_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/compile/golden/spring_001.json")
}

#[test]
fn spring例のstorybundleはgolden_jsonと一致する() {
    let result = compile_path(spring(), &CompileOptions::default());
    let bundle = result.bundle.as_ref().unwrap();
    let actual: serde_json::Value = serde_json::to_value(bundle).unwrap();

    let golden_text = std::fs::read_to_string(golden_fixture()).expect(
        "tests/fixtures/compile/golden/spring_001.json が読める（先に生成してコミットする）",
    );
    let expected: serde_json::Value = serde_json::from_str(&golden_text).unwrap();

    assert_eq!(
        actual, expected,
        "StoryBundle の出力が変化した。意図した変更なら tests/fixtures/compile/golden/spring_001.json を更新すること"
    );
}

// -------------------------------------------------------------- CLIプロセス経由の確認

fn unique_output_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tsumugai-compile-test-{}-{}.json",
        name,
        std::process::id()
    ))
}

fn run_compile(scenario: &str, output: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_tsumugai"))
        .args([
            "compile",
            scenario,
            "--target",
            "web",
            "--output",
            output.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("tsumugai バイナリを起動できる")
}

#[test]
fn cli正常系はgolden_jsonと同じ内容のファイルを書き出す() {
    let output = unique_output_path("success");
    let _ = std::fs::remove_file(&output);

    let result = run_compile("examples/spring/scenario/spring_001.md", &output);
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output).expect("出力ファイルができる"))
            .unwrap();
    let golden: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(golden_fixture()).unwrap()).unwrap();
    assert_eq!(written, golden);

    let _ = std::fs::remove_file(&output);
}

#[test]
fn cliで不正なjump先があると出力ファイルを生成せず失敗する() {
    let output = unique_output_path("broken-jump");
    let _ = std::fs::remove_file(&output);

    let result = run_compile("tests/fixtures/trace/broken/scenario.md", &output);
    assert!(!result.status.success());
    assert!(
        !output.exists(),
        "check エラー時は出力ファイルを書き出さない"
    );
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("broken-link"),
        "stdout: {}",
        String::from_utf8_lossy(&result.stdout)
    );
}

#[test]
fn cliで存在しないasset参照があると出力ファイルを生成せず失敗する() {
    let output = unique_output_path("missing-asset");
    let _ = std::fs::remove_file(&output);

    let result = run_compile("tests/fixtures/check/missing_asset/scene.md", &output);
    assert!(!result.status.success());
    assert!(
        !output.exists(),
        "check エラー時は出力ファイルを書き出さない"
    );
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("missing-asset"),
        "stdout: {}",
        String::from_utf8_lossy(&result.stdout)
    );
}
