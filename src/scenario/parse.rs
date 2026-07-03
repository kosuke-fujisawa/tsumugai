//! v1 記法パーサー本体
//!
//! pulldown-cmark のイベント列を歩いて [`Scene`] を構築する。
//! SPEC 6.1 に従い、エラーで中断せず解釈できた範囲の Scene と
//! すべての Diagnostic を返す。

use super::anchor::{percent_decode, slugify};
use super::diagnostic::Diagnostic;
use super::{Block, ChoiceItem, LinkTarget, Scene, Section};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::ops::Range;
use std::path::Path;

/// パース結果。Scene と Diagnostic は常に両方返る
#[derive(Debug)]
pub struct Parsed {
    pub scene: Scene,
    pub diagnostics: Vec<Diagnostic>,
    /// front matter の各キーの行番号。check が missing-asset /
    /// duplicate-scene-id の span を付けるのに使う
    pub front_matter_spans: FrontMatterSpans,
}

/// front matter のキーごとの行番号（1-origin、ファイル全体での行）
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FrontMatterSpans {
    pub id: Option<usize>,
    pub background: Option<usize>,
    pub bgm: Option<usize>,
}

/// ファイルを読み込んでパースする
pub fn parse_file(path: &Path) -> Result<Parsed, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("{} を読み込めません: {}", path.display(), e))?;
    Ok(parse_str(&source, path))
}

/// 文字列をパースする。`path` は Diagnostic とリンク解決の基準に使う
pub fn parse_str(source: &str, path: &Path) -> Parsed {
    let mut p = SceneParser::new(source, path);
    p.run();
    Parsed {
        scene: p.scene,
        diagnostics: p.diagnostics,
        front_matter_spans: p.fm_spans,
    }
}

/// front matter の解析結果
struct FrontMatter {
    /// body が source の何バイト目から始まるか
    body_offset: usize,
    /// front matter ブロックの開始行（1-origin）。ない場合は None
    present: Option<usize>,
}

struct SceneParser<'a> {
    source: &'a str,
    path: &'a Path,
    /// source 全体の各行の開始バイト位置（行番号計算用）
    line_starts: Vec<usize>,
    body_offset: usize,
    scene: Scene,
    diagnostics: Vec<Diagnostic>,
    current_section: Option<Section>,
    /// H1 を見た行（invalid-h1 判定用）
    title_line: Option<usize>,
    fm_spans: FrontMatterSpans,
}

