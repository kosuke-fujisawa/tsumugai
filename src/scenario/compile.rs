//! `compile --target web`（#128）: StoryBundle JSON 生成
//!
//! arikoi 側の Svelte 製 player が読み込む JSON を、既存の実行前検査
//! （[`check_path`](super::check::check_path)）に通ったプロジェクトから生成する。
//! tsumugai は npm ライブラリとして配布せず、CLI サブプロセス + JSON ファイルで
//! Web フロントエンドと疎結合する（表示・アセット読み込みは compile 先の責務）。
//!
//! # 設計メモ
//! - `story_build_id` はビルド時刻や乱数ではなく、bundle の内容から決定的に
//!   計算する（FNV-1a）。同じ入力からは常に同じ ID になり、Golden JSON 比較
//!   や arikoi 側のキャッシュ判定に使える
//! - 1 ファイル = 1 [`super::Scene`] を 1 [`BundleScene`] に対応させ、
//!   リード部とセクションのブロックをファイル内の出現順にそのまま `steps` へ
//!   平坦化する。セクション終端に ending・ジャンプ・選択肢がなければ次の
//!   セクションへ続けて実行される（SPEC 5章のフォールスルーと同じ規則）ため、
//!   この平坦化だけで既存の実行モデルを再現できる
//! - jump / choice の飛び先はソース表記（`#anchor` 等）のまま持たず、
//!   `{ scene_id, step_index }` に解決済みの形で持たせる。これは
//!   `check_path` を通過済み（broken-link なし）という前提で解決できる
//! - 現行の v1 記法には変数（`set_variable`）に相当する構文がないため、
//!   このステップ種別は実装しない（narration / dialogue / choice / jump /
//!   ending の 5 種のみ）。構文が追加された時点で対応する

use super::check::{CheckOptions, CheckResult, check_path};
use super::diagnostic::Severity;
use super::project::{LoadedScene, file_level, load_project, resolve_sibling};
use super::routes::{RoutesOptions, routes_path};
use super::{Block, LinkTarget, Scene};
use serde::Serialize;
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: &str = "1";

/// compile の動作オプション
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// background / bgm の実在チェック（`--no-assets` で false）
    pub check_assets: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self { check_assets: true }
    }
}

/// compile の結果。実行前検査（check と同じ規則）の結果を必ず含む
#[derive(Debug)]
pub struct CompileResult {
    /// 開始シーンとして指定されたパス
    pub file: PathBuf,
    /// 実行前検査の結果
    pub check: CheckResult,
    /// 生成した StoryBundle。check が error のときは None
    pub bundle: Option<StoryBundle>,
}

impl CompileResult {
    /// exit code を 1 にすべきか（check エラー）
    pub fn has_errors(&self) -> bool {
        self.check.has_errors()
    }
}

/// arikoi 側の Svelte 製 player が読み込む StoryBundle
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryBundle {
    pub schema_version: String,
    /// bundle の内容から決定的に計算した ID（ビルド時刻・乱数は使わない）
    pub story_build_id: String,
    pub title: String,
    pub entry_scene_id: String,
    pub scenes: Vec<BundleScene>,
    pub assets: Vec<BundleAsset>,
}

/// シナリオ Markdown 上の位置（arikoi 側のデバッグ表示用）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
}

/// 1 ファイル = 1 シーン（[`super::Scene`] に対応）
#[derive(Debug, Clone, Serialize)]
pub struct BundleScene {
    pub id: String,
    pub title: Option<String>,
    pub source: SourceLocation,
    pub steps: Vec<BundleStep>,
}

/// 飛び先。ソース表記ではなく、解決済みの scene 内インデックスで持つ
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepTarget {
    pub scene_id: String,
    pub step_index: usize,
}

/// 選択肢 1 項目
#[derive(Debug, Clone, Serialize)]
pub struct ChoiceOption {
    pub label: String,
    pub target: StepTarget,
    pub source: SourceLocation,
}

/// 1 ステップ（SPEC 4章のブロックに対応。set_variable は現行記法に無いため未実装）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BundleStep {
    Narration {
        text: String,
        source: SourceLocation,
    },
    Dialogue {
        speaker: String,
        text: String,
        source: SourceLocation,
    },
    Choice {
        items: Vec<ChoiceOption>,
        source: SourceLocation,
    },
    Jump {
        target: StepTarget,
        source: SourceLocation,
    },
    Ending {
        id: String,
        source: SourceLocation,
    },
}

