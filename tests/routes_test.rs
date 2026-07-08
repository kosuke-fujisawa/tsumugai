//! tsumugai routes（#78）の統合テスト
//!
//! SPEC 5.2「全分岐探索」の仕様を検証する。
//! examples/spring を全分岐が既知の正常系サンプルとして使い、
//! tests/fixtures/routes/ と tests/fixtures/trace/ を異常系
//! （到達不能・経路数上限・循環・深度超過・check エラー）の入力例に使う。

use std::path::Path;
use tsumugai::scenario::{
    RouteEnd, RoutesOptions, render_routes_human, render_routes_json, routes_path,
};

fn spring() -> &'static Path {
    Path::new("examples/spring/scenario/spring_001.md")
}

fn options(max_routes: usize, max_depth: usize) -> RoutesOptions {
    RoutesOptions {
        max_routes,
        max_depth,
        ..RoutesOptions::default()
    }
}

// ------------------------------------------------------------ 正常系の探索

#[test]
fn spring例で4経路すべてを発見しendingを網羅する() {
    let result = routes_path(spring(), &RoutesOptions::default());
    assert!(!result.check.has_errors());
    let report = result
        .report
        .as_ref()
        .expect("check が通れば report は返る");

    assert_eq!(report.routes.len(), 4);
    assert!(!report.truncated);
    assert!(!result.has_errors());

    let mut choice_lists: Vec<&Vec<usize>> = report.routes.iter().map(|r| &r.choices).collect();
    choice_lists.sort();
    assert_eq!(
        choice_lists,
        vec![&vec![1usize], &vec![2], &vec![3, 1], &vec![3, 2]]
    );

    let mut reached = report.reached_endings.clone();
    reached.sort();
    assert_eq!(
        reached,
        vec!["calm_route", "childhood_route", "sprint_route"]
    );
    assert!(report.unreached_endings.is_empty());
    assert!(report.unreachable_scenes.is_empty());
}

#[test]
fn 選択番号列はtraceのchoicesにそのまま渡せる形式になっている() {
    let result = routes_path(spring(), &RoutesOptions::default());
    let report = result.report.as_ref().unwrap();

    let route = report
        .routes
        .iter()
        .find(|r| r.choices == vec![3, 1])
        .expect("[3,1] の経路が見つかる");
    assert!(matches!(&route.end, RouteEnd::Ending { id } if id == "sprint_route"));
}

// ------------------------------------------------------------ 到達不能の検出

#[test]
fn 動的に到達しないシーンとendingが報告される() {
    let result = routes_path(
        Path::new("tests/fixtures/routes/unreachable/entry.md"),
        &RoutesOptions::default(),
    );
    // orphan セクションは check の unreachable-section warning が出るが、
    // error ではないため routes は実行される
    assert!(!result.check.has_errors());
    let report = result.report.as_ref().unwrap();

    assert_eq!(report.routes.len(), 1);
    assert_eq!(report.reached_endings, vec!["main_end".to_string()]);
    assert_eq!(report.unreached_endings, vec!["sibling_end".to_string()]);
    assert_eq!(
        report.unreachable_scenes,
        vec![Path::new("tests/fixtures/routes/unreachable/sibling.md").to_path_buf()]
    );
    // 到達不能は warning 相当であり、循環や check エラーではないため exit は 0
    assert!(!result.has_errors());
}

// ------------------------------------------------------------ 循環・深度超過

#[test]
fn エンディングを宣言しないままファイル末尾に達するとroute_without_endingのwarningになる() {
    // #147: 書き忘れ（<!-- ending: id --> を一切書かないままファイルが終わる）を
    // 無警告のexit 0で見逃さないことを確認する
    let result = routes_path(
        Path::new("tests/fixtures/trace/eof/scenario.md"),
        &RoutesOptions::default(),
    );
    let report = result.report.as_ref().unwrap();

    assert_eq!(report.routes.len(), 1);
    assert!(matches!(report.routes[0].end, RouteEnd::EndOfFile));
    assert!(!result.has_errors(), "warning相当なのでexitは0のまま");
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "route-without-ending"
                && d.severity == tsumugai::scenario::Severity::Warning),
        "diagnostics: {:?}",
        report.diagnostics
    );
}

