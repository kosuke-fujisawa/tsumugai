//! `scenario::compile_path`（#128）の統合テスト
//!
//! StoryBundle JSON 生成の仕様を検証する。examples/spring を全ブロック種別
//! （narration / dialogue / choice / jump / ending）を含む正常系サンプルとして
//! 使い、tests/fixtures/ 配下の check 用フィクスチャを失敗系に流用する。

use std::path::Path;
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
