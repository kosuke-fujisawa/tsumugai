//! SPEC 5章の実行モデルに基づく、副作用のないナビゲーション
//!
//! [`trace`](super::trace) と [`routes`](super::routes) はどちらも
//! 「シーンの中を SPEC 5章の規則で進む」処理を必要とする。両者の違いは
//! 進んだ結果を**どう記録するか**（trace は 1 経路の逐次ログ、routes は
//! 選択肢ごとに枝分かれした探索木）だけなので、進み方そのものはここに
//! 共通化する。

use super::project::{LoadedScene, resolve_sibling};
use super::{Block, LinkTarget, Scene};

/// 実行位置。`seg` 0 = リード部、`seg` n = `sections[n-1]`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Cursor {
    pub(super) scene: usize,
    pub(super) seg: usize,
    pub(super) block: usize,
}

/// [`goto`] が実際に何をまたいだか（呼び出し側が記録・集計するための情報）
pub(super) struct GotoResult {
    /// 新しいシーンファイルに進入した場合、その scenes 内インデックス
    pub(super) entered_scene: Option<usize>,
    /// アンカー付きリンクでセクションに直接着地した場合、そのセクション
    /// インデックス（`scenes[cursor.scene].parsed.scene.sections` 内）
    pub(super) entered_section: Option<usize>,
}

/// リンク先へ実行位置を移す。実行前検査（broken-link）を通っている前提の
/// 呼び出しのみを想定しており、解決は失敗しない
pub(super) fn goto(cursor: &mut Cursor, scenes: &[LoadedScene], target: &LinkTarget) -> GotoResult {
    let scene_idx = match &target.file {
        None => cursor.scene,
        Some(file) => {
            let canon = resolve_sibling(&scenes[cursor.scene].path, file)
                .and_then(|p| p.canonicalize().ok())
                .expect("check 済みのリンク先ファイルは解決できる");
            scenes
                .iter()
                .position(|s| s.canon == canon)
                .expect("check 済みのリンク先ファイルは読み込み済み")
        }
    };
    let entered_new_scene = scene_idx != cursor.scene || target.file.is_some();
    cursor.scene = scene_idx;
    cursor.block = 0;
    let entered_section = match &target.anchor {
        None => {
            cursor.seg = 0;
            None
        }
        Some(anchor) => {
            let section_idx = scenes[scene_idx]
                .parsed
                .scene
                .sections
                .iter()
                .position(|s| s.anchor == *anchor)
                .expect("check 済みのアンカーは解決できる");
            cursor.seg = section_idx + 1;
            Some(section_idx)
        }
    };
    GotoResult {
        entered_scene: entered_new_scene.then_some(scene_idx),
        entered_section,
    }
}

/// カーソルが指すセグメント（リード部 or セクション）のブロック列
pub(super) fn segment_blocks(scene: &Scene, seg: usize) -> &[Block] {
    if seg == 0 {
        &scene.lead
    } else {
        &scene.sections[seg - 1].blocks
    }
}

/// 飛び先をソースの表記（`#anchor` / `file.md` / `file.md#anchor`）に戻す
pub(super) fn target_string(target: &LinkTarget) -> String {
    match (&target.file, &target.anchor) {
        (Some(file), Some(anchor)) => format!("{file}#{anchor}"),
        (Some(file), None) => file.clone(),
        (None, Some(anchor)) => format!("#{anchor}"),
        (None, None) => String::new(),
    }
}
