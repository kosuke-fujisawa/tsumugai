# tsumugai

tsumugai is a Rust library that parses Markdown scenarios into Command sequences and provides step-by-step execution with Directive emission for visual novel-like applications.

The library does NOT implement audio/video playback, rendering, or UI - it only provides the execution logic and tells your application what to do through Directives.

## Features

- **Markdown-based scenarios**: Write scenarios in plain Markdown with simple command syntax
- **Step-by-step execution**: Control flow with `Step::Next`, `Step::Wait(User|Branch|Timer)`, `Step::Jump`, and `Step::Halt`
- **Resource resolution**: Map logical names to file paths with customizable resolvers
- **Variables and branching**: Support for variables, conditions, and user choices
- **Save/load support**: Serialize and restore execution state
- **HTML comment support**: Use `<!-- ... -->` for writing notes that are ignored by the parser

## Quick Start

Add tsumugai to your `Cargo.toml`:

```toml
[dependencies]
tsumugai = "0.1.0"
```

### Basic Example

```rust
use tsumugai::{parse, Engine, Step, WaitKind, Directive};

let markdown = r#"
[SAY speaker=Ayumi]
Hello, world!

[PLAY_BGM name=intro]

[WAIT 1.5s]
"#;

let program = parse(markdown)?;
let mut engine = Engine::new(program);

loop {
    match engine.step() {
        Step::Next => continue,
        Step::Wait(WaitKind::User) => {
            // Handle user input, then continue
            break;
        }
        Step::Wait(WaitKind::Branch(choices)) => {
            // Handle branch selection, then jump
            break;
        }
        Step::Wait(WaitKind::Timer(secs)) => {
            // Handle timer wait
            break;
        }
        Step::Jump(label) => {
            engine.jump_to(&label)?;
        }
        Step::Halt => break,
    }
    
    // Get emitted directives
    let directives = engine.take_emitted();
    for directive in directives {
        match directive {
            Directive::Say { speaker, text } => {
                println!("{}: {}", speaker, text);
            }
            Directive::PlayBgm { res } => {
                println!("Playing BGM: {}", res.logical);
            }
            Directive::Wait { secs } => {
                println!("Waiting {} seconds", secs);
            }
            // Handle other directive types...
            _ => {}
        }
    }
}
```

### Resource Resolution

Use `BasicResolver` or implement your own:

```rust
use tsumugai::{Engine, BasicResolver};

let resolver = BasicResolver::new("assets");
let mut engine = Engine::with_resolver(program, Box::new(resolver));
```

The `BasicResolver` searches for files with these patterns:
- BGM: `assets/bgm/{name}.{ogg,mp3,wav}`
- SE: `assets/se/{name}.{ogg,mp3,wav}`
- Images: `assets/images/{name}.{png,jpg,webp}`
- Movies: `assets/movies/{name}.{mp4,webm}`

## Command Reference

### Basic Commands

- `[SAY speaker=Name] Dialogue text` - Character dialogue
- `[PLAY_BGM name=intro]` - Play background music (non-blocking)
- `[PLAY_SE name=sound]` - Play sound effect (non-blocking)
- `[SHOW_IMAGE file=image]` - Display image (non-blocking)
- `[PLAY_MOVIE file=video]` - Play video (blocking, waits for completion)
- `[WAIT 1.5s]` or `[WAIT secs=1.5]` - Wait for specified seconds

### Control Flow

- `[LABEL name=target]` - Define a jump target
- `[JUMP label=target]` - Jump to label
- `[BRANCH choice=Left label=go_left, choice=Right label=go_right]` - Present choices

### Variables

- `[SET name=score value=10]` - Set variable
- `[MODIFY name=score op=add value=5]` - Modify variable (add/sub)
- `[JUMP_IF var=score cmp=ge value=15 label=success]` - Conditional jump

Comparison operators: `eq`, `ne`, `lt`, `le`, `gt`, `ge`

Variable types: `i32`, `bool`, `string`

### Comments

Use HTML comments for notes that are ignored by the parser:

```markdown
<!-- This is a note about the scene setup -->
[PLAY_BGM name=morning]

<!-- The player should see Ayumi looking happy -->
[SHOW_IMAGE file=ayumi_happy]
```

## Demo Example

Try the demo scenario:

```bash
cargo run --example strange_encounter
```

This demonstrates:
- BGM and image display
- Character dialogue
- User choices with branching
- Resource resolution

## Save/Load

```rust
// Take a snapshot
let save_data = engine.snapshot();

// Restore later
engine.restore(save_data)?;
```

Save data includes:
- Program counter position
- Variable values  
- Optional seen flags and RNG seed for future extensions

## Design Philosophy

- **Resource names use logical names** (no file extensions) - resolved by your Resolver
- **HTML comments are completely ignored** - useful for writing hints and notes
- **No audio/video/rendering implementation** - tsumugai only provides execution logic
- **Step-by-step execution model** - your application controls timing and user input
- **Directive emission** - the engine tells you what to do, you decide how to do it

## License

This project is licensed under the MIT License.