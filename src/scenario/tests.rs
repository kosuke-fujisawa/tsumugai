//! v1 パーサーのテスト
//!
//! SPEC.md の各節に対応する。examples/spring/ は仕様網羅サンプルであり
//! Diagnostic 0 件で解析できることが #75 の完了条件。

use super::*;
use std::path::Path;

fn parse(source: &str) -> Parsed {
    parse_str(source, Path::new("test.md"))
}

/// front matter と H1 を補って本文だけをテストするヘルパー
fn parse_body(body: &str) -> Parsed {
    parse(&format!("---\nid: test\n---\n\n# テスト\n\n{body}\n"))
}

fn rule_ids(parsed: &Parsed) -> Vec<&'static str> {
    parsed.diagnostics.iter().map(|d| d.rule_id).collect()
}

fn all_blocks(parsed: &Parsed) -> Vec<&Block> {
    parsed
        .scene
        .lead
        .iter()
        .chain(parsed.scene.sections.iter().flat_map(|s| s.blocks.iter()))
        .collect()
}

// ------------------------------------------------------------ サンプル網羅

#[test]
fn examples_springが診断0件で解析できる() {
    for name in ["spring_001.md", "spring_002.md"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/spring/scenario")
            .join(name);
        let parsed = parse_file(&path).unwrap();
        assert_eq!(
            parsed.diagnostics,
            vec![],
            "{name} は仕様網羅サンプルなので Diagnostic 0 件のはず"
        );
    }
}

#[test]
fn spring_001の構造が仕様どおりに解釈される() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/spring/scenario/spring_001.md");
    let parsed = parse_file(&path).unwrap();
    let scene = &parsed.scene;

    assert_eq!(scene.id.as_deref(), Some("spring_001"));
    assert_eq!(scene.title.as_deref(), Some("春・出会い"));
    assert_eq!(
        scene.background.as_deref(),
        Some("../assets/bg/school_gate.png")
    );
    assert_eq!(scene.bgm.as_deref(), Some("../assets/bgm/spring.ogg"));

    // リード部: ナレーション → セリフ×2 → ナレーション
    assert_eq!(scene.lead.len(), 4);
    assert!(matches!(&scene.lead[0], Block::Narration { line: 9, .. }));
    assert!(
        matches!(&scene.lead[1], Block::Dialogue { speaker, line: 11, .. } if speaker == "幼なじみ")
    );

    // セクション: 選択肢 / run-together / walk-together
    let anchors: Vec<&str> = scene.sections.iter().map(|s| s.anchor.as_str()).collect();
    assert_eq!(anchors, vec!["選択肢", "run-together", "walk-together"]);

    // 選択肢: ファイル内×2 + 別ファイル×1
    let Block::Choices { items, .. } = &scene.sections[0].blocks[0] else {
        panic!(
            "選択肢セクションの先頭は Choices のはず: {:?}",
            scene.sections[0].blocks
        );
    };
    assert_eq!(items.len(), 3);
    assert_eq!(
        items[0].target,
        LinkTarget {
            file: None,
            anchor: Some("run-together".into())
        }
    );
    assert_eq!(
        items[2].target,
        LinkTarget {
            file: Some("spring_002.md".into()),
            anchor: None
        }
    );

    // run-together はエンディングで終わる
    assert!(matches!(
        scene.sections[1].blocks.last(),
        Some(Block::Ending { id, .. }) if id == "childhood_route"
    ));
    // walk-together は別ファイル内アンカーへのジャンプで終わる
    assert!(matches!(
        scene.sections[2].blocks.last(),
        Some(Block::Jump { target: LinkTarget { file: Some(f), anchor: Some(a) }, .. })
            if f == "spring_002.md" && a == "after-school"
    ));
}

// ---------------------------------------------------------- front matter

#[test]
fn front_matterがないとmissing_scene_idになる() {
    let parsed = parse("# タイトル\n\n本文。\n");
    assert_eq!(rule_ids(&parsed), vec!["missing-scene-id"]);
    assert!(
        parsed.diagnostics[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("id:")
    );
}

#[test]
fn idのないfront_matterはmissing_scene_idになる() {
    let parsed = parse("---\nbackground: bg.png\n---\n\n# t\n\n本文。\n");
    assert_eq!(rule_ids(&parsed), vec!["missing-scene-id"]);
}

#[test]
fn 未知のfront_matterキーはwarningになる() {
    let parsed = parse("---\nid: t\nbgv: x\n---\n\n# t\n\n本文。\n");
    assert_eq!(rule_ids(&parsed), vec!["unknown-frontmatter-key"]);
    assert!(parsed.diagnostics[0].message.contains("bgv"));
}

#[test]
fn 壊れたyamlはinvalid_frontmatterになる() {
    let parsed = parse("---\nid: [\n---\n\n# t\n\n本文。\n");
    assert!(rule_ids(&parsed).contains(&"invalid-frontmatter"));
}

#[test]
fn 文字列でないidはinvalid_frontmatterになる() {
    let parsed = parse("---\nid: [1, 2]\n---\n\n# t\n\n本文。\n");
    assert!(rule_ids(&parsed).contains(&"invalid-frontmatter"));
}

#[test]
fn 閉じられていないfront_matterはinvalid_frontmatterになる() {
    let parsed = parse("---\nid: t\n");
    assert!(rule_ids(&parsed).contains(&"invalid-frontmatter"));
}

// ------------------------------------------------------------------ セリフ

#[test]
fn 半角コロンと全角コロンの両方がセリフになる() {
    let parsed = parse_body("あゆみ: こんにちは。\n\n先生：おはよう。");
    assert_eq!(parsed.diagnostics, vec![]);
    let blocks = all_blocks(&parsed);
    assert!(
        matches!(blocks[0], Block::Dialogue { speaker, text, .. } if speaker == "あゆみ" && text == "こんにちは。")
    );
    assert!(
        matches!(blocks[1], Block::Dialogue { speaker, text, .. } if speaker == "先生" && text == "おはよう。")
    );
}

#[test]
fn コロン前に空白があればナレーションになる() {
    let parsed = parse_body("これは 昼下がり: の物語。");
    assert_eq!(parsed.diagnostics, vec![]);
    assert!(matches!(all_blocks(&parsed)[0], Block::Narration { .. }));
}

#[test]
fn 段落内の改行は同一発話の継続になる() {
    let parsed = parse_body("あゆみ: こんにちは。\n今日はいい天気だね。");
    let blocks = all_blocks(&parsed);
    assert!(
        matches!(blocks[0], Block::Dialogue { text, .. } if text == "こんにちは。\n今日はいい天気だね。")
    );
}

// ------------------------------------------------------------------ 選択肢

#[test]
fn 番号付きリストも選択肢になる() {
    let parsed = parse_body("## a\n\n本文。\n\n1. [走る](#a)\n2. [歩く](#a)");
    assert_eq!(parsed.diagnostics, vec![]);
    let choices = all_blocks(&parsed)
        .into_iter()
        .find(|b| matches!(b, Block::Choices { .. }));
    assert!(choices.is_some());
}

#[test]
fn リンクとテキストが混ざる項目はinvalid_choice_itemになる() {
    let parsed = parse_body("- [走る](#a) おすすめ\n- [歩く](#b)");
    assert_eq!(rule_ids(&parsed), vec!["invalid-choice-item"]);
    // エラー項目を除いた選択肢は生きている（エラー回復）
    let blocks = all_blocks(&parsed);
    assert!(matches!(&blocks[0], Block::Choices { items, .. } if items.len() == 1));
}

#[test]
fn リンクのないリストはlinkless_listのwarningになる() {
    let parsed = parse_body("- 走る\n- 歩く");
    assert_eq!(rule_ids(&parsed), vec!["linkless-list"]);
    assert!(parsed.diagnostics[0].message.contains("fmt"));
    assert!(all_blocks(&parsed).is_empty());
}

#[test]
fn 空のリンクテキストはempty_choice_labelになる() {
    let parsed = parse_body("- [](#a)\n- [歩く](#b)");
    assert_eq!(rule_ids(&parsed), vec!["empty-choice-label"]);
}

#[test]
fn ネストしたリストはunsupported_elementになる() {
    let parsed = parse_body("- [走る](#a)\n  - [全力で](#b)");
    assert!(rule_ids(&parsed).contains(&"unsupported-element"));
}

#[test]
fn urlスキームの飛び先はbroken_linkになる() {
    let parsed = parse_body("- [外部サイト](https://example.com)\n- [歩く](#a)");
    assert_eq!(rule_ids(&parsed), vec!["broken-link"]);
}

#[test]
fn パーセントエンコードされたアンカーはデコードして解決される() {
    let parsed = parse_body("- [選ぶ](#%E9%81%B8%E6%8A%9E%E8%82%A2)");
    assert_eq!(parsed.diagnostics, vec![]);
    let blocks = all_blocks(&parsed);
    assert!(matches!(
        &blocks[0],
        Block::Choices { items, .. } if items[0].target.anchor.as_deref() == Some("選択肢")
    ));
}

// ---------------------------------------------------------------- ジャンプ

#[test]
fn リンクだけの段落はジャンプになる() {
    let parsed = parse_body("[翌朝へ](spring_002.md#朝)");
    assert_eq!(parsed.diagnostics, vec![]);
    assert!(matches!(
        all_blocks(&parsed)[0],
        Block::Jump { label, target: LinkTarget { file: Some(f), anchor: Some(a) }, .. }
            if label == "翌朝へ" && f == "spring_002.md" && a == "朝"
    ));
}

#[test]
fn 本文中のインラインリンクはinline_linkのwarningになる() {
    let parsed = parse_body("[翌朝へ](spring_002.md) と彼は思った。");
    assert_eq!(rule_ids(&parsed), vec!["inline-link"]);
    // テキストはナレーションとして残る
    assert!(matches!(
        all_blocks(&parsed)[0],
        Block::Narration { text, .. } if text == "翌朝へ と彼は思った。"
    ));
}

// ------------------------------------------------------------------ 見出し

#[test]
fn h1が2つあるとinvalid_h1になる() {
    let parsed = parse("---\nid: t\n---\n\n# 一つ目\n\n# 二つ目\n\n本文。\n");
    assert_eq!(rule_ids(&parsed), vec!["invalid-h1"]);
    assert_eq!(parsed.diagnostics[0].related_spans.len(), 1);
}

#[test]
fn h1がないとmissing_titleになる() {
    let parsed = parse("---\nid: t\n---\n\n本文。\n");
    assert_eq!(rule_ids(&parsed), vec!["missing-title"]);
}

#[test]
fn h3以深はdeep_headingのwarningになる() {
    let parsed = parse_body("### 深い見出し\n\n本文。");
    assert_eq!(rule_ids(&parsed), vec!["deep-heading"]);
}

#[test]
fn 記号のみの見出しはempty_anchorになる() {
    let parsed = parse_body("## !?\n\n本文。");
    assert_eq!(rule_ids(&parsed), vec!["empty-anchor"]);
}

#[test]
fn 同名の見出しはduplicate_anchorになる() {
    let parsed = parse_body("## 朝\n\n一つ目。\n\n## 朝\n\n二つ目。");
    assert_eq!(rule_ids(&parsed), vec!["duplicate-anchor"]);
    assert_eq!(parsed.diagnostics[0].related_spans.len(), 1);
}

#[test]
fn setext見出しは見出しとして扱われずwarningになる() {
    let parsed = parse_body("区切りのつもり\n---\n\n本文。");
    assert_eq!(rule_ids(&parsed), vec!["setext-heading"]);
    // セクションは作られず、テキストは本文として残る
    assert_eq!(parsed.scene.sections.len(), 0);
}

#[test]
fn 空行を挟んだ水平線は無視される() {
    let parsed = parse_body("前半。\n\n---\n\n後半。");
    assert_eq!(parsed.diagnostics, vec![]);
    assert_eq!(all_blocks(&parsed).len(), 2);
}

// -------------------------------------------------------------- エンディング

#[test]
fn endingコメントでエンディングになる() {
    let parsed = parse_body("本文。\n\n<!-- ending: good_end -->");
    assert_eq!(parsed.diagnostics, vec![]);
    assert!(matches!(
        all_blocks(&parsed)[1],
        Block::Ending { id, .. } if id == "good_end"
    ));
}

#[test]
fn 不正なending_idはunknown_directiveになる() {
    let parsed = parse_body("<!-- ending: ハッピー -->");
    assert_eq!(rule_ids(&parsed), vec!["unknown-directive"]);
}

#[test]
fn 未知の制御キーはunknown_directiveになる() {
    let parsed = parse_body("<!-- bgm: change.ogg -->");
    assert_eq!(rule_ids(&parsed), vec!["unknown-directive"]);
}

#[test]
fn 自由文のhtmlコメントはメモとして無視される() {
    let parsed = parse_body("本文。\n\n<!-- ここは後で書き直す -->");
    assert_eq!(parsed.diagnostics, vec![]);
}

// ------------------------------------------------------------------ 旧記法

#[test]
fn 旧記法の括弧コマンドはlegacy_commandになる() {
    let parsed = parse_body("[SAY speaker=あゆみ]\nこんにちは。");
    assert_eq!(rule_ids(&parsed), vec!["legacy-command"]);
    assert!(
        parsed.diagnostics[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("名前: 本文")
    );
}

#[test]
fn 旧記法のfencedブロックはlegacy_commandになる() {
    let parsed = parse_body(":::choices\n- 走る @run\n:::");
    assert!(rule_ids(&parsed).contains(&"legacy-command"));
}

// ------------------------------------------------------------ 定義外の要素

#[test]
fn 引用とコードブロックはunsupported_elementになる() {
    let parsed = parse_body("> 引用文\n\n```\ncode\n```");
    assert_eq!(
        rule_ids(&parsed),
        vec!["unsupported-element", "unsupported-element"]
    );
    assert!(all_blocks(&parsed).is_empty());
}

// -------------------------------------------------------- characters.yaml

#[test]
fn characters_yamlを祖先ディレクトリから発見して読み込める() {
    let scene_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/spring/scenario/spring_001.md");
    let found = find_characters_file(&scene_path).expect("characters.yaml が見つかるはず");
    let characters = load_characters(&found).unwrap();
    assert!(characters.contains("幼なじみ"));
    assert!(characters.contains("主人公"));
    assert!(!characters.contains("先生"));
}
