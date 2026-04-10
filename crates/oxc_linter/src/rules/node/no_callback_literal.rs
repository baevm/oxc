use oxc_ast::{AstKind, ast::Expression};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::{GetSpan, Span};

use crate::{AstNode, context::LintContext, rule::Rule};

fn no_callback_literal_diagnostic(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("Unexpected literal in error position of callback.")
        .with_help("Pass an `Error` object (or `null`/`undefined` when there is no error) as the first callback argument.")
        .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct NoCallbackLiteral;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Enforce Node.js-style error-first callback pattern is followed.
    ///
    /// ### Why is this bad?
    ///
    /// When invoking a callback function which uses the Node.js error-first callback pattern, all of your errors should either use the `Error` class or a subclass of it.
    /// It is also acceptable to use `undefined` or `null` if there is no error.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```js
    /// cb('this is an error string');
    /// callback(0);
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```js
    /// cb(undefined);
    /// cb(null, 5);
    /// callback(new Error('some error'));
    /// callback(someVariable);
    /// ```
    NoCallbackLiteral,
    node,
    style,
);

impl Rule for NoCallbackLiteral {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let AstKind::CallExpression(call_expr) = node.kind() else {
            return;
        };

        let Expression::Identifier(ident) = &call_expr.callee else {
            return;
        };

        if ident.name != "callback" && ident.name != "cb" {
            return;
        }

        let Some(error_arg) = call_expr.arguments.first() else {
            return;
        };

        let Some(error_expr) = error_arg.as_expression() else {
            return;
        };

        if could_be_error(error_expr) {
            return;
        }

        ctx.diagnostic(no_callback_literal_diagnostic(error_expr.span()));
    }
}

/// Determine if a node has a possibility to be an Error object
fn could_be_error(error_expr: &Expression) -> bool {
    match error_expr.without_parentheses() {
        Expression::BooleanLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::StringLiteral(_) => false,
        Expression::AssignmentExpression(assign_expr) => could_be_error(&assign_expr.right),
        Expression::SequenceExpression(seq_expr) => {
            let exprs = &seq_expr.expressions;

            let Some(last) = exprs.last() else {
                return false;
            };

            could_be_error(last)
        }
        Expression::LogicalExpression(logic_expr) => {
            could_be_error(&logic_expr.left) || could_be_error(&logic_expr.right)
        }
        Expression::ConditionalExpression(cond_expr) => {
            could_be_error(&cond_expr.consequent) || could_be_error(&cond_expr.alternate)
        }
        _ => true,
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        "horse()",
        "sort(null)",
        r#"require("zyx")"#,
        r#"require("zyx", data)"#,
        "callback()",
        "callback(undefined)",
        "callback(null)",
        "callback(x)",
        r#"callback(new Error("error"))"#,
        "callback(friendly, data)",
        "callback(undefined, data)",
        "callback(null, data)",
        "callback(x, data)",
        r#"callback(new Error("error"), data)"#,
        "callback(x = obj, data)",
        "callback((1, a), data)",
        "callback(a || b, data)",
        "callback(a ? b : c, data)",
        "callback(a ? 1 : c, data)",
        "callback(a ? b : 1, data)",
        "cb()",
        "cb(undefined)",
        "cb(null)",
        r#"cb(undefined, "super")"#,
        r#"cb(null, "super")"#,
        "cb(e as Error)",           // { "parser": tsParser }
        r#"cb("help" as unknown)"#, // { "parser": tsParser }
        "cb({ a: 1 })",
        "cb([])",
    ];

    let fail = vec![
        r#"callback(false, "snork")"#,
        r#"callback("help")"#,
        r#"callback("help", data)"#,
        "cb(false)",
        r#"cb("help")"#,
        r#"cb("help", data)"#,
        "callback((a, 1), data)",
    ];

    Tester::new(NoCallbackLiteral::NAME, NoCallbackLiteral::PLUGIN, pass, fail).test_and_snapshot();
}
