//! 全分岐探索（SPEC 5.2「全分岐探索」）
//!
//! [`routes_path`] は選択肢ブロックのすべての項目を辿ることでプロジェクト内の
//! 全経路を DFS で探索し、到達可能な ending・到達不能な ending / シーンを
//! 報告する。[`trace`](super::trace) が `--choices` で指定した 1 経路だけを
//! 再現するのに対し、routes はすべての分岐を機械的に網羅する。
//!
//! - 実行前に check と同じ検査を行い、error があれば探索しない（SPEC 6.1）
//! - 経路は「選択番号列」（`tsumugai trace --choices` にそのまま渡せる形式）
//!   として表現する
//! - 循環（同一経路内で同じ地点に再到達）は error、それ以外（到達不能
//!   ending / シーン・深度超過・経路数上限による打ち切り）は warning

use super::Block;
use super::check::{CheckOptions, CheckResult, check_path};
use super::diagnostic::{Diagnostic, Severity};
use super::exec::{Cursor, format_choices, goto, segment_blocks};
use super::project::{LoadedScene, file_level, load_project};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 既定の経路数上限（SPEC 5.2: 探索が無限に分岐してもハングしない）
const DEFAULT_MAX_ROUTES: usize = 1000;
/// 既定の 1 経路あたりの深度上限（循環検出をすり抜けた場合の保護）
const DEFAULT_MAX_DEPTH: usize = 1000;

/// routes の動作オプション
#[derive(Debug, Clone)]
pub struct RoutesOptions {
    /// background / bgm の実在チェック（`--no-assets` で false）
    pub check_assets: bool,
    /// 探索する経路の総数の上限
    pub max_routes: usize,
    /// 1 経路あたりのステップ数の上限
    pub max_depth: usize,
}

