//! パーサーレベルの静的検証
//!
//! :::conditions ブロックで宣言された条件の整合性チェック。

use crate::types::ast::{Ast, AstNode};
use std::collections::HashSet;

/// 検証結果
#[derive(Debug, Clone, PartialEq)]
pub struct CheckResult {
    pub warnings: Vec<String>,
    pub is_valid: bool,
}

/// AST の静的検証を行う
pub fn check(ast: &Ast) -> CheckResult {
    let mut warnings = Vec::new();

    // 使用されている条件を収集
    let mut used_conditions = HashSet::new();
    for node in &ast.nodes {
        if let AstNode::Branch { choices } = node {
            for choice in choices {
                if let Some(ref cond) = choice.condition {
                    used_conditions.insert(cond.clone());
                    if !ast.conditions.contains(cond) {
                        warnings.push(format!(
                            "未宣言の条件 '{}' が選択肢で使用されています",
                            cond
                        ));
                    }
                }
            }
        }
    }

    // 宣言されているが使用されていない条件
    for declared in &ast.conditions {
        if !used_conditions.contains(declared) {
            warnings.push(format!(
                "条件 '{}' は宣言されていますが使用されていません",
                declared
            ));
        }
    }

    CheckResult {
        is_valid: true,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn 未宣言条件は警告() {
        let md = r#"
[BRANCH choice=右へ if=unknown label=right, choice=左へ label=left]

[LABEL name=right]
[SAY speaker=A]
右

[LABEL name=left]
[SAY speaker=A]
左
"#;
        let ast = parse(md).unwrap();
        let result = check(&ast);
        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("unknown"));
    }

    #[test]
    fn 未使用宣言条件は警告() {
        let md = r#"
:::conditions
unused
can_go
:::

[BRANCH choice=行く if=can_go label=go, choice=戻る label=back]

[LABEL name=go]
[SAY speaker=A]
行く

[LABEL name=back]
[SAY speaker=A]
戻る
"#;
        let ast = parse(md).unwrap();
        let result = check(&ast);
        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("unused"));
    }
}