#[test]
fn 循環はcircular_routeとしてerror扱いになる() {
    let result = routes_path(
        Path::new("tests/fixtures/trace/loop/scenario.md"),
        &RoutesOptions::default(),
    );
    let report = result.report.as_ref().unwrap();

    assert_eq!(report.routes.len(), 1);
    assert!(matches!(report.routes[0].end, RouteEnd::Circular));
    assert!(result.has_errors());
    assert!(report.diagnostics.iter().any(
        |d| d.rule_id == "circular-route" && d.severity == tsumugai::scenario::Severity::Error
    ));
}

#[test]
fn 深度上限に達するとmax_depth_exceededになりwarning扱いになる() {
    // eof フィクスチャ（ナレーション1件のみ）に極端に小さい max_depth を与えて
    // 上限機構そのものを検証する（現実的な深さの経路を用意する必要はない）
    let result = routes_path(
        Path::new("tests/fixtures/trace/eof/scenario.md"),
        &options(1000, 1),
    );
    let report = result.report.as_ref().unwrap();

    assert_eq!(report.routes.len(), 1);
    assert!(matches!(
        report.routes[0].end,
        RouteEnd::MaxDepthExceeded { .. }
    ));
    // 深度超過は warning 相当（循環ほど確実な不具合ではないため）
    assert!(!result.has_errors());
}

// ------------------------------------------------------------ 経路数の上限

#[test]
fn 経路数の上限に達すると打ち切りtruncatedになる() {
    let result = routes_path(
        Path::new("tests/fixtures/routes/two_choices/entry.md"),
        &options(2, 1000),
    );
    let report = result.report.as_ref().unwrap();

    assert_eq!(report.routes.len(), 2);
    assert!(report.truncated);
    // 打ち切りは警告相当（circular ではない）
    assert!(!result.has_errors());
}

// ------------------------------------------------------------ 実行前検査

#[test]
fn checkエラーがあると探索せず診断を返す() {
    let result = routes_path(
        Path::new("tests/fixtures/trace/broken/scenario.md"),
        &RoutesOptions::default(),
    );

    assert!(result.check.has_errors());
    assert!(result.report.is_none());
    assert!(result.has_errors());
}

#[test]
fn ディレクトリを渡すとio_errorになる() {
    let result = routes_path(Path::new("examples/spring"), &RoutesOptions::default());
    assert!(result.check.has_errors());
    assert!(result.report.is_none());
    assert!(
        result
            .check
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "io-error")
    );
}

// ------------------------------------------------------------ 出力形式

#[test]
fn json出力は安定形式で経路と到達可能性を保持する() {
    let result = routes_path(spring(), &RoutesOptions::default());
    let json: serde_json::Value = serde_json::from_str(&render_routes_json(&result)).unwrap();

    assert_eq!(json["status"], "ok");
    assert_eq!(json["report"]["routes"].as_array().unwrap().len(), 4);
    let reached = json["report"]["reached_endings"].as_array().unwrap();
    assert_eq!(reached.len(), 3);
    assert_eq!(json["report"]["truncated"], false);
}

#[test]
fn 存在しないファイルでもjson形式が崩れない() {
    let result = routes_path(Path::new("no/such/file.md"), &RoutesOptions::default());
    let json: serde_json::Value = serde_json::from_str(&render_routes_json(&result)).unwrap();

    assert_eq!(json["status"], "error");
    assert!(json["report"].is_null());
    let diagnostics = json["diagnostics"].as_array().unwrap();
    assert!(!diagnostics.is_empty());
    assert_eq!(diagnostics[0]["rule_id"], "io-error");
}

#[test]
fn 人間向け出力に経路一覧とendingの到達可否が含まれる() {
    let result = routes_path(spring(), &RoutesOptions::default());
    let human = render_routes_human(&result);

    assert!(human.contains("--choices 3,1"), "出力: {human}");
    assert!(human.contains("sprint_route"), "出力: {human}");
    assert!(human.contains("発見した経路数: 4"), "出力: {human}");
}

#[test]
fn 到達不能endingとシーンが人間向け出力に含まれる() {
    let result = routes_path(
        Path::new("tests/fixtures/routes/unreachable/entry.md"),
        &RoutesOptions::default(),
    );
    let human = render_routes_human(&result);

    assert!(human.contains("sibling_end"), "出力: {human}");
    assert!(human.contains("sibling.md"), "出力: {human}");
}