impl Default for RoutesOptions {
    fn default() -> Self {
        Self {
            check_assets: true,
            max_routes: DEFAULT_MAX_ROUTES,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

/// 1 経路の終わり方
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum RouteEnd {
    /// `<!-- ending: id -->` に到達した
    Ending { id: String },
    /// ファイル末尾に到達した（暗黙の終了、SPEC 5章）
    EndOfFile,
    /// 同一経路内で以前と同じ地点に再到達した（error）
    Circular,
    /// 1 経路のステップ数が上限に達した（warning）
    MaxDepthExceeded { max_depth: usize },
}

/// 探索で見つかった 1 経路
#[derive(Debug, Clone, Serialize)]
pub struct RouteRecord {
    /// 選択肢ブロックで選んだ項目の並び順（1 始まり）。
    /// `tsumugai trace --choices` にそのまま渡せる
    pub choices: Vec<usize>,
    pub end: RouteEnd,
}

/// 全分岐探索の結果
#[derive(Debug, Serialize)]
pub struct RoutesReport {
    pub routes: Vec<RouteRecord>,
    /// 到達できた ending id（重複なし、ソート済み）
    pub reached_endings: Vec<String>,
    /// プロジェクトに宣言されているが、どの経路からも到達できない ending id
    pub unreached_endings: Vec<String>,
    /// プロジェクトに読み込まれているが、どの経路からも実行されないシーン
    pub unreachable_scenes: Vec<PathBuf>,
    /// 経路数の上限に達し、探索を打ち切ったか
    pub truncated: bool,
    /// circular-route / unreachable-ending / unreachable-scene /
    /// route-limit-exceeded / route-max-depth-exceeded の Diagnostic
    #[serde(skip)]
    pub diagnostics: Vec<Diagnostic>,
}

/// routes の結果。実行前検査の結果（check）を必ず含む
#[derive(Debug)]
pub struct RoutesResult {
    /// 開始シーンとして指定されたパス
    pub file: PathBuf,
    /// 実行前検査（check と同じ規則）の結果
    pub check: CheckResult,
    /// 探索結果。check が error のときは None
    pub report: Option<RoutesReport>,
}

impl RoutesResult {
    /// exit code を 1 にすべきか（check エラー、または循環の検出）
    pub fn has_errors(&self) -> bool {
        self.check.has_errors()
            || self
                .report
                .as_ref()
                .is_some_and(|r| r.diagnostics.iter().any(|d| d.severity == Severity::Error))
    }
}

/// シーンファイルを実行前検査してから全分岐を探索する（SPEC 5.2）。
///
/// パスが存在しない・ディレクトリ・検査 error の場合も panic や Err にせず、
/// Diagnostic 入りの [`RoutesResult`] を返す（report は None になる）。
pub fn routes_path(path: &Path, options: &RoutesOptions) -> RoutesResult {
    if path.is_dir() {
        let diag = file_level(
            "io-error",
            Severity::Error,
            path,
            format!(
                "{} はディレクトリです。routes は開始するシーンファイル（.md）を 1 つ指定してください",
                path.display()
            ),
        );
        return RoutesResult {
            file: path.to_path_buf(),
            check: CheckResult {
                files: Vec::new(),
                diagnostics: vec![diag],
            },
            report: None,
        };
    }

    let check_options = CheckOptions {
        check_assets: options.check_assets,
    };
    let mut check = check_path(path, &check_options);
    if check.has_errors() {
        return RoutesResult {
            file: path.to_path_buf(),
            check,
            report: None,
        };
    }

    let mut load_diagnostics = Vec::new();
    let scenes = load_project(vec![path.to_path_buf()], &mut load_diagnostics);
    check.diagnostics.extend(load_diagnostics);
    if check.has_errors() || scenes.is_empty() {
        return RoutesResult {
            file: path.to_path_buf(),
            check,
            report: None,
        };
    }

    let report = explore(&scenes, path, options);
    RoutesResult {
        file: path.to_path_buf(),
        check,
        report: Some(report),
    }
}

// ---------------------------------------------------------------- 探索

struct Explorer<'a> {
    scenes: &'a [LoadedScene],
    max_routes: usize,
    max_depth: usize,
    routes: Vec<RouteRecord>,
    visited_scenes: HashSet<usize>,
    truncated: bool,
}

impl Explorer<'_> {
    /// 1 つの実行位置から、経路が終わる（または上限に達する）まで進む。
    /// 選択肢ブロックに到達したら、そこですべての項目へ再帰的に分岐する
    fn walk(&mut self, mut cursor: Cursor, choices: Vec<usize>, mut visited: HashSet<Cursor>) {
        loop {
            if self.routes.len() >= self.max_routes {
                self.truncated = true;
                return;
            }
            if visited.len() >= self.max_depth {
                self.routes.push(RouteRecord {
                    choices,
                    end: RouteEnd::MaxDepthExceeded {
                        max_depth: self.max_depth,
                    },
                });
                return;
            }
            if !visited.insert(cursor) {
                self.routes.push(RouteRecord {
                    choices,
                    end: RouteEnd::Circular,
                });
                return;
            }
            self.visited_scenes.insert(cursor.scene);

            let loaded = &self.scenes[cursor.scene];
            let scene = &loaded.parsed.scene;
            let blocks = segment_blocks(scene, cursor.seg);

            if cursor.block >= blocks.len() {
                if cursor.seg < scene.sections.len() {
                    cursor.seg += 1;
                    cursor.block = 0;
                    continue;
                }
                self.routes.push(RouteRecord {
                    choices,
                    end: RouteEnd::EndOfFile,
                });
                return;
            }

            match &blocks[cursor.block] {
                Block::Narration { .. } | Block::Dialogue { .. } => {
                    cursor.block += 1;
                }
                Block::Ending { id, .. } => {
                    self.routes.push(RouteRecord {
                        choices,
                        end: RouteEnd::Ending { id: id.clone() },
                    });
                    return;
                }
                Block::Jump { target, .. } => {
                    goto(&mut cursor, self.scenes, target);
                }
                Block::Choices { items, .. } => {
                    for (i, item) in items.iter().enumerate() {
                        if self.routes.len() >= self.max_routes {
                            self.truncated = true;
                            return;
                        }
                        let mut branch_cursor = cursor;
                        goto(&mut branch_cursor, self.scenes, &item.target);
                        let mut branch_choices = choices.clone();
                        branch_choices.push(i + 1);
                        self.walk(branch_cursor, branch_choices, visited.clone());
                    }
                    return;
                }
            }
        }
    }
}

fn explore(scenes: &[LoadedScene], entry: &Path, options: &RoutesOptions) -> RoutesReport {
    let mut explorer = Explorer {
        scenes,
        max_routes: options.max_routes,
        max_depth: options.max_depth,
        routes: Vec::new(),
        visited_scenes: HashSet::new(),
        truncated: false,
    };
    let start = Cursor {
        scene: 0,
        seg: 0,
        block: 0,
    };
    explorer.walk(start, Vec::new(), HashSet::new());

    let mut reached_endings: Vec<String> = explorer
        .routes
        .iter()
        .filter_map(|r| match &r.end {
            RouteEnd::Ending { id } => Some(id.clone()),
            _ => None,
        })
        .collect();
    reached_endings.sort();
    reached_endings.dedup();

    let mut declared_endings: Vec<String> = scenes
        .iter()
        .flat_map(|s| all_blocks(&s.parsed.scene))
        .filter_map(|b| match b {
            Block::Ending { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect();
    declared_endings.sort();
    declared_endings.dedup();
    let unreached_endings: Vec<String> = declared_endings
        .into_iter()
        .filter(|id| !reached_endings.contains(id))
        .collect();

    let mut unreachable_scenes: Vec<PathBuf> = scenes
        .iter()
        .enumerate()
        .filter(|(i, _)| !explorer.visited_scenes.contains(i))
        .map(|(_, s)| s.path.clone())
        .collect();
    unreachable_scenes.sort();

    let diagnostics = build_diagnostics(
        entry,
        &explorer.routes,
        &unreached_endings,
        &unreachable_scenes,
        explorer.truncated,
        options,
    );

    RoutesReport {
        routes: explorer.routes,
        reached_endings,
        unreached_endings,
        unreachable_scenes,
        truncated: explorer.truncated,
        diagnostics,
    }
}

fn all_blocks(scene: &super::Scene) -> impl Iterator<Item = &Block> {
    scene
        .lead
        .iter()
        .chain(scene.sections.iter().flat_map(|s| s.blocks.iter()))
}

fn build_diagnostics(
    entry: &Path,
    routes: &[RouteRecord],
    unreached_endings: &[String],
    unreachable_scenes: &[PathBuf],
    truncated: bool,
    options: &RoutesOptions,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for route in routes {
        let (route_desc, trace_cmd) = describe_route(entry, &route.choices);
        match &route.end {
            RouteEnd::Circular => {
                diagnostics.push(file_level(
                    "circular-route",
                    Severity::Error,
                    entry,
                    format!(
                        "{route_desc}は同じ地点に戻り続けるため、これ以上進んでも同じ結果を繰り返します（無限ループの可能性があります）。`{trace_cmd}` で該当箇所を確認してください"
                    ),
                ));
            }
            RouteEnd::MaxDepthExceeded { max_depth } => {
                diagnostics.push(file_level(
                    "route-max-depth-exceeded",
                    Severity::Warning,
                    entry,
                    format!(
                        "{route_desc}はステップ数の上限（{max_depth}）に達したため打ち切りました。非常に長い経路か、循環検出をすり抜ける長いループの可能性があります"
                    ),
                ));
            }
            RouteEnd::EndOfFile => {
                diagnostics.push(file_level(
                    "route-without-ending",
                    Severity::Warning,
                    entry,
                    format!(
                        "{route_desc}はエンディング（`<!-- ending: id -->`）を一切宣言しないままファイル末尾に到達しました。書き忘れでなければ問題ありませんが、`{trace_cmd}` で該当箇所を確認してください"
                    ),
                ));
            }
            RouteEnd::Ending { .. } => {}
        }
    }
    for id in unreached_endings {
        diagnostics.push(file_level(
            "unreachable-ending",
            Severity::Warning,
            entry,
            format!(
                "エンディング「{id}」はプロジェクト内で宣言されていますが、どの経路からも到達できません。リンクの張り忘れがないか確認してください"
            ),
        ));
    }
    for scene in unreachable_scenes {
        diagnostics.push(file_level(
            "unreachable-scene",
            Severity::Warning,
            scene,
            format!(
                "{} はプロジェクトに読み込まれていますが、{} からのどの経路からも実行されません。リンクを含むセクション自体が到達不能になっていないか確認してください",
                scene.display(),
                entry.display()
            ),
        ));
    }
    if truncated {
        diagnostics.push(file_level(
            "route-limit-exceeded",
            Severity::Warning,
            entry,
            format!(
                "経路数の上限（{}）に達したため、残りの分岐の探索を打ち切りました。結果は不完全な可能性があります",
                options.max_routes
            ),
        ));
    }
    diagnostics
}

/// 経路の説明文（`route_desc`）と、その経路を再現する trace コマンド
/// （`trace_cmd`）を組み立てる。選択肢を 1 つも経由しない経路
/// （純粋なジャンプの循環など）では `--choices` を省いた自然な文にする
fn describe_route(entry: &Path, choices: &[usize]) -> (String, String) {
    if choices.is_empty() {
        (
            "経路（選択肢を経由しない）".to_string(),
            format!("tsumugai trace {}", entry.display()),
        )
    } else {
        let list = format_choices(choices);
        (
            format!("経路 --choices {list} "),
            format!("tsumugai trace {} --choices {list}", entry.display()),
        )
    }
}
