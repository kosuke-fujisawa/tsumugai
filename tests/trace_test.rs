//! tsumugai trace（#77）の統合テスト
//!
//! SPEC 5.1「経路の再現」の仕様を検証する。
//! examples/spring を正常系サンプル、tests/fixtures/trace/ を
//! 異常系（ループ・暗黙終了・リンク切れ）の入力例として使う。

use std::path::Path;
use tsumugai::scenario::{
    TraceEnd, TraceOptions, TraceStep, render_trace_human, render_trace_json, trace_path,
};

fn spring() -> &'static Path {
    Path::new("examples/spring/scenario/spring_001.md")
}

fn options(choices: &[usize]) -> TraceOptions {
    TraceOptions {
        choices: choices.to_vec(),
        ..TraceOptions::default()
    }
}

// ------------------------------------------------------------ 正常系の経路

#[test]
fn 選択肢に到達すると入力待ちで停止する() {
    let result = trace_path(spring(), &TraceOptions::default());

    assert!(!result.check.has_errors());
    let trace = result.trace.as_ref().expect("check が通れば trace は返る");
    assert!(matches!(trace.end, TraceEnd::AwaitingChoice));

    // 最後のステップは未選択の選択肢ブロック（3 項目）
    let Some(TraceStep::Choice {
        options, selected, ..
    }) = trace.steps.last()
    else {
        panic!("最後のステップが選択肢ではない: {:?}", trace.steps.last());
    };
    assert_eq!(options.len(), 3);
    assert_eq!(*selected, None);

    // 停止するまでにナレーションとセリフが記録されている
    assert!(
        trace
            .steps
            .iter()
            .any(|s| matches!(s, TraceStep::Narration { .. }))
    );
    assert!(
        trace
            .steps
            .iter()
            .any(|s| matches!(s, TraceStep::Dialogue { speaker, .. } if speaker == "幼なじみ"))
    );
}

#[test]
fn シーン進入で表示タイトルと素材が記録される() {
    let result = trace_path(spring(), &TraceOptions::default());
    let trace = result.trace.as_ref().unwrap();

    let Some(TraceStep::SceneEnter {
        id,
        title,
        background,
        bgm,
        ..
    }) = trace.steps.first()
    else {
        panic!(
            "最初のステップがシーン進入ではない: {:?}",
            trace.steps.first()
        );
    };
    assert_eq!(id.as_deref(), Some("spring_001"));
    assert_eq!(title.as_deref(), Some("春・出会い"));
    assert_eq!(background.as_deref(), Some("../assets/bg/school_gate.png"));
    assert_eq!(bgm.as_deref(), Some("../assets/bgm/spring.ogg"));
}

#[test]
fn リード部からセクションへのフォールスルーが記録される() {
    let result = trace_path(spring(), &TraceOptions::default());
    let trace = result.trace.as_ref().unwrap();

    // リード部の後、選択肢ブロックを含むセクション「選択肢」に進入している
    assert!(
        trace
            .steps
            .iter()
            .any(|s| matches!(s, TraceStep::SectionEnter { anchor, .. } if anchor == "選択肢"))
    );
}

#[test]
fn choicesで経路を再現しエンディングに到達する() {
    let result = trace_path(spring(), &options(&[1]));
    let trace = result.trace.as_ref().unwrap();

    assert!(
        matches!(&trace.end, TraceEnd::Ending { id } if id == "childhood_route"),
        "end: {:?}",
        trace.end
    );
    // 選択肢ステップに「1 番を選んだ」と記録される
    assert!(trace.steps.iter().any(|s| matches!(
        s,
        TraceStep::Choice {
            selected: Some(1),
            ..
        }
    )));
    // 選んだ先のセリフが実行されている
    assert!(
        trace
            .steps
            .iter()
            .any(|s| matches!(s, TraceStep::Dialogue { text, .. } if text.contains("急ぐよ")))
    );
}

#[test]
fn ファイルをまたぐ選択で次のシーンに進入する() {
    // 3 番「先に行ってもらう」→ spring_002.md → 1 番「休み時間に話しかける」
    let result = trace_path(spring(), &options(&[3, 1]));
    let trace = result.trace.as_ref().unwrap();

    assert!(trace.steps.iter().any(
        |s| matches!(s, TraceStep::SceneEnter { id, .. } if id.as_deref() == Some("spring_002"))
    ));
    assert!(matches!(&trace.end, TraceEnd::Ending { id } if id == "sprint_route"));
    assert_eq!(trace.choices_used, 2);
}

#[test]
fn ジャンプ段落で同一ファイル内を移動する() {
    // 2 番「放課後まで待つ」→ wait-until-after-school → ジャンプで after-school へ
    let result = trace_path(spring(), &options(&[3, 2]));
    let trace = result.trace.as_ref().unwrap();

    assert!(
        trace
            .steps
            .iter()
            .any(|s| matches!(s, TraceStep::Jump { target, .. } if target == "#after-school"))
    );
    assert!(matches!(&trace.end, TraceEnd::Ending { id } if id == "calm_route"));
}

// ------------------------------------------------------------ choices の消費

#[test]
fn 未消費の選択番号が記録される() {
    let result = trace_path(spring(), &options(&[1, 1]));
    let trace = result.trace.as_ref().unwrap();

    assert!(matches!(trace.end, TraceEnd::Ending { .. }));
    assert_eq!(trace.choices_requested, vec![1, 1]);
    assert_eq!(trace.choices_used, 1);
    // 経路は再現できているのでエラーではない
    assert!(!result.has_errors());
}

#[test]
fn 範囲外の選択番号はエラーになる() {
    let result = trace_path(spring(), &options(&[9]));
    let trace = result.trace.as_ref().unwrap();

    assert!(
        matches!(
            trace.end,
            TraceEnd::InvalidChoice {
                given: 9,
                available: 3
            }
        ),
        "end: {:?}",
        trace.end
    );
    assert!(result.has_errors());
}

// ------------------------------------------------------------ 終了と保護

#[test]
fn ファイル末尾で暗黙に終了する() {
    let result = trace_path(
        Path::new("tests/fixtures/trace/eof/scenario.md"),
        &TraceOptions::default(),
    );
    let trace = result.trace.as_ref().unwrap();
    assert!(matches!(trace.end, TraceEnd::EndOfFile));
    assert!(!result.has_errors());
}

#[test]
fn ジャンプループは上限で打ち切られエラーになる() {
    let result = trace_path(
        Path::new("tests/fixtures/trace/loop/scenario.md"),
        &TraceOptions::default(),
    );
    let trace = result.trace.as_ref().unwrap();
    assert!(matches!(trace.end, TraceEnd::Truncated { .. }));
    assert!(result.has_errors());
}

// ------------------------------------------------------------ 実行前検査

#[test]
fn checkエラーがあると実行せず診断を返す() {
    let result = trace_path(
        Path::new("tests/fixtures/trace/broken/scenario.md"),
        &TraceOptions::default(),
    );

    assert!(result.check.has_errors());
    assert!(result.trace.is_none());
    assert!(result.has_errors());
    assert!(
        result
            .check
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "broken-link")
    );
}

// ------------------------------------------------------------ 出力形式

#[test]
fn json出力は安定形式でエンディングを保持する() {
    let result = trace_path(spring(), &options(&[1]));
    let json: serde_json::Value = serde_json::from_str(&render_trace_json(&result)).unwrap();

    assert_eq!(json["status"], "ok");
    assert!(json["diagnostics"].is_array());
    let steps = json["trace"]["steps"].as_array().unwrap();
    assert!(!steps.is_empty());
    assert_eq!(steps[0]["type"], "scene_enter");
    assert_eq!(json["trace"]["end"]["reason"], "ending");
    assert_eq!(json["trace"]["end"]["id"], "childhood_route");
}

#[test]
fn 存在しないファイルでもjson形式が崩れない() {
    let result = trace_path(Path::new("no/such/file.md"), &TraceOptions::default());
    let json: serde_json::Value = serde_json::from_str(&render_trace_json(&result)).unwrap();

    assert_eq!(json["status"], "error");
    assert!(json["trace"].is_null());
    let diagnostics = json["diagnostics"].as_array().unwrap();
    assert!(!diagnostics.is_empty());
    assert_eq!(diagnostics[0]["rule_id"], "io-error");
}

#[test]
fn 人間向け出力に経路と到達エンディングが含まれる() {
    let result = trace_path(spring(), &options(&[1]));
    let human = render_trace_human(&result);

    assert!(human.contains("春・出会い"), "出力: {human}");
    assert!(human.contains("一緒に走る"), "出力: {human}");
    assert!(human.contains("childhood_route"), "出力: {human}");
}

#[test]
fn 入力待ちで停止したとき選択肢一覧と番号が表示される() {
    let result = trace_path(spring(), &TraceOptions::default());
    let human = render_trace_human(&result);

    // 番号付きの選択肢一覧（SPEC 5.1: 次に足す番号が分かる）
    assert!(human.contains("1."), "出力: {human}");
    assert!(human.contains("先に行ってもらう"), "出力: {human}");
    assert!(human.contains("--choices"), "出力: {human}");
}
