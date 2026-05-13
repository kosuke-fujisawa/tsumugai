//! 統合テスト
//!
//! parser → runtime の連携を end-to-end でテストする。

use serde_json::{Value, json};
use tsumugai::{
    parser,
    runtime::{self, Input, WaitingType, ir::Event, ir::Program},
    types::state::State,
};

// ──────────────────────────────────────────────
// ゴールデンテスト用ヘルパー
// ──────────────────────────────────────────────

fn input_to_json(input: Option<&Input>) -> Value {
    match input {
        None => Value::Null,
        Some(Input::Advance) => json!("Advance"),
        Some(Input::SelectChoice(id)) => json!({ "SelectChoice": id }),
    }
}

fn waiting_for_to_json(wf: Option<&WaitingType>) -> Value {
    match wf {
        None => Value::Null,
        Some(WaitingType::Advance) => json!("Advance"),
        Some(WaitingType::Choice(opts)) => {
            let choices: Vec<Value> = opts
                .iter()
                .map(|o| json!({ "id": o.id, "label": o.label }))
                .collect();
            json!({ "Choice": choices })
        }
        Some(WaitingType::Ended { id, name }) => json!({ "Ended": { "id": id, "name": name } }),
    }
}

fn play_scenario(program: &Program, inputs: &[Option<Input>]) -> Vec<Value> {
    let mut state = State::new();
    let mut steps = vec![];
    for (idx, input) in inputs.iter().enumerate() {
        let (next_state, output) = runtime::step(state, program, input.clone());
        state = next_state;
        let events: Vec<Value> = output
            .events
            .iter()
            .map(|e| serde_json::to_value(e).unwrap())
            .collect();
        steps.push(json!({
            "step": idx,
            "input": input_to_json(input.as_ref()),
            "events": events,
            "waiting_for": waiting_for_to_json(output.waiting_for.as_ref()),
        }));
    }
    steps
}

fn compare_or_update_golden(path: &str, actual: &str) {
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, actual).unwrap();
        eprintln!("Updated golden file: {path}");
        return;
    }
    match std::fs::read_to_string(path) {
        Ok(expected) => assert_eq!(
            actual, expected,
            "Golden file mismatch: {path}\nRun `UPDATE_GOLDEN=1 cargo test` to update."
        ),
        Err(_) => panic!(
            "Golden file not found: {path}\nRun `UPDATE_GOLDEN=1 cargo test` to create it.\n\nActual output:\n{actual}"
        ),
    }
}

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
        // ID は `{scene}_branch_{branch_idx}_choice_{choice_idx}` 形式
        assert!(opts[0].id.ends_with("_choice_0"));
        assert!(opts[1].id.ends_with("_choice_1"));
    } else {
        panic!("選択肢が返されなかった");
    }
}

