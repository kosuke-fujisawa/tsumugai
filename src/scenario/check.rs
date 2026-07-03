//! 意味論検査（SPEC 6章のうち、参照解決・ファイル横断・実行フローのルール）
//!
//! parser（構文系ルール）が返す Diagnostic に加えて、プロジェクト全体を
//! 見ないと判定できないルールを検査する:
//!
//! - `broken-link`: リンク先のファイル・アンカー（H2）の実在解決
//! - `duplicate-scene-id`: シーン ID のファイル横断の一意性
//! - `missing-asset`: front matter の background / bgm の実在
//! - `undefined-character` / `missing-characters-file` / `invalid-characters-file`
//! - `implicit-fallthrough` / `unreachable-section`: 実行フロー
//!
//! SPEC 6.1「Diagnostic は学習教材である」に従い、最初のエラーで止まらず
//! 検出できたすべての Diagnostic を返す。[`check_path`] は入出力エラーでも
//! 失敗せず、`io-error` の Diagnostic として報告する（JSON / SARIF 出力の
//! 形式を崩さないため）。

use super::characters::{Characters, find_characters_file, load_characters};
use super::diagnostic::{Diagnostic, Severity, Span};
use super::parse::{Parsed, parse_file};
use super::{Block, LinkTarget, Scene, slugify};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// check の動作オプション
#[derive(Debug, Clone)]
pub struct CheckOptions {
    /// background / bgm の実在チェック（`--no-assets` で false）
    pub check_assets: bool,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self { check_assets: true }
    }
}

/// check の結果。Diagnostic はファイル順 → 行順に並ぶ
#[derive(Debug)]
pub struct CheckResult {
    /// 検査対象になったファイル(走査順)
    pub files: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

/// 読み込んだ 1 シーン
struct LoadedScene {
    /// 表示用パス（入力引数からの相対のまま Diagnostic に載せる）
    path: PathBuf,
    /// 同一ファイル判定用の正規化パス
    canon: PathBuf,
    parsed: Parsed,
}

/// Markdown ファイルまたはディレクトリを検査する（SPEC 6章）。
///
/// - ディレクトリ: 配下のすべての `.md` を 1 つのプロジェクトとして検査する
/// - ファイル: そのファイルとリンクで辿れる範囲を検査する
///
/// パスが存在しない・読めない場合も panic や Err にせず、`io-error` の
/// Diagnostic を持つ [`CheckResult`] を返す。
pub fn check_path(path: &Path, options: &CheckOptions) -> CheckResult {
    let mut diagnostics = Vec::new();
    let seeds: Vec<PathBuf> = if path.is_dir() {
        let mut files = Vec::new();
        collect_md_files(path, &mut files);
        if files.is_empty() {
            diagnostics.push(file_level(
                "io-error",
                Severity::Error,
                path,
                format!(
                    "{} に .md ファイルがありません。シナリオファイルのあるディレクトリか、ファイルそのものを指定してください",
                    path.display()
                ),
            ));
        }
        files
    } else if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        diagnostics.push(file_level(
            "io-error",
            Severity::Error,
            path,
            format!(
                "{} が見つかりません。パスを確認してください",
                path.display()
            ),
        ));
        Vec::new()
    };

    let scenes = load_project(seeds, &mut diagnostics);
    for scene in &scenes {
        diagnostics.extend(scene.parsed.diagnostics.iter().cloned());
    }
    check_duplicate_scene_ids(&scenes, &mut diagnostics);
    check_links(&scenes, &mut diagnostics);
    if options.check_assets {
        check_assets(&scenes, &mut diagnostics);
    }
    check_characters(&scenes, &mut diagnostics);
    for scene in &scenes {
        check_fallthrough(scene, &mut diagnostics);
    }
    check_unreachable(&scenes, &mut diagnostics);

