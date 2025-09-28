//! Debug branch parsing

use tsumugai::{parser, types::ast::AstNode};

fn main() -> anyhow::Result<()> {
    let scenario = r#"
[BRANCH choice=森の道 choice=山の道]

[LABEL name=森の道]
[SAY speaker=ガイド]
森の道を選びました。

[LABEL name=山の道]
[SAY speaker=ガイド]
山の道を選びました。
"#;

    println!("=== BRANCH デバッグ ===\n");

    let ast = parser::parse(scenario)?;

    println!("パースされたAST:");
    println!("ノード数: {}", ast.nodes.len());
    println!("ラベル: {:?}", ast.labels);

    for (i, node) in ast.nodes.iter().enumerate() {
        println!("\nノード {}: {:?}", i, node);

        if let AstNode::Branch { choices } = node {
            println!("  選択肢の詳細:");
            for (j, choice) in choices.iter().enumerate() {
                println!(
                    "    {}: id='{}', label='{}', target='{}'",
                    j, choice.id, choice.label, choice.target
                );
            }
        }
    }

    Ok(())
}
