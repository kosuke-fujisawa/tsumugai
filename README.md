tsumugai

A script engine that makes it easy to write visual novel scenarios in Markdown.

Features

Write interactive stories using simple Markdown syntax

Parse scenarios into command sequences for custom rendering

Support for branching, text display, music playback, and more

Designed to be lightweight and LLM-friendly

Example

# scene: opening

[SAY speaker=Ren]
Welcome to the world.

[PLAY_BGM name=intro.mp3]

[WAIT 2s]

[SAY speaker=Mika]
Are you ready?

[BRANCH choice=Yes label=start, choice=No label=exit]

Getting Started

Add to your Cargo.toml:

tsumugai = { git = "[https://github.com/yourname/tsumugai](https://github.com/kosuke-fujisawa/tsumugai)", tag = "v0.1.0" }

Usage

use tsumugai::{parse, Engine, MockEngine};

let source = std::fs::read_to_string("script.md")?;
let commands = parse(&source)?;

let mut engine = MockEngine::new();
for command in commands {
    engine.execute(&command);
}

License

MIT License