    // ファイル順 → 行順に並べ、入力との対応を追いやすくする
    let order: HashMap<&Path, usize> = scenes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.path.as_path(), i))
        .collect();
    diagnostics.sort_by_key(|d| {
        (
            order.get(d.file.as_path()).copied().unwrap_or(usize::MAX),
            d.span.as_ref().map_or(0, |s| s.line),
        )
    });

    CheckResult {
        files: scenes.iter().map(|s| s.path.clone()).collect(),
        diagnostics,
    }
}

// ---------------------------------------------------------------- 読み込み

/// ディレクトリ配下の `.md` を再帰的に集める（名前順）。
/// 隠しディレクトリと、シーンではない README.md は除く（SPEC 6章）
fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
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
fn load_project(seeds: Vec<PathBuf>, diagnostics: &mut Vec<Diagnostic>) -> Vec<LoadedScene> {
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
fn resolve_sibling(scene_path: &Path, relative: &str) -> Option<PathBuf> {
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
fn scene_links(scene: &Scene) -> Vec<(&str, &LinkTarget, usize)> {
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

// ------------------------------------------------------- duplicate-scene-id

fn check_duplicate_scene_ids(scenes: &[LoadedScene], diagnostics: &mut Vec<Diagnostic>) {
    let mut first_by_id: BTreeMap<&str, &LoadedScene> = BTreeMap::new();
    for scene in scenes {
        let Some(id) = &scene.parsed.scene.id else {
            continue; // missing-scene-id は parser が報告済み
        };
        match first_by_id.get(id.as_str()) {
            None => {
                first_by_id.insert(id, scene);
            }
            Some(first) => {
                let line = scene.parsed.front_matter_spans.id.unwrap_or(1);
                let first_line = first.parsed.front_matter_spans.id.unwrap_or(1);
                diagnostics.push(Diagnostic::error(
                    "duplicate-scene-id",
                    &scene.path,
                    line,
                    format!(
                        "シーン ID「{id}」は {}（{first_line} 行目）でも使われています。シーン ID はプロジェクト内で一意にしてください",
                        first.path.display()
                    ),
                ));
            }
        }
    }
}

// -------------------------------------------------------------- broken-link

fn check_links(scenes: &[LoadedScene], diagnostics: &mut Vec<Diagnostic>) {
    let by_canon: HashMap<&Path, &LoadedScene> =
        scenes.iter().map(|s| (s.canon.as_path(), s)).collect();
    for scene in scenes {
        for (label, target, line) in scene_links(&scene.parsed.scene) {
            check_one_link(scene, label, target, line, &by_canon, diagnostics);
        }
    }
}

fn check_one_link(
    scene: &LoadedScene,
    label: &str,
    target: &LinkTarget,
    line: usize,
    by_canon: &HashMap<&Path, &LoadedScene>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // 1) ファイルの解決
    let target_scene: &LoadedScene = match &target.file {
        None => scene,
        Some(file) => {
            let Some(resolved) = resolve_sibling(&scene.path, file) else {
                diagnostics.push(Diagnostic::error(
                    "broken-link",
                    &scene.path,
                    line,
                    format!(
                        "リンク先「{file}」は絶対パスです。飛び先にできるのは、このファイルからの相対パスで参照できるプロジェクト内の .md だけです（SPEC 2章）"
                    ),
                ));
                return;
            };
            if !resolved.is_file() {
                let mut diag = Diagnostic::error(
                    "broken-link",
                    &scene.path,
                    line,
                    format!(
                        "リンク先のファイル「{file}」が見つかりません（{} からの相対パス {} を探しました）",
                        scene.path.display(),
                        resolved.display()
                    ),
                );
                if let Some(similar) = closest_md_in(&resolved) {
                    diag.message
                        .push_str(&format!("。よく似た「{similar}」があります"));
                    diag.suggestion = Some(format!(
                        "[{label}]({})",
                        with_anchor(&similar, target.anchor.as_deref())
                    ));
                }
                diagnostics.push(diag);
                return;
            }
            if resolved.extension().and_then(|e| e.to_str()) != Some("md") {
                diagnostics.push(Diagnostic::error(
                    "broken-link",
                    &scene.path,
                    line,
                    format!(
                        "「{file}」は Markdown ファイルではありません。飛び先にできるのはプロジェクト内の .md ファイルとその見出し（##）だけです"
                    ),
                ));
                return;
            }
            match resolved
                .canonicalize()
                .ok()
                .and_then(|c| by_canon.get(c.as_path()))
            {
                Some(s) => s,
                None => return, // 読み込みに失敗したファイル（io-error 報告済み）
            }
        }
    };

    // 2) アンカーの解決
    let Some(anchor) = &target.anchor else {
        return; // ファイル先頭（リード部）への着地
    };
    let target_md = &target_scene.parsed.scene;
    if target_md.sections.iter().any(|s| s.anchor == *anchor) {
        return;
    }

    let place = match &target.file {
        Some(_) => target_scene.path.display().to_string(),
        None => "このファイル".to_string(),
    };
    let mut diag = Diagnostic::error(
        "broken-link",
        &scene.path,
        line,
        format!("{place}に「{anchor}」という見出し（##）はありません。"),
    );
    let anchors: Vec<&str> = target_md
        .sections
        .iter()
        .map(|s| s.anchor.as_str())
        .filter(|a| !a.is_empty())
        .collect();
    let h1_slug = target_md.title.as_deref().map(slugify);
    if h1_slug.as_deref() == Some(anchor.as_str()) {
        // H1 へのリンク（SPEC 3.2）
        diag.message.push_str(
            "H1 タイトルはアンカーにならず、分岐先にできるのは H2 セクションだけです。ファイルの先頭から始めたい場合は、アンカーなしでファイル名だけをリンクしてください",
        );
        if let Some(file) = &target.file {
            diag.suggestion = Some(format!("[{label}]({file})"));
        }
    } else if let Some(similar) = closest(anchor, &anchors) {
        let section = target_md
            .sections
            .iter()
            .find(|s| s.anchor == similar)
            .expect("closest はセクション由来");
        let fixed = with_anchor_target(target.file.as_deref(), similar);
        diag.message.push_str(&format!(
            "よく似た「## {}」があります。`[{label}]({fixed})` の間違いではありませんか？",
            section.heading
        ));
        diag.suggestion = Some(format!("[{label}]({fixed})"));
        if target.file.is_none() {
            diag.related_spans.push(Span { line: section.line });
        }
    } else if anchors.is_empty() {
        diag.message.push_str(
            "分岐先にできる H2 見出しがまだありません。飛び先にしたい場所に `## 見出し` を書いてください",
        );
    } else {
        let list: Vec<String> = anchors.iter().take(5).map(|a| format!("#{a}")).collect();
        diag.message
            .push_str(&format!("使える見出し: {}", list.join("、")));
    }
    diagnostics.push(diag);
}

/// `file.md` + アンカー有無 → リンク先文字列
fn with_anchor(file: &str, anchor: Option<&str>) -> String {
    match anchor {
        Some(a) => format!("{file}#{a}"),
        None => file.to_string(),
    }
}

/// ファイル有無 + アンカー → リンク先文字列
fn with_anchor_target(file: Option<&str>, anchor: &str) -> String {
    match file {
        Some(f) => format!("{f}#{anchor}"),
        None => format!("#{anchor}"),
    }
}

/// 見つからなかったファイルと同じディレクトリにある、よく似た名前の `.md`
fn closest_md_in(missing: &Path) -> Option<String> {
    let dir = missing.parent()?;
    let name = missing.file_name()?.to_str()?;
    let entries = std::fs::read_dir(dir).ok()?;
    let mut candidates: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .collect();
    candidates.sort();
    let refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
    closest(name, &refs).map(String::from)
}

// ------------------------------------------------------------ missing-asset

fn check_assets(scenes: &[LoadedScene], diagnostics: &mut Vec<Diagnostic>) {
    for scene in scenes {
        let md = &scene.parsed.scene;
        let spans = &scene.parsed.front_matter_spans;
        let entries = [
            ("background", &md.background, spans.background),
            ("bgm", &md.bgm, spans.bgm),
        ];
        for (key, value, span) in entries {
            let Some(value) = value else { continue };
            let Some(resolved) = resolve_sibling(&scene.path, value) else {
                diagnostics.push(Diagnostic::error(
                    "missing-asset",
                    &scene.path,
                    span.unwrap_or(1),
                    format!(
                        "{key} の「{value}」は絶対パスです。{} からの相対パスで書いてください（SPEC 2章）",
                        scene.path.display()
                    ),
                ));
                continue;
            };
            if resolved.is_file() {
                continue;
            }
            let mut diag = Diagnostic::error(
                "missing-asset",
                &scene.path,
                span.unwrap_or(1),
                format!(
                    "{key} のファイル「{value}」が見つかりません（{} からの相対パス {} を探しました）",
                    scene.path.display(),
                    resolved.display()
                ),
            );
            if let Some(similar) = closest_file_in(&resolved) {
                let dir = Path::new(value).parent().unwrap_or_else(|| Path::new(""));
                let fixed = dir.join(&similar).to_string_lossy().replace('\\', "/");
                diag.message
                    .push_str(&format!("。よく似た「{fixed}」があります"));
                diag.suggestion = Some(format!("{key}: {fixed}"));
            }
            diagnostics.push(diag);
        }
    }
}

/// 見つからなかったアセットと同じディレクトリにある、よく似た名前のファイル
fn closest_file_in(missing: &Path) -> Option<String> {
    let dir = missing.parent()?;
    let name = missing.file_name()?.to_str()?;
    let entries = std::fs::read_dir(dir).ok()?;
    let mut candidates: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .filter(|n| !n.starts_with('.'))
        .collect();
    candidates.sort();
    let refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
    closest(name, &refs).map(String::from)
}

// ------------------------------------------- characters.yaml と話者の検査

fn check_characters(scenes: &[LoadedScene], diagnostics: &mut Vec<Diagnostic>) {
    // 同じ characters.yaml を何度も読まない。None は読み込み失敗（報告済み）
    let mut cache: HashMap<PathBuf, Option<Characters>> = HashMap::new();
    let mut missing_reported = false;
    for scene in scenes {
        let Some(chars_path) = find_characters_file(&scene.path) else {
            // 見つからない場合の warning はプロジェクトで 1 件だけ（SPEC 2.1）
            if !missing_reported {
                missing_reported = true;
                let dir = scene.path.parent().unwrap_or_else(|| Path::new("."));
                diagnostics.push(
                    file_level(
                        "missing-characters-file",
                        Severity::Warning,
                        &scene.path,
                        format!(
                            "characters.yaml が見つかりません（{} とその祖先ディレクトリを探しました）。話者を宣言すると、話者名の書き間違い検査（undefined-character）が使えます。見つかるまで話者名の検査は行いません",
                            dir.display()
                        ),
                    )
                    .with_suggestion("characters:\n  話者名: {}".to_string()),
                );
            }
            continue;
        };
        if !cache.contains_key(&chars_path) {
            let loaded = match load_characters(&chars_path) {
                Ok(c) => Some(c),
                Err(e) => {
                    diagnostics.push(
                        file_level(
                            "invalid-characters-file",
                            Severity::Error,
                            &chars_path,
                            format!(
                                "{e}。修正するまで話者名の検査（undefined-character）は行いません"
                            ),
                        )
                        .with_suggestion("characters:\n  話者名: {}".to_string()),
                    );
                    None
                }
            };
            cache.insert(chars_path.clone(), loaded);
        }
        let Some(chars) = cache.get(&chars_path).and_then(|c| c.as_ref()) else {
            continue;
        };
        check_speakers(scene, chars, diagnostics);
    }
}

fn check_speakers(scene: &LoadedScene, chars: &Characters, diagnostics: &mut Vec<Diagnostic>) {
    /// 未宣言話者の出現箇所（最初の 1 件に warning、以降は related_spans）
    struct Occurrence {
        first_line: usize,
        first_text: String,
        rest: Vec<usize>,
    }
    let md = &scene.parsed.scene;
    let mut undefined: BTreeMap<&str, Occurrence> = BTreeMap::new();
    let blocks = md
        .lead
        .iter()
        .chain(md.sections.iter().flat_map(|s| s.blocks.iter()));
    for block in blocks {
        let Block::Dialogue {
            speaker,
            text,
            line,
        } = block
        else {
            continue;
        };
        if chars.contains(speaker) {
            continue;
        }
        undefined
            .entry(speaker.as_str())
            .and_modify(|o| o.rest.push(*line))
            .or_insert_with(|| Occurrence {
                first_line: *line,
                first_text: text.lines().next().unwrap_or("").to_string(),
                rest: Vec::new(),
            });
    }
    let declared: Vec<&str> = chars.entries.keys().map(String::as_str).collect();
    for (speaker, occ) in undefined {
        let mut message = format!(
            "話者「{speaker}」は {} に宣言されていません。",
            chars.path.display()
        );
        let mut suggestion = None;
        if let Some(similar) = closest(speaker, &declared) {
            message.push_str(&format!(
                "宣言済みの「{similar}」の書き間違いではありませんか？新しい登場人物なら characters.yaml に追加してください"
            ));
            suggestion = Some(format!("{similar}: {}", occ.first_text));
        } else {
            let list: Vec<&str> = declared.iter().take(8).copied().collect();
            message.push_str(&format!(
                "書き間違いなら正しい名前に直し、新しい登場人物なら characters.yaml に追加してください（宣言済み: {}）",
                list.join("、")
            ));
        }
        let mut diag =
            Diagnostic::warning("undefined-character", &scene.path, occ.first_line, message);
        diag.suggestion = suggestion;
        diag.related_spans = occ.rest.into_iter().map(|line| Span { line }).collect();
        diagnostics.push(diag);
    }
}

// ---------------------------------------------------- implicit-fallthrough

fn check_fallthrough(scene: &LoadedScene, diagnostics: &mut Vec<Diagnostic>) {
    let sections = &scene.parsed.scene.sections;
    for pair in sections.windows(2) {
        let (cur, next) = (&pair[0], &pair[1]);
        if ends_with_terminator(cur.blocks.last()) {
            continue;
        }
        let line = cur.blocks.last().map(block_line).unwrap_or(cur.line);
        diagnostics.push(
            Diagnostic::warning(
                "implicit-fallthrough",
                &scene.path,
                line,
                format!(
                    "セクション「{}」の末尾が ending・ジャンプ・選択肢のいずれでもないため、実行は次のセクション「{}」（{} 行目）に流れ込みます。意図した合流でなければ、`<!-- ending: id -->` で終えるか、ジャンプで飛び先を明示してください",
                    cur.heading, next.heading, next.line
                ),
            )
            .with_related(next.line),
        );
    }
}

// ----------------------------------------------------- unreachable-section

fn check_unreachable(scenes: &[LoadedScene], diagnostics: &mut Vec<Diagnostic>) {
    // プロジェクト全体でリンクされている (正規化ファイル, アンカー) の集合
    let mut linked: HashSet<(PathBuf, String)> = HashSet::new();
    for scene in scenes {
        for (_, target, _) in scene_links(&scene.parsed.scene) {
            let Some(anchor) = &target.anchor else {
                continue;
            };
            let canon = match &target.file {
                None => scene.canon.clone(),
                Some(file) => {
                    let resolved =
                        resolve_sibling(&scene.path, file).and_then(|r| r.canonicalize().ok());
                    match resolved {
                        Some(c) => c,
                        None => continue, // broken-link 報告済み
                    }
                }
            };
            linked.insert((canon, anchor.clone()));
        }
    }
    for scene in scenes {
        let md = &scene.parsed.scene;
        // リード部は必ず実行される。末尾が ending・ジャンプ・選択肢でなければ
        // 最初のセクションへフォールスルーする（空のリードも同様）
        let mut falls = !ends_with_terminator(md.lead.last());
        for section in &md.sections {
            let is_linked = linked.contains(&(scene.canon.clone(), section.anchor.clone()));
            let reachable = falls || is_linked;
            // アンカーが空のセクションは empty-anchor（error）が報告済みなので重ねない
            if !reachable && !section.anchor.is_empty() {
                diagnostics.push(Diagnostic::warning(
                    "unreachable-section",
                    &scene.path,
                    section.line,
                    format!(
                        "セクション「{}」はどこからも参照されず、前のセクションからのフォールスルーでも到達しません。選択肢やジャンプから `#{}` で参照するか、不要なら削除してください",
                        section.heading, section.anchor
                    ),
                ));
            }
            falls = reachable && !ends_with_terminator(section.blocks.last());
        }
    }
}

/// 実行がここで必ず終わる・飛ぶブロックか（SPEC 5章のフォールスルー判定）
fn ends_with_terminator(last: Option<&Block>) -> bool {
    matches!(
        last,
        Some(Block::Ending { .. } | Block::Jump { .. } | Block::Choices { .. })
    )
}

fn block_line(block: &Block) -> usize {
    match block {
        Block::Narration { line, .. }
        | Block::Dialogue { line, .. }
        | Block::Choices { line, .. }
        | Block::Jump { line, .. }
        | Block::Ending { line, .. } => *line,
    }
}

/// span を持たないファイルレベルの Diagnostic
fn file_level(
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

// ------------------------------------------------------------- 類似候補探索

/// 書き間違いの可能性が高い、よく似た候補を返す（SPEC 6.1 の suggestion 用）。
///
/// 編集距離 2 以内、または先頭の文字が同じで編集距離 3 以内を「よく似ている」
/// とみなす（「run-togather」→「run-together」、「幼馴染」→「幼なじみ」など）。
/// 候補との距離が文字数と同程度（ほぼ総入れ替え）の場合は候補にしない。
fn closest<'a>(target: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let target_len = target.chars().count();
    let target_head = target.chars().next();
    // (距離, 先頭が違う) の辞書順で最小の「よく似た」候補を選ぶ
    let mut best: Option<((usize, bool), &str)> = None;
    for candidate in candidates {
        let d = levenshtein(target, candidate);
        let max_len = target_len.max(candidate.chars().count());
        let same_head = target_head == candidate.chars().next();
        if !(d < max_len && (d <= 2 || (d <= 3 && same_head))) {
            continue;
        }
        let key = (d, !same_head);
        if best.is_none_or(|(bk, _)| key < bk) {
            best = Some((key, candidate));
        }
    }
    best.map(|(_, candidate)| candidate)
}

/// 文字単位の編集距離
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = Vec::with_capacity(b.len() + 1);
        cur.push(i + 1);
        for (j, cb) in b.iter().enumerate() {
            let subst = prev[j] + usize::from(ca != cb);
            cur.push(subst.min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 編集距離が正しい() {
        assert_eq!(levenshtein("run-togather", "run-together"), 1);
        assert_eq!(levenshtein("同じ", "同じ"), 0);
        assert_eq!(levenshtein("", "abc"), 3);
    }

    #[test]
    fn よく似た候補だけが提案される() {
        assert_eq!(
            closest("run-togather", &["run-together", "walk-together"]),
            Some("run-together")
        );
        // 先頭が同じ日本語名の書き間違い
        assert_eq!(closest("幼馴染", &["幼なじみ", "主人公"]), Some("幼なじみ"));
        // ほぼ総入れ替えの短い文字列は候補にしない
        assert_eq!(closest("朝", &["夕方"]), None);
        assert_eq!(closest("morning", &["evening"]), None);
    }
}
