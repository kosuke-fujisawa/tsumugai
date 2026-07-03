//! プロジェクト（シーンファイル集合）の読み込み
//!
//! check（意味論検査）と trace（実行トレース）が同じ読み込み規則を共有する:
//!
//! - ディレクトリ: 配下のすべての `.md`（README.md と隠しディレクトリを除く）
//! - ファイル: そのファイルとリンクで辿れる閉包
//! - ファイル参照はシーンファイルからの相対パスのみ（SPEC 2章）

use super::diagnostic::{Diagnostic, Severity};
use super::parse::{Parsed, parse_file};
use super::{Block, LinkTarget, Scene};
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// 読み込んだ 1 シーン
pub(super) struct LoadedScene {
    /// 表示用パス（入力引数からの相対のまま Diagnostic に載せる）
    pub(super) path: PathBuf,
    /// 同一ファイル判定用の正規化パス
    pub(super) canon: PathBuf,
    pub(super) parsed: Parsed,
}

/// ディレクトリ配下の `.md` を再帰的に集める（名前順）。
/// 隠しディレクトリと、シーンではない README.md は除く（SPEC 6章）
pub(super) fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.eq_ignore_ascii_case("readme.md") {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// seeds とそこからリンクで辿れる `.md` をすべてパースする
pub(super) fn load_project(
    seeds: Vec<PathBuf>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<LoadedScene> {
    let mut scenes = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = seeds.into();
    while let Some(display) = queue.pop_front() {
        // 実在は enqueue 前に確認済みだが、競合で消えた場合は黙って飛ばさず報告する
        let canon = match display.canonicalize() {
            Ok(c) => c,
            Err(e) => {
                diagnostics.push(file_level(
                    "io-error",
                    Severity::Error,
                    &display,
                    format!("{} を読み込めません: {}", display.display(), e),
                ));
                continue;
            }
        };
        if !seen.insert(canon.clone()) {
            continue;
        }
        let parsed = match parse_file(&display) {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(file_level("io-error", Severity::Error, &display, e));
                continue;
            }
        };
        // リンク先の .md も検査対象に加える（閉包）。実在しない・絶対パスの
        // ファイルは check_links が broken-link として報告する
        for (_, target, _) in scene_links(&parsed.scene) {
            if let Some(file) = &target.file
                && let Some(resolved) = resolve_sibling(&display, file)
                && resolved.is_file()
                && resolved.extension().and_then(|e| e.to_str()) == Some("md")
            {
                queue.push_back(resolved);
            }
        }
        scenes.push(LoadedScene {
            path: display,
            canon,
            parsed,
        });
    }
    scenes
}

/// シーンファイルからの相対パスを解決する。
/// tsumugai が読むのは相対パスで参照されるファイルだけ（SPEC 2章）なので、
/// 絶対パスは解決せず None を返す（呼び出し側が broken-link / missing-asset に倒す）
pub(super) fn resolve_sibling(scene_path: &Path, relative: &str) -> Option<PathBuf> {
    if Path::new(relative).is_absolute() {
        return None;
    }
    Some(
        scene_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(relative),
    )
}

/// シーン内のすべてのリンク（ジャンプ + 選択肢項目）を (ラベル, 飛び先, 行) で列挙する
pub(super) fn scene_links(scene: &Scene) -> Vec<(&str, &LinkTarget, usize)> {
    let mut out = Vec::new();
    let blocks = scene
        .lead
        .iter()
        .chain(scene.sections.iter().flat_map(|s| s.blocks.iter()));
    for block in blocks {
        match block {
            Block::Jump {
                label,
                target,
                line,
            } => out.push((label.as_str(), target, *line)),
            Block::Choices { items, .. } => {
                for item in items {
                    out.push((item.label.as_str(), &item.target, item.line));
                }
            }
            _ => {}
        }
    }
    out
}

/// span を持たないファイルレベルの Diagnostic
pub(super) fn file_level(
    rule_id: &'static str,
    severity: Severity,
    file: &Path,
    message: String,
) -> Diagnostic {
    Diagnostic {
        rule_id,
        severity,
        message,
        file: file.to_path_buf(),
        span: None,
        related_spans: Vec::new(),
        suggestion: None,
    }
}
