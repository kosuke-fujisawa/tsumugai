tsumugai

Markdownでノベルゲームのシナリオを簡単に記述できるスクリプトエンジンです。

特徴

シンプルなMarkdown記法でインタラクティブな物語を記述

シナリオをコマンド列に変換してカスタム実行可能

分岐・台詞表示・BGM再生などに対応

軽量かつLLMフレンドリーな設計

例

# scene: opening

[SAY speaker=Ren]
Welcome to the world.

[PLAY_BGM name=intro.mp3]

[WAIT 2s]

[SAY speaker=Mika]
Are you ready?

[BRANCH choice=Yes label=start, choice=No label=exit]

はじめかた

Cargo.toml に以下を追加してください：

tsumugai = { git = "https://github.com/yourname/tsumugai", tag = "v0.1.0" }

使い方

use tsumugai::{parse, Engine, MockEngine};

let source = std::fs::read_to_string("script.md")?;
let commands = parse(&source)?;

let mut engine = MockEngine::new();
for command in commands {
    engine.execute(&command);
}

ライセンス

MIT License