/// アセット参照（現行記法にある background / bgm のみ）
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BundleAsset {
    Background { path: String },
    Bgm { path: String },
}

/// シーンファイルを実行前検査してから StoryBundle を生成する（#128）。
///
/// check 相当の検査に加えて、`routes` 相当の全分岐探索も実行前検証に含める
/// （#144）。circular-route のような error は StoryBundle を生成させない。
/// unreachable-ending / unreachable-scene のような warning は StoryBundle
/// を生成しつつ診断として報告する（実行系に渡す前に気づけるようにする）。
///
/// パスが存在しない・ディレクトリ・検査 error の場合も panic や Err にせず、
/// Diagnostic 入りの [`CompileResult`] を返す（bundle は None になる）。
pub fn compile_path(path: &Path, options: &CompileOptions) -> CompileResult {
    if path.is_dir() {
        let diag = file_level(
            "io-error",
            Severity::Error,
            path,
            format!(
                "{} はディレクトリです。compile は開始するシーンファイル（.md）を 1 つ指定してください",
                path.display()
            ),
        );
        return CompileResult {
            file: path.to_path_buf(),
            check: CheckResult {
                files: Vec::new(),
                diagnostics: vec![diag],
            },
            bundle: None,
        };
    }

    let check_options = CheckOptions {
        check_assets: options.check_assets,
        ..CheckOptions::default()
    };
    let mut check = check_path(path, &check_options);
    if check.has_errors() {
        return CompileResult {
            file: path.to_path_buf(),
            check,
            bundle: None,
        };
    }

    let mut load_diagnostics = Vec::new();
    let scenes = load_project(vec![path.to_path_buf()], &mut load_diagnostics);
    check.diagnostics.extend(load_diagnostics);
    if check.has_errors() || scenes.is_empty() {
        return CompileResult {
            file: path.to_path_buf(),
            check,
            bundle: None,
        };
    }

    // check だけでは分からない循環・到達不能を routes の全分岐探索で検出する
    let routes_options = RoutesOptions {
        check_assets: options.check_assets,
        ..RoutesOptions::default()
    };
    if let Some(report) = routes_path(path, &routes_options).report {
        check.diagnostics.extend(report.diagnostics);
    }
    if check.has_errors() {
        return CompileResult {
            file: path.to_path_buf(),
            check,
            bundle: None,
        };
    }

    let bundle = build_bundle(&scenes, path);
    CompileResult {
        file: path.to_path_buf(),
        check,
        bundle: Some(bundle),
    }
}

// -------------------------------------------------------------- bundle構築

fn build_bundle(scenes: &[LoadedScene], entry: &Path) -> StoryBundle {
    let scene_ids: Vec<String> = scenes
        .iter()
        .map(|s| s.parsed.scene.id.clone().expect("check済みなのでidがある"))
        .collect();
    let layouts: Vec<Vec<usize>> = scenes
        .iter()
        .map(|s| segment_offsets(&s.parsed.scene))
        .collect();

    let entry_canon = entry.canonicalize().expect("check済みなので実在する");
    let entry_idx = scenes
        .iter()
        .position(|s| s.canon == entry_canon)
        .expect("entry はロード済み");

    let bundle_scenes: Vec<BundleScene> = scenes
        .iter()
        .enumerate()
        .map(|(i, loaded)| build_scene(i, loaded, scenes, &scene_ids, &layouts))
        .collect();

    let mut assets: Vec<BundleAsset> = scenes
        .iter()
        .flat_map(|s| {
            let md = &s.parsed.scene;
            let mut a = Vec::new();
            if let Some(bg) = &md.background {
                a.push(BundleAsset::Background { path: bg.clone() });
            }
            if let Some(bgm) = &md.bgm {
                a.push(BundleAsset::Bgm { path: bgm.clone() });
            }
            a
        })
        .collect();
    assets.sort();
    assets.dedup();

    let title = scenes[entry_idx]
        .parsed
        .scene
        .title
        .clone()
        .unwrap_or_else(|| scene_ids[entry_idx].clone());

    let mut bundle = StoryBundle {
        schema_version: SCHEMA_VERSION.to_string(),
        story_build_id: String::new(),
        title,
        entry_scene_id: scene_ids[entry_idx].clone(),
        scenes: bundle_scenes,
        assets,
    };
    bundle.story_build_id = compute_build_id(&bundle);
    bundle
}