impl<'a> SceneParser<'a> {
    fn new(source: &'a str, path: &'a Path) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self {
            source,
            path,
            line_starts,
            body_offset: 0,
            scene: Scene {
                path: path.to_path_buf(),
                id: None,
                title: None,
                background: None,
                bgm: None,
                lead: Vec::new(),
                sections: Vec::new(),
            },
            diagnostics: Vec::new(),
            current_section: None,
            title_line: None,
            fm_spans: FrontMatterSpans::default(),
        }
    }

    /// source 全体でのバイト位置 → 行番号（1-origin）
    fn line_at(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i + 1,
            Err(i) => i,
        }
    }

    /// body 内の range → source 全体での行番号
    fn line_of(&self, range: &Range<usize>) -> usize {
        self.line_at(self.body_offset + range.start)
    }

    fn error(&mut self, rule_id: &'static str, line: usize, message: String) -> &mut Diagnostic {
        self.diagnostics
            .push(Diagnostic::error(rule_id, self.path, line, message));
        self.diagnostics.last_mut().expect("just pushed")
    }

    fn warning(&mut self, rule_id: &'static str, line: usize, message: String) -> &mut Diagnostic {
        self.diagnostics
            .push(Diagnostic::warning(rule_id, self.path, line, message));
        self.diagnostics.last_mut().expect("just pushed")
    }

    fn push_block(&mut self, block: Block) {
        match &mut self.current_section {
            Some(section) => section.blocks.push(block),
            None => self.scene.lead.push(block),
        }
    }

    fn run(&mut self) {
        let fm = self.parse_front_matter();
        self.body_offset = fm.body_offset;
        // self への可変借用と衝突しないよう、'a の参照を取り出してから使う
        let source: &'a str = self.source;
        let body = &source[self.body_offset..];

        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        let events: Vec<(Event, Range<usize>)> =
            Parser::new_ext(body, options).into_offset_iter().collect();
        self.walk(&events);

        // 開きっぱなしのセクションを確定する
        if let Some(section) = self.current_section.take() {
            self.scene.sections.push(section);
        }
        if self.scene.title.is_none() {
            let line = fm
                .present
                .map(|_| self.line_at(self.body_offset))
                .unwrap_or(1);
            self.warning(
                "missing-title",
                line,
                "H1 タイトルがありません。ファイル先頭に `# タイトル` を書いてください".to_string(),
            );
        }
    }

    // ---------------------------------------------------------- front matter

    fn parse_front_matter(&mut self) -> FrontMatter {
        let mut lines = self.source.lines();
        if lines.next().map(|l| l.trim_end()) != Some("---") {
            self.error(
                "missing-scene-id",
                1,
                "front matter がありません。ファイル先頭にシーン ID を書いてください".to_string(),
            )
            .suggestion = Some("---\nid: scene_id\n---".to_string());
            return FrontMatter {
                body_offset: 0,
                present: None,
            };
        }
        // 閉じの --- を探す（2 行目以降）
        let mut yaml_end: Option<usize> = None; // 閉じ行の行番号（1-origin）
        for (i, line) in self.source.lines().enumerate().skip(1) {
            if line.trim_end() == "---" {
                yaml_end = Some(i + 1);
                break;
            }
        }
        let Some(close_line) = yaml_end else {
            self.error(
                "invalid-frontmatter",
                1,
                "front matter が閉じられていません。`---` の行で閉じてください".to_string(),
            );
            return FrontMatter {
                body_offset: 0,
                present: None,
            };
        };
        let yaml_start_offset = self.line_starts[1]; // 2 行目の先頭
        let yaml_end_offset = self.line_starts[close_line - 1]; // 閉じ --- 行の先頭
        let body_offset = self
            .line_starts
            .get(close_line)
            .copied()
            .unwrap_or(self.source.len());
        let yaml = &self.source[yaml_start_offset..yaml_end_offset];
        self.read_front_matter_yaml(yaml);
        FrontMatter {
            body_offset,
            present: Some(1),
        }
    }

    fn read_front_matter_yaml(&mut self, yaml: &str) {
        // 各トップレベルキーの行番号（front matter は 1 行目の `---` から
        // 始まるので、yaml の i 行目 = ファイルの 2 + i 行目）
        let mut key_lines: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for (i, line) in yaml.lines().enumerate() {
            if line.starts_with([' ', '\t']) || line.trim().is_empty() {
                continue;
            }
            if let Some((key, _)) = line.split_once(':') {
                let key = key.trim().trim_matches(['"', '\'']);
                key_lines.entry(key).or_insert(2 + i);
            }
        }
        let line_of_key = |key: &str| key_lines.get(key).copied().unwrap_or(2);

        let value: serde_yaml::Value = match serde_yaml::from_str(yaml) {
            Ok(v) => v,
            Err(e) => {
                self.error(
                    "invalid-frontmatter",
                    2,
                    format!("front matter の YAML が解析できません: {e}"),
                );
                return;
            }
        };
        if value.is_null() {
            self.error(
                "missing-scene-id",
                1,
                "front matter が空です。プロジェクト内で一意なシーン ID を書いてください"
                    .to_string(),
            )
            .suggestion = Some("id: scene_id".to_string());
            return;
        }
        let Some(mapping) = value.as_mapping() else {
            self.error(
                "invalid-frontmatter",
                2,
                "front matter は `キー: 値` のマッピングで書いてください".to_string(),
            );
            return;
        };
        for (key, val) in mapping {
            let Some(key) = key.as_str() else {
                self.error(
                    "invalid-frontmatter",
                    2,
                    "front matter のキーが文字列ではありません".to_string(),
                );
                continue;
            };
            match key {
                "id" | "background" | "bgm" => {
                    let line = line_of_key(key);
                    let Some(s) = val.as_str() else {
                        self.error(
                            "invalid-frontmatter",
                            line,
                            format!("front matter の `{key}` は文字列で書いてください"),
                        );
                        continue;
                    };
                    match key {
                        "id" => {
                            self.scene.id = Some(s.to_string());
                            self.fm_spans.id = Some(line);
                        }
                        "background" => {
                            self.scene.background = Some(s.to_string());
                            self.fm_spans.background = Some(line);
                        }
                        _ => {
                            self.scene.bgm = Some(s.to_string());
                            self.fm_spans.bgm = Some(line);
                        }
                    }
                }
                unknown => {
                    self.warning(
                        "unknown-frontmatter-key",
                        line_of_key(unknown),
                        format!(
                            "front matter の `{unknown}` は v1 では定義されていないキーです（使えるのは id / background / bgm）"
                        ),
                    );
                }
            }
        }
        if self.scene.id.is_none() {
            self.error(
                "missing-scene-id",
                1,
                "front matter に `id` がありません。プロジェクト内で一意なシーン ID を書いてください"
                    .to_string(),
            )
            .suggestion = Some("id: scene_id".to_string());
        }
    }

    // ------------------------------------------------------------- 本文walk

    fn walk(&mut self, events: &[(Event, Range<usize>)]) {
        let mut i = 0;
        while i < events.len() {
            let (event, range) = &events[i];
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    i = self.consume_heading(events, i, *level, range);
                }
                Event::Start(Tag::Paragraph) => {
                    i = self.consume_paragraph(events, i);
                }
                Event::Start(Tag::List(_)) => {
                    i = self.consume_list(events, i, range);
                }
                Event::Html(html) | Event::InlineHtml(html) => {
                    let line = self.line_of(range);
                    self.consume_html(html, line);
                    i += 1;
                }
                Event::Start(Tag::HtmlBlock) | Event::End(TagEnd::HtmlBlock) => {
                    i += 1;
                }
                Event::Rule => {
                    // 空行を挟んだ水平線は区切りとして無視する（SPEC 4.6）
                    i += 1;
                }
                Event::Start(tag) => {
                    // 引用・コードブロック・テーブル・画像など v1 で
                    // 意味を定義していない要素（SPEC 4.6）
                    let line = self.line_of(range);
                    self.warning(
                        "unsupported-element",
                        line,
                        format!(
                            "{} は v1 では解釈されません（無視します）。使えるのは見出し・段落・リンクのみのリスト・HTML コメントです",
                            tag_name(tag)
                        ),
                    );
                    i = skip_to_end(events, i);
                }
                _ => {
                    i += 1;
                }
            }
        }
    }

    /// 見出しを読み切る。戻り値は次のイベント位置
    fn consume_heading(
        &mut self,
        events: &[(Event, Range<usize>)],
        start: usize,
        level: HeadingLevel,
        range: &Range<usize>,
    ) -> usize {
        let line = self.line_of(range);
        let (text, next) = collect_text(events, start, TagEnd::Heading(level));

        // setext 見出し（下線式）は見出しとして扱わない（SPEC 3.2）
        let body = &self.source[self.body_offset..];
        let is_atx = body[range.start..].trim_start().starts_with('#');
        if !is_atx {
            self.warning(
                "setext-heading",
                line,
                format!(
                    "「{text}」の直下の `---` により見出しとして書かれていますが、v1 では下線式（setext）を見出しとして扱いません。セクションにするなら `## {text}` を使ってください"
                ),
            );
            if !text.is_empty() {
                self.push_block(Block::Narration { text, line });
            }
            return next;
        }

        match level {
            HeadingLevel::H1 => {
                if self.scene.title.is_some() {
                    let first = self.title_line.unwrap_or(line);
                    self.error(
                        "invalid-h1",
                        line,
                        format!(
                            "H1 は 1 ファイルに 1 つだけです（{first} 行目で定義済み）。セクションにするなら `## {text}` を使ってください"
                        ),
                    )
                    .related_spans
                    .push(super::Span { line: first });
                } else if !self.scene.lead.is_empty() || !self.scene.sections.is_empty() {
                    self.error(
                        "invalid-h1",
                        line,
                        "H1 タイトルはファイル先頭（front matter 直後）に書いてください"
                            .to_string(),
                    );
                    self.scene.title = Some(text);
                    self.title_line = Some(line);
                } else {
                    self.scene.title = Some(text);
                    self.title_line = Some(line);
                }
            }
            HeadingLevel::H2 => {
                // 先に直前のセクションを確定してから重複判定する
                if let Some(section) = self.current_section.take() {
                    self.scene.sections.push(section);
                }
                let anchor = slugify(&text);
                if anchor.is_empty() {
                    self.error(
                        "empty-anchor",
                        line,
                        format!(
                            "見出し「{text}」からアンカー名を導出できません。英数字か日本語を含む見出しにしてください"
                        ),
                    );
                } else if let Some(prev) = self
                    .scene
                    .sections
                    .iter()
                    .find(|s| s.anchor == anchor)
                    .map(|s| s.line)
                {
                    self.error(
                        "duplicate-anchor",
                        line,
                        format!(
                            "アンカー名「{anchor}」は {prev} 行目の見出しと重複しています。見出しテキストを変えてください"
                        ),
                    )
                    .related_spans
                    .push(super::Span { line: prev });
                }
                self.current_section = Some(Section {
                    heading: text,
                    anchor,
                    line,
                    blocks: Vec::new(),
                });
            }
            _ => {
                self.warning(
                    "deep-heading",
                    line,
                    format!(
                        "H3 以深の見出し「{text}」は v1 では未対応です（無視します）。分岐先にするなら `## {text}` を使ってください"
                    ),
                );
            }
        }
        next
    }

    /// 段落を読み切り、Jump / Dialogue / Narration に分類する
    fn consume_paragraph(&mut self, events: &[(Event, Range<usize>)], start: usize) -> usize {
        let line = self.line_of(&events[start].1);
        let mut i = start + 1;
        let mut text = String::new();
        let mut links: Vec<(String, String, usize)> = Vec::new(); // (label, href, line)
        let mut in_link: Option<(String, String, usize)> = None;

        while i < events.len() {
            let (event, range) = &events[i];
            match event {
                Event::End(TagEnd::Paragraph) => {
                    i += 1;
                    break;
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    in_link = Some((String::new(), dest_url.to_string(), self.line_of(range)));
                }
                Event::End(TagEnd::Link) => {
                    if let Some((label, href, link_line)) = in_link.take() {
                        links.push((label, href, link_line));
                        text.push_str(links.last().map(|(l, _, _)| l.as_str()).unwrap_or(""));
                    }
                }
                Event::Text(t) | Event::Code(t) => match &mut in_link {
                    Some((label, _, _)) => label.push_str(t),
                    None => text.push_str(t),
                },
                Event::SoftBreak | Event::HardBreak => {
                    // 段落内の改行は同一発話の継続（SPEC 4.2）
                    match &mut in_link {
                        Some((label, _, _)) => label.push('\n'),
                        None => text.push('\n'),
                    }
                }
                Event::InlineHtml(html) => {
                    let l = self.line_of(range);
                    self.consume_html(html, l);
                }
                Event::Start(Tag::Image { .. }) => {
                    let l = self.line_of(range);
                    self.warning(
                        "unsupported-element",
                        l,
                        "画像は v1 では解釈されません（無視します）。背景は front matter の background に書いてください".to_string(),
                    );
                    i = skip_to_end(events, i);
                    continue;
                }
                // 強調などの装飾はテキストだけを残す
                Event::Start(_) | Event::End(_) => {}
                _ => {}
            }
            i += 1;
        }

        // リンク 1 つだけの段落はジャンプ（SPEC 4.4）
        let non_link_text: String = {
            let mut t = text.clone();
            for (label, _, _) in &links {
                t = t.replacen(label.as_str(), "", 1);
            }
            t
        };
        if links.len() == 1 && non_link_text.trim().is_empty() {
            let (label, href, link_line) = links.into_iter().next().expect("len checked");
            if let Some(target) = self.parse_link_target(&href, link_line) {
                self.push_block(Block::Jump {
                    label,
                    target,
                    line,
                });
            } else if !label.is_empty() {
                self.push_block(Block::Narration { text: label, line });
            }
            return i;
        }

        // 本文中のインラインリンクは意味を持たない（SPEC 4.4）
        for (label, href, link_line) in &links {
            self.warning(
                "inline-link",
                *link_line,
                format!(
                    "本文中のリンク「[{label}]({href})」は v1 では意味を持ちません。ジャンプのつもりなら、リンクだけの段落として独立させてください"
                ),
            );
        }

        let text = text.trim().to_string();
        if text.is_empty() {
            return i;
        }
        if let Some(diag) = legacy_command(&text) {
            let (message, suggestion) = diag;
            self.error("legacy-command", line, message).suggestion = Some(suggestion);
            self.push_block(Block::Narration { text, line });
            return i;
        }
        match split_dialogue(&text) {
            Some((speaker, body)) => self.push_block(Block::Dialogue {
                speaker,
                text: body,
                line,
            }),
            None => self.push_block(Block::Narration { text, line }),
        }
        i
    }

    /// リストを読み切り、選択肢ブロックに分類する（SPEC 4.3）
    fn consume_list(
        &mut self,
        events: &[(Event, Range<usize>)],
        start: usize,
        range: &Range<usize>,
    ) -> usize {
        let list_line = self.line_of(range);
        struct Item {
            line: usize,
            links: Vec<(String, String)>, // (label, href)
            extra_text: String,
        }
        let mut items: Vec<Item> = Vec::new();
        let mut in_link: Option<(String, String)> = None;
        let mut i = start + 1;
        let mut depth = 1usize; // List の入れ子深さ

        while i < events.len() && depth > 0 {
            let (event, ev_range) = &events[i];
            match event {
                Event::Start(Tag::List(_)) => {
                    // ネストしたリストは v1 非対応（SPEC 4.3）
                    let l = self.line_of(ev_range);
                    self.warning(
                        "unsupported-element",
                        l,
                        "ネストしたリストは v1 では解釈されません（無視します）。選択肢は 1 段のリストで書いてください".to_string(),
                    );
                    i = skip_to_end(events, i);
                    continue;
                }
                Event::End(TagEnd::List(_)) => {
                    depth -= 1;
                }
                Event::Start(Tag::Item) => {
                    items.push(Item {
                        line: self.line_of(ev_range),
                        links: Vec::new(),
                        extra_text: String::new(),
                    });
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    in_link = Some((String::new(), dest_url.to_string()));
                }
                Event::End(TagEnd::Link) => {
                    if let (Some(link), Some(item)) = (in_link.take(), items.last_mut()) {
                        item.links.push(link);
                    }
                }
                Event::Text(t) | Event::Code(t) => {
                    if let Some((label, _)) = &mut in_link {
                        label.push_str(t);
                    } else if let Some(item) = items.last_mut() {
                        item.extra_text.push_str(t);
                    }
                }
                _ => {}
            }
            i += 1;
        }

        if items.iter().all(|it| it.links.is_empty()) {
            self.warning(
                "linkless-list",
                list_line,
                "リンクを 1 つも含まないリストです。選択肢のつもりなら各項目を `- [ラベル](#飛び先)` の形にしてください（`tsumugai fmt` で変換できます）".to_string(),
            );
            return i;
        }

        let mut choice_items: Vec<ChoiceItem> = Vec::new();
        for item in items {
            if item.links.len() != 1 || !item.extra_text.trim().is_empty() {
                self.error(
                    "invalid-choice-item",
                    item.line,
                    "選択肢の項目は 1 つのリンクだけで書いてください（例: `- [ラベル](#飛び先)`）。リンク以外のテキストや複数リンクは混ぜられません".to_string(),
                );
                continue;
            }
            let (label, href) = item.links.into_iter().next().expect("len checked");
            if label.trim().is_empty() {
                self.error(
                    "empty-choice-label",
                    item.line,
                    "選択肢のリンクテキストが空です。プレイヤーに表示する文言を書いてください"
                        .to_string(),
                );
                continue;
            }
            if let Some(target) = self.parse_link_target(&href, item.line) {
                choice_items.push(ChoiceItem {
                    label: label.trim().to_string(),
                    target,
                    line: item.line,
                });
            }
        }
        if !choice_items.is_empty() {
            self.push_block(Block::Choices {
                items: choice_items,
                line: list_line,
            });
        }
        i
    }

    /// HTML コメントから ending 等の制御情報を読む（SPEC 4.5）
    fn consume_html(&mut self, html: &str, line: usize) {
        let mut rest = html;
        let mut saw_comment = false;
        while let Some(open) = rest.find("<!--") {
            let Some(close) = rest[open..].find("-->") else {
                break;
            };
            saw_comment = true;
            let inner = rest[open + 4..open + close].trim();
            self.consume_comment(inner, line);
            rest = &rest[open + close + 3..];
        }
        if !saw_comment && !html.trim().is_empty() {
            self.warning(
                "unsupported-element",
                line,
                "HTML は v1 では解釈されません（無視します）".to_string(),
            );
        }
    }

    fn consume_comment(&mut self, inner: &str, line: usize) {
        // `key: value` 形式だけが制御情報。それ以外はメモとして無視する
        let Some((key, value)) = inner.split_once(':') else {
            return;
        };
        let key = key.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return; // 自由文のメモ
        }
        let value = value.trim();
        if key == "ending" {
            if !value.is_empty()
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                self.push_block(Block::Ending {
                    id: value.to_string(),
                    line,
                });
            } else {
                self.warning(
                    "unknown-directive",
                    line,
                    format!(
                        "ending id「{value}」に使えるのは英数字・ハイフン・アンダースコアだけです"
                    ),
                );
            }
        } else {
            self.warning(
                "unknown-directive",
                line,
                format!(
                    "`<!-- {key}: ... -->` は v1 では定義されていない制御情報です（使えるのは ending のみ）"
                ),
            );
        }
    }

    /// href を LinkTarget に解析する。URL などプロジェクト外は None + Diagnostic
    fn parse_link_target(&mut self, href: &str, line: usize) -> Option<LinkTarget> {
        if has_url_scheme(href) {
            self.error(
                "broken-link",
                line,
                format!(
                    "「{href}」は飛び先にできません。飛び先にできるのはプロジェクト内の Markdown ファイルとアンカー（#見出し）だけです"
                ),
            );
            return None;
        }
        let (file_part, anchor_part) = match href.split_once('#') {
            Some((f, a)) => (f, Some(a)),
            None => (href, None),
        };
        let file = (!file_part.is_empty()).then(|| file_part.to_string());
        let anchor = anchor_part.map(percent_decode).filter(|a| !a.is_empty());
        if file.is_none() && anchor.is_none() {
            self.error(
                "broken-link",
                line,
                "リンク先が空です。`#見出し` または `ファイル.md` を指定してください".to_string(),
            );
            return None;
        }
        Some(LinkTarget { file, anchor })
    }
}

// ------------------------------------------------------------------ helpers

/// Start イベント位置から対応する End までスキップし、次の位置を返す
fn skip_to_end(events: &[(Event, Range<usize>)], start: usize) -> usize {
    let mut depth = 0usize;
    let mut i = start;
    while i < events.len() {
        match &events[i].0 {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    i
}

/// Start イベント位置から対応する End までのテキストを集める
fn collect_text(
    events: &[(Event, Range<usize>)],
    start: usize,
    end_tag: TagEnd,
) -> (String, usize) {
    let mut text = String::new();
    let mut i = start + 1;
    while i < events.len() {
        match &events[i].0 {
            Event::End(t) if *t == end_tag => return (text.trim().to_string(), i + 1),
            Event::Text(t) | Event::Code(t) => text.push_str(t),
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
        i += 1;
    }
    (text.trim().to_string(), i)
}

/// `名前: 本文` のセリフ判定（SPEC 4.2）
///
/// コロン（半角 `:` / 全角 `：`）より前が空白を含まない場合だけセリフ。
fn split_dialogue(text: &str) -> Option<(String, String)> {
    let colon = text.find([':', '：'])?;
    let speaker = &text[..colon];
    if speaker.is_empty() || speaker.chars().any(char::is_whitespace) {
        return None;
    }
    let rest = text[colon..]
        .trim_start_matches([':', '：'])
        .trim_start()
        .to_string();
    Some((speaker.to_string(), rest))
}

/// 旧記法（v0）の検出と書き換え案内（SPEC 6 `legacy-command`、11.1 対応表）
fn legacy_command(text: &str) -> Option<(String, String)> {
    if let Some(rest) = text.strip_prefix(":::") {
        let name = rest.split_whitespace().next().unwrap_or("");
        return Some((
            format!("`:::{name}` ブロックは旧記法（v0）です。v1 では使えません"),
            match name {
                "choices" => "選択肢はリンクだけのリストで書いてください（例: `- [ラベル](#飛び先)`）".to_string(),
                "route" => "分岐先は H2 見出しで書いてください（例: `## 飛び先`）".to_string(),
                _ => "変数・フラグ・条件分岐は v1 では非対応です。SPEC.md 11.1 の対応表を参照してください".to_string(),
            },
        ));
    }
    let inner = text.strip_prefix('[')?;
    let name: String = inner
        .chars()
        .take_while(|c| c.is_ascii_uppercase() || *c == '_')
        .collect();
    if name.is_empty() || !inner[name.len()..].starts_with([' ', ']']) {
        return None;
    }
    let message = format!("`[{name} ...]` は旧記法（v0）の括弧コマンドです。v1 では使えません");
    let suggestion = match name.as_str() {
        "SAY" => {
            "セリフは「名前: 本文」の形で書いてください（例: `あゆみ: こんにちは。`）".to_string()
        }
        "BRANCH" => {
            "選択肢はリンクだけのリストで書いてください（例: `- [ラベル](#飛び先)`）".to_string()
        }
        "LABEL" => "分岐先は H2 見出しで書いてください（例: `## 飛び先`）".to_string(),
        "JUMP" => "ジャンプはリンクだけの段落で書いてください（例: `[次へ](#飛び先)`）".to_string(),
        "SHOW_IMAGE" | "PLAY_MUSIC" | "PLAY_BGM" | "PLAY_SE" => {
            "背景・BGM は front matter に書いてください（例: `background: ../assets/bg/xxx.png`）"
                .to_string()
        }
        "WAIT" => {
            "WAIT は v1 で廃止されました。演出は compile 先（Ren'Py）で調整してください".to_string()
        }
        "SET" => {
            "変数操作は v1 では非対応です（v2 候補）。SPEC.md 9章を参照してください".to_string()
        }
        _ => "SPEC.md 11.1 の対応表を参照して v1 記法に書き換えてください".to_string(),
    };
    Some((message, suggestion))
}

/// unsupported-element の警告メッセージ用の要素名
fn tag_name(tag: &Tag) -> &'static str {
    match tag {
        Tag::BlockQuote(_) => "引用（>）",
        Tag::CodeBlock(_) => "コードブロック",
        Tag::Table(_) => "テーブル",
        Tag::Image { .. } => "画像",
        Tag::FootnoteDefinition(_) => "脚注",
        _ => "この Markdown 要素",
    }
}

/// href が URL スキームを持つか（`https:` など）
fn has_url_scheme(href: &str) -> bool {
    let Some(colon) = href.find(':') else {
        return false;
    };
    let scheme = &href[..colon];
    !scheme.is_empty()
        && scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '.' || c == '-')
        // `#` より後のコロンはアンカー内文字なのでスキームではない
        && href.find('#').is_none_or(|h| colon < h)
}
