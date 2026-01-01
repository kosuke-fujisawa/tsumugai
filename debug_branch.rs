use tsumugai::{parser, types::event::Event};

fn main() {
    let markdown = r#"
[SAY speaker=Narrator]
Choose your path:

[BRANCH choice=left choice=right]

[LABEL name=left]
[SAY speaker=Guide]
You chose the left path.

[LABEL name=right]
[SAY speaker=Guide]
You chose the right path.
"#;

    let ast = parser::parse(markdown).expect("Failed to parse");

    println!("AST nodes:");
    for (i, node) in ast.nodes.iter().enumerate() {
        println!("{}: {:?}", i, node);
    }

    println!("\nLabels:");
    for (label, index) in &ast.labels {
        println!("{} -> {}", label, index);
    }

    // Test what the choices look like specifically
    if let Some(node) = ast.nodes.get(1) {
        if let tsumugai::types::ast::AstNode::Branch { choices } = node {
            println!("\nBranch choices:");
            for (i, choice) in choices.iter().enumerate() {
                println!("Choice {}: id={}, label={}, target={}", i, choice.id, choice.label, choice.target);
            }
        }
    }
}