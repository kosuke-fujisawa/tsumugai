//! 統合テスト
//!
//! parser → runtime の連携を end-to-end でテストする。

use tsumugai::{
    parser,
    runtime::{self, Input, WaitingType, ir::Event},
    types::state::State,
};

// ──────────────────────────────────────────────
// 基本動作
// ──────────────────────────────────────────────

/// 台詞 → Enter → 終了 の最小シナリオ
#[test]
fn 最小シナリオをプレイできる() {
    let md = "[SAY speaker=Alice]\nこんにちは！\n";
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    // 1回目: Say + AwaitAdvance で停止
    let state = State::new();
    let (state, output) = runtime::step(state, &program, None);
    assert_eq!(output.events.len(), 1);
    assert!(matches!(output.waiting_for, Some(WaitingType::Advance)));

    // Enter で進む
    let (state, output) = runtime::step(state, &program, Some(Input::Advance));
    assert!(output.waiting_for.is_none()); // 終了
    assert_eq!(state.pc, program.len());
}

/// 複数の台詞を順番に進める
#[test]
fn 複数台詞を順番にプレイできる() {
    let md = "[SAY speaker=A]\n台詞1\n[SAY speaker=B]\n台詞2\n";
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let state = State::new();
    let (state, output) = runtime::step(state, &program, None);
    let text = if let Some(Event::Say { text, .. }) = output.events.first() {
        text.clone()
    } else {
        panic!("台詞イベントがない");
    };
    assert_eq!(text, "台詞1");

    let (state, output) = runtime::step(state, &program, Some(Input::Advance));
    let text = if let Some(Event::Say { text, .. }) = output.events.first() {
        text.clone()
    } else {
        panic!("台詞イベントがない");
    };
    assert_eq!(text, "台詞2");

    let (state, _) = runtime::step(state, &program, Some(Input::Advance));
    assert_eq!(state.pc, program.len()); // 終了
}

// ──────────────────────────────────────────────
// 選択肢
// ──────────────────────────────────────────────

/// 選択肢でルートが分岐する
#[test]
fn 選択肢で分岐できる() {
    let md = r#"
[BRANCH choice=左 label=left, choice=右 label=right]

[LABEL name=left]
[SAY speaker=A]
左ルート

[LABEL name=right]
[SAY speaker=A]
右ルート
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    // AwaitChoice で停止
    let state = State::new();
    let (state, output) = runtime::step(state, &program, None);
    let options = if let Some(WaitingType::Choice(opts)) = output.waiting_for {
        opts
    } else {
        panic!("選択肢が返されなかった");
    };
    assert_eq!(options.len(), 2);

    // 2番目（右）を選ぶ
    let right_id = options[1].id.clone();
    let (_, output) = runtime::step(state, &program, Some(Input::SelectChoice(right_id)));
    let text = output
        .events
        .iter()
        .find_map(|e| {
            if let Event::Say { text, .. } = e {
                Some(text.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert!(text.contains("右ルート"));
}

/// 選択肢の ID が決定論的であることを確認
#[test]
fn 選択肢idが決定論的に生成される() {
    let md = r#"
[BRANCH choice=A label=a, choice=B label=b]
[LABEL name=a]
[SAY speaker=X]
A
[LABEL name=b]
[SAY speaker=X]
B
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let (_, output) = runtime::step(State::new(), &program, None);
    if let Some(WaitingType::Choice(opts)) = output.waiting_for {
        // ID は `{scene}_choice_{index}` 形式
        assert!(opts[0].id.ends_with("_choice_0"));
        assert!(opts[1].id.ends_with("_choice_1"));
    } else {
        panic!("選択肢が返されなかった");
    }
}

// ──────────────────────────────────────────────
// 変数と条件
// ──────────────────────────────────────────────

/// 変数セットと JUMP_IF の連携
#[test]
fn 変数によって分岐が変わる() {
    let md = r#"
[SET name=route value=good]
[JUMP_IF var=route cmp=eq value=good label=good_end]

[SAY speaker=A]
バッドエンド

[LABEL name=good_end]
[SAY speaker=A]
グッドエンド
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let (_, output) = runtime::step(State::new(), &program, None);
    let text = output
        .events
        .iter()
        .find_map(|e| {
            if let Event::Say { text, .. } = e {
                Some(text.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert!(text.contains("グッドエンド"));
}

// ──────────────────────────────────────────────
// エフェクト
// ──────────────────────────────────────────────

/// BGM・画像イベントが Output に積まれる
#[test]
fn エフェクトイベントが出力される() {
    let md = r#"
[PLAY_BGM name=intro.mp3]
[SHOW_IMAGE name=castle.png layer=bg]
[SAY speaker=A]
はじまり
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let (_, output) = runtime::step(State::new(), &program, None);

    let has_bgm = output
        .events
        .iter()
        .any(|e| matches!(e, Event::PlayBgm { .. }));
    let has_image = output
        .events
        .iter()
        .any(|e| matches!(e, Event::ShowImage { .. }));
    let has_say = output.events.iter().any(|e| matches!(e, Event::Say { .. }));

    assert!(has_bgm, "BGM イベントがない");
    assert!(has_image, "ShowImage イベントがない");
    assert!(has_say, "Say イベントがない");
}

// ──────────────────────────────────────────────
// analyzer
// ──────────────────────────────────────────────

/// 正常なシナリオは analyzer でエラーなし
#[test]
fn 正常なシナリオはanalyzerでクリーン() {
    let md = r#"
[SAY speaker=Alice]
こんにちは。

[BRANCH choice=はい label=yes, choice=いいえ label=no]

[LABEL name=yes]
[SAY speaker=Alice]
よかった！

[LABEL name=no]
[SAY speaker=Alice]
残念。
"#;
    let ast = parser::parse(md).unwrap();
    let result = tsumugai::analyzer::analyze(&ast);
    assert!(!result.has_errors());
}
