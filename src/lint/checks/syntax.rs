//! Syntax checking implementation

use crate::lint::LintResult;
use crate::lint::config::LintConfig;
use crate::types::ast::Ast;

/// Check syntax issues in the AST
pub fn check(_ast: &Ast, _result: &mut LintResult, _config: &LintConfig) {
    // Basic syntax checking is already done during parsing
    // This is a placeholder for future syntax-level checks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn syntax_check_valid_scenario() {
        let markdown = r#"
[SAY speaker=Alice]
Hello!
"#;

        let ast = parse(markdown).unwrap();
        let mut result = LintResult::new();
        let config = LintConfig::default();

        check(&ast, &mut result, &config);

        assert!(result.is_clean());
    }
}