/// セグメント（`seg` 0 = リード部、`seg` n = `sections[n-1]`）の開始 step
/// インデックス。長さは `sections.len() + 1`
fn segment_offsets(scene: &Scene) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(scene.sections.len() + 1);
    let mut cursor = 0usize;
    offsets.push(cursor);
    cursor += scene.lead.len();
    for section in &scene.sections {
        offsets.push(cursor);
        cursor += section.blocks.len();
    }
    offsets
}

fn build_scene(
    idx: usize,
    loaded: &LoadedScene,
    scenes: &[LoadedScene],
    scene_ids: &[String],
    layouts: &[Vec<usize>],
) -> BundleScene {
    let md = &loaded.parsed.scene;
    let file = loaded.path.display().to_string();
    let blocks = md
        .lead
        .iter()
        .chain(md.sections.iter().flat_map(|s| s.blocks.iter()));
    let steps = blocks
        .map(|block| build_step(block, &file, scenes, scene_ids, layouts, idx))
        .collect();

    BundleScene {
        id: scene_ids[idx].clone(),
        title: md.title.clone(),
        source: SourceLocation {
            file: file.clone(),
            line: 1,
        },
        steps,
    }
}

fn build_step(
    block: &Block,
    file: &str,
    scenes: &[LoadedScene],
    scene_ids: &[String],
    layouts: &[Vec<usize>],
    current: usize,
) -> BundleStep {
    let src = |line: usize| SourceLocation {
        file: file.to_string(),
        line,
    };
    match block {
        Block::Narration { text, line } => BundleStep::Narration {
            text: text.clone(),
            source: src(*line),
        },
        Block::Dialogue {
            speaker,
            text,
            line,
        } => BundleStep::Dialogue {
            speaker: speaker.clone(),
            text: text.clone(),
            source: src(*line),
        },
        Block::Ending { id, line } => BundleStep::Ending {
            id: id.clone(),
            source: src(*line),
        },
        Block::Jump { target, line, .. } => BundleStep::Jump {
            target: resolve_target(scenes, scene_ids, layouts, current, target),
            source: src(*line),
        },
        Block::Choices { items, line } => BundleStep::Choice {
            items: items
                .iter()
                .map(|item| ChoiceOption {
                    label: item.label.clone(),
                    target: resolve_target(scenes, scene_ids, layouts, current, &item.target),
                    source: src(item.line),
                })
                .collect(),
            source: src(*line),
        },
    }
}

/// リンク先を `{ scene_id, step_index }` に解決する。
/// check_path を通過済み（broken-link なし）という前提でのみ呼ばれる
fn resolve_target(
    scenes: &[LoadedScene],
    scene_ids: &[String],
    layouts: &[Vec<usize>],
    current: usize,
    target: &LinkTarget,
) -> StepTarget {
    let scene_idx = match &target.file {
        None => current,
        Some(file) => {
            let canon = resolve_sibling(&scenes[current].path, file)
                .and_then(|p| p.canonicalize().ok())
                .expect("check済みのリンク先ファイルは解決できる");
            scenes
                .iter()
                .position(|s| s.canon == canon)
                .expect("check済みのリンク先ファイルは読み込み済み")
        }
    };
    let step_index = match &target.anchor {
        None => 0,
        Some(anchor) => {
            let section_idx = scenes[scene_idx]
                .parsed
                .scene
                .sections
                .iter()
                .position(|s| s.anchor == *anchor)
                .expect("check済みのアンカーは解決できる");
            layouts[scene_idx][section_idx + 1]
        }
    };
    StepTarget {
        scene_id: scene_ids[scene_idx].clone(),
        step_index,
    }
}

// -------------------------------------------------------------- story_build_id

/// bundle の内容（`story_build_id` を除く）から決定的に ID を計算する。
/// ビルド時刻や乱数を使わないため、同じ入力からは常に同じ ID になり、
/// Golden JSON 比較や arikoi 側のキャッシュ判定に使える
fn compute_build_id(bundle: &StoryBundle) -> String {
    let payload = serde_json::json!({
        "schemaVersion": bundle.schema_version,
        "title": bundle.title,
        "entrySceneId": bundle.entry_scene_id,
        "scenes": bundle.scenes,
        "assets": bundle.assets,
    });
    let bytes = serde_json::to_vec(&payload).expect("シリアライズに失敗しない");
    format!("{:016x}", fnv1a64(&bytes))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