/// 同一シーン内に複数の BRANCH があっても Choice ID が衝突しない
#[test]
fn 同一シーン内複数branchでid衝突しない() {
    let md = r#"
# scene: shop

[BRANCH choice=買う label=buy, choice=見るだけ label=look]

[LABEL name=buy]
[SAY speaker=店主]
毎度。

[LABEL name=look]
[BRANCH choice=去る label=leave, choice=もう一度 label=look2]

[LABEL name=leave]
[SAY speaker=A]
また来ます。

[LABEL name=look2]
[SAY speaker=A]
やっぱり買います。
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let (state, output) = runtime::step(State::new(), &program, None);
    let opts1 = if let Some(WaitingType::Choice(opts)) = output.waiting_for {
        opts
    } else {
        panic!("1つ目の選択肢が返されなかった");
    };

    // 1つ目のBRANCH: 「見るだけ」を選んで2つ目のBRANCHへ
    let (state, output) = runtime::step(
        state,
        &program,
        Some(Input::SelectChoice(opts1[1].id.clone())),
    );
    let opts2 = if let Some(WaitingType::Choice(opts)) = output.waiting_for {
        opts
    } else {
        panic!("2つ目の選択肢が返されなかった");
    };

    // 2つのBRANCHのID群が完全に異なることを確認
    for id1 in &opts1 {
        for id2 in &opts2 {
            assert_ne!(id1.id, id2.id, "Choice IDが衝突: {}", id1.id);
        }
    }

    // 2つ目のBRANCHから「去る」を選べることも確認
    let (_, output) = runtime::step(
        state,
        &program,
        Some(Input::SelectChoice(opts2[0].id.clone())),
    );
    let text = output
        .events
        .iter()
        .find_map(|e| {
            if let tsumugai::runtime::ir::Event::Say { text, .. } = e {
                Some(text.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert!(text.contains("また来ます"));
}

/// ネストした WhenBlock が正しく動作する
#[test]
fn ネストしたwhenblockが動作する() {
    let md = r#"
[SET name=level value=5]
[SET name=bonus value=true]

:::when level >= 3
:::when bonus == true
[SAY speaker=System]
ボーナス発動！
:::
:::

[SAY speaker=System]
終了
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let (state, output) = runtime::step(State::new(), &program, None);
    let texts: Vec<String> = output
        .events
        .iter()
        .filter_map(|e| {
            if let tsumugai::runtime::ir::Event::Say { text, .. } = e {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("ボーナス発動")),
        "ネストwhenが真のとき内側の台詞が出るはず: {:?}",
        texts
    );

    // Enter で進めると「終了」が来る
    let (_, output2) = runtime::step(state, &program, Some(Input::Advance));
    let has_end = output2.events.iter().any(|e| {
        if let tsumugai::runtime::ir::Event::Say { text, .. } = e {
            text.contains("終了")
        } else {
            false
        }
    });
    assert!(has_end, "終了台詞が来るはず");
}

/// modify_var のエラー（文字列変数への数値演算）が Custom error イベントとして返る
#[test]
fn modify_varのエラーがeventに積まれる() {
    let md = r#"
[SET name=name value=Alice]
[MODIFY name=name op=add value=1]
[SAY speaker=System]
完了
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let (_, output) = runtime::step(State::new(), &program, None);
    let has_error = output.events.iter().any(|e| {
        matches!(
            e,
            tsumugai::runtime::ir::Event::Custom { tag, .. } if tag == "error"
        )
    });
    assert!(
        has_error,
        "文字列への数値演算は error イベントを生成するはず"
    );
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

// ──────────────────────────────────────────────
// check --json
// ──────────────────────────────────────────────

/// 正常シナリオの JSON 出力は status="ok"、issues=[] になる
#[test]
fn check_json_正常シナリオはstatus_ok() {
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
    let output = tsumugai::analyzer::CheckJsonOutput::from(&result);

    let json = serde_json::to_string(&output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(v["status"], "ok");
    assert_eq!(v["error_count"], 0);
    assert!(v["issues"].is_array());
}

/// 警告があるとき status="ok" のまま warning_count が反映される
#[test]
fn check_json_警告ありはstatus_ok_with_warnings() {
    // 選択肢1つの BRANCH → warning
    let md = r#"
[BRANCH choice=進む label=go]

[LABEL name=go]
[SAY speaker=A]
進む
"#;
    let ast = parser::parse(md).unwrap();
    let result = tsumugai::analyzer::analyze(&ast);
    let output = tsumugai::analyzer::CheckJsonOutput::from(&result);

    let json = serde_json::to_string(&output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(v["status"], "ok");
    assert!(v["warning_count"].as_u64().unwrap() > 0);
    let issues = v["issues"].as_array().unwrap();
    assert!(issues.iter().any(|i| i["level"] == "warning"));
}

/// パースエラー時の JSON 出力が valid JSON で status="error" になる
#[test]
fn check_json_parseエラーはstatus_error() {
    let output = tsumugai::analyzer::CheckJsonOutput::parse_error(
        "Undefined label 'missing' referenced".to_string(),
    );

    let json = serde_json::to_string(&output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(v["status"], "error");
    assert_eq!(v["error_count"], 1);
    let issues = v["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0]["level"], "error");
}

/// JSON 出力フォーマットが変わっていないことを確認する Golden テスト
#[test]
fn check_json_フォーマットが安定している() {
    let md = "[SAY speaker=Alice]\nこんにちは。\n";
    let ast = parser::parse(md).unwrap();
    let result = tsumugai::analyzer::analyze(&ast);
    let output = tsumugai::analyzer::CheckJsonOutput::from(&result);

    let json = serde_json::to_string_pretty(&output).unwrap();
    let expected = r#"{
  "status": "ok",
  "error_count": 0,
  "warning_count": 0,
  "issues": []
}"#;
    assert_eq!(json, expected, "JSON フォーマットが意図せず変わっています");
}

/// 正常なシナリオは analyzer でエラーなし
/// ENDING コマンドでシナリオが終了しエンディングIDが出力に含まれる
#[test]
fn endingコマンドでシナリオが終了する() {
    let md = r#"
[SAY speaker=Alice]
これが最後の台詞。

[ENDING id=good name=グッドエンド]
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    // 1回目: Say + AwaitAdvance
    let state = State::new();
    let (state, output) = runtime::step(state, &program, None);
    assert!(matches!(output.waiting_for, Some(WaitingType::Advance)));

    // Enter で進む → Ending イベントが発生し、Ended で停止
    let (_state, output) = runtime::step(state, &program, Some(Input::Advance));

    assert!(output.events.iter().any(|e| matches!(
        e,
        Event::Ending { id, name } if id == "good" && name == "グッドエンド"
    )));
    assert!(matches!(
        output.waiting_for,
        Some(WaitingType::Ended { ref id, ref name }) if id == "good" && name == "グッドエンド"
    ));
}

/// END コマンド（ENDINGの別名）も動作する
#[test]
fn endコマンドがendingと同様に動作する() {
    let md = "[END id=bad]\n";
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let state = State::new();
    let (_state, output) = runtime::step(state, &program, None);

    assert!(
        output
            .events
            .iter()
            .any(|e| matches!(e, Event::Ending { id, .. } if id == "bad"))
    );
    assert!(matches!(
        output.waiting_for,
        Some(WaitingType::Ended { .. })
    ));
}

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

// ──────────────────────────────────────────────
// 条件付き選択肢
// ──────────────────────────────────────────────

/// 条件が真の選択肢は表示され、偽の選択肢は表示されない
#[test]
fn 条件付き選択肢_条件が偽の選択肢は非表示() {
    // has_sword が設定されていない状態 → if=has_sword の選択肢は表示されない
    // パーサーは if= を choice インデックス順に割り当てるので
    // 最初の choice に条件を付ける形式: choice=剣で戦う if=has_sword label=fight
    let md = r#"
[BRANCH choice=剣で戦う if=has_sword label=fight, choice=逃げる label=run]

[LABEL name=fight]
[SAY speaker=主人公]
戦った！

[LABEL name=run]
[SAY speaker=主人公]
逃げた！
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let state = State::new(); // has_sword は未設定
    let (_, output) = runtime::step(state, &program, None);

    let opts = if let Some(WaitingType::Choice(opts)) = output.waiting_for {
        opts
    } else {
        panic!("選択肢が返されなかった");
    };

    assert_eq!(opts.len(), 1, "条件が偽の選択肢は除外されるはず");
    assert_eq!(opts[0].label, "逃げる");
}

/// 条件が真になると選択肢が表示される
#[test]
fn 条件付き選択肢_条件が真の選択肢は表示される() {
    let md = r#"
[SET name=has_sword value=true]
[BRANCH choice=剣で戦う if=has_sword label=fight, choice=逃げる label=run]

[LABEL name=fight]
[SAY speaker=主人公]
戦った！

[LABEL name=run]
[SAY speaker=主人公]
逃げた！
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let state = State::new();
    let (_, output) = runtime::step(state, &program, None);

    let opts = if let Some(WaitingType::Choice(opts)) = output.waiting_for {
        opts
    } else {
        panic!("選択肢が返されなかった");
    };

    assert_eq!(opts.len(), 2, "条件が真なので両方の選択肢が表示されるはず");
}

/// 条件が偽の選択肢 ID を直接送っても無視される（バイパス防止）
#[test]
fn 条件付き選択肢_条件偽の選択肢はバイパスできない() {
    let md = r#"
[BRANCH choice=剣で戦う if=has_sword label=fight, choice=逃げる label=run]

[LABEL name=fight]
[SAY speaker=主人公]
戦った！

[LABEL name=run]
[SAY speaker=主人公]
逃げた！
"#;
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    // has_sword 未設定で fight の選択肢 ID を直接送る
    let state = State::new();
    let (state, output) = runtime::step(state, &program, None);
    let fight_id = if let Some(WaitingType::Choice(_opts)) = &output.waiting_for {
        // 表示されていない fight の ID を IR から直接取得する
        use tsumugai::runtime::ir::Op;
        if let Some(Op::AwaitChoice { options }) = program.get(state.pc) {
            options
                .iter()
                .find(|o| o.label == "剣で戦う")
                .unwrap()
                .id
                .clone()
        } else {
            panic!("AwaitChoice 命令がない");
        }
    } else {
        panic!("選択肢待ちでない");
    };

    // 条件が偽の ID を送ってもジャンプしない（PC はそのまま）
    let pc_before = state.pc;
    let (state_after, _) = runtime::step(state, &program, Some(Input::SelectChoice(fight_id)));
    assert_eq!(
        state_after.pc, pc_before,
        "条件が偽の選択肢はバイパスできないはず"
    );
}

// ──────────────────────────────────────────────
// ゴールデン JSON テスト
// ──────────────────────────────────────────────

/// simple.md の全ステップ出力が変わっていないことを確認する
#[test]
fn golden_simple_events() {
    let md = include_str!("fixtures/simple.md");
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let inputs: Vec<Option<Input>> = vec![None, Some(Input::Advance), Some(Input::Advance)];
    let steps = play_scenario(&program, &inputs);
    let actual = serde_json::to_string_pretty(&steps).unwrap();
    compare_or_update_golden("tests/golden/simple.events.json", &actual);
}

/// branch.md の左右両分岐のステップ出力が変わっていないことを確認する
#[test]
fn golden_branch_events() {
    let md = include_str!("fixtures/branch.md");
    let ast = parser::parse(md).unwrap();
    let program = runtime::compile(&ast);

    let left_inputs: Vec<Option<Input>> = vec![
        None,
        Some(Input::Advance),
        Some(Input::SelectChoice("root_branch_0_choice_0".to_string())),
        Some(Input::Advance),
    ];
    let right_inputs: Vec<Option<Input>> = vec![
        None,
        Some(Input::Advance),
        Some(Input::SelectChoice("root_branch_0_choice_1".to_string())),
        Some(Input::Advance),
    ];

    let result = json!({
        "left": play_scenario(&program, &left_inputs),
        "right": play_scenario(&program, &right_inputs),
    });
    let actual = serde_json::to_string_pretty(&result).unwrap();
    compare_or_update_golden("tests/golden/branch.events.json", &actual);
}
