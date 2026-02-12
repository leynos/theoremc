//! Syntactic validation of Rust expression strings using `syn`.
//!
//! This module provides [`validate_rust_expr`], which parses a string as
//! `syn::Expr` and rejects statement-like forms (blocks, loops,
//! assignments, and flow-control constructs) that are not single
//! expressions. It is called from the post-deserialization validation
//! pipeline in `validate.rs`.

/// Validates that `input` is a syntactically valid Rust expression and
/// is not a statement-like form (block, loop, assignment, or
/// flow-control construct).
///
/// Returns `Ok(())` if the input is a valid single expression.  Returns
/// `Err(reason)` with a human-readable reason string if parsing fails
/// or a disallowed form is detected.
///
/// # Examples
///
/// ```rust,ignore
/// use theoremc::schema::expr::validate_rust_expr;
///
/// assert!(validate_rust_expr("x > 0").is_ok());
/// assert!(validate_rust_expr("{ let x = 1; x }").is_err());
/// ```
pub(crate) fn validate_rust_expr(input: &str) -> Result<(), String> {
    let parsed: syn::Expr = syn::parse_str(input)
        .map_err(|err| format!("{}{}", "is not a valid Rust expression: ", err))?;

    if is_statement_like(&parsed) {
        return Err(
            concat!("must be a single expression, ", "not a statement or block",).to_owned(),
        );
    }

    Ok(())
}

/// Returns `true` if the given `syn::Expr` variant is a statement-like
/// form that is disallowed in theorem expressions.
///
/// Rejected forms: blocks, loops (`for`, `while`, `loop`), `let`
/// bindings, `unsafe`/`async`/`const`/`try` blocks, flow-control
/// (`return`, `break`, `continue`, `yield`), assignments, and
/// compound assignments (`+=`, `-=`, etc.).
///
/// Allowed forms: `if`, `match`, closures, function/method calls,
/// binary/unary operations, literals, paths, field access, indexing,
/// casts, references, ranges, tuples, arrays, struct literals, macros,
/// and the `?` operator.
const fn is_statement_like(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Assign(_)
            | syn::Expr::Async(_)
            | syn::Expr::Block(_)
            | syn::Expr::Break(_)
            | syn::Expr::Const(_)
            | syn::Expr::Continue(_)
            | syn::Expr::ForLoop(_)
            | syn::Expr::Let(_)
            | syn::Expr::Loop(_)
            | syn::Expr::Return(_)
            | syn::Expr::TryBlock(_)
            | syn::Expr::Unsafe(_)
            | syn::Expr::While(_)
            | syn::Expr::Yield(_)
    ) || is_compound_assignment(expr)
}

/// Returns `true` if `expr` is a compound assignment operator
/// (`+=`, `-=`, `*=`, `/=`, `%=`, `^=`, `&=`, `|=`, `<<=`, `>>=`).
///
/// In `syn` 2.x, compound assignments are represented as
/// `Expr::Binary` with a `BinOp::*Assign` operator, unlike simple
/// `=` which uses `Expr::Assign`.
const fn is_compound_assignment(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Binary(b) if matches!(
            b.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        )
    )
}

#[cfg(test)]
mod tests {
    //! Unit tests for Rust expression syntax validation.

    use rstest::rstest;

    use super::validate_rust_expr;

    // ── Happy path: valid single expressions ─────────────────────

    #[rstest]
    #[case::bool_literal("true")]
    #[case::comparison("x > 0")]
    #[case::method_call("result.is_valid()")]
    #[case::method_call_with_comparison("result.balance() >= amount")]
    #[case::function_call_with_ref("hnsw.is_bidirectional(&graph)")]
    #[case::negated_call("!hnsw.edge_present(&graph, 2, 0, 1)")]
    #[case::parenthesised_arithmetic("amount <= (u64::MAX - a.balance)")]
    #[case::plain_identifier("x")]
    #[case::if_expression("if x > 0 { a } else { b }")]
    #[case::match_expression("match x { 1 => true, _ => false }")]
    #[case::closure("|x| x > 0")]
    fn given_valid_expression_when_validated_then_accepted(#[case] input: &str) {
        let result = validate_rust_expr(input);
        assert!(
            result.is_ok(),
            "expected '{input}' to be accepted, got: {:?}",
            result.err()
        );
    }

    // ── Unhappy path: rejected statement-like forms ──────────────

    #[rstest]
    #[case::block_expression("{ let x = 1; x > 0 }", "not a statement or block")]
    #[case::for_loop("for i in 0..10 { }", "not a statement or block")]
    #[case::while_loop("while true { }", "not a statement or block")]
    #[case::infinite_loop("loop { break 42; }", "not a statement or block")]
    #[case::let_expression("let x = 5", "not a statement or block")]
    #[case::unsafe_block("unsafe { x }", "not a statement or block")]
    #[case::async_block("async { x }", "not a statement or block")]
    #[case::const_block("const { 42 }", "not a statement or block")]
    #[case::return_expr("return 42", "not a statement or block")]
    #[case::break_expr("break 42", "not a statement or block")]
    #[case::continue_expr("continue", "not a statement or block")]
    #[case::assignment("x = 5", "not a statement or block")]
    #[case::add_assign("x += 1", "not a statement or block")]
    #[case::sub_assign("x -= 2", "not a statement or block")]
    #[case::mul_assign("x *= 3", "not a statement or block")]
    fn given_statement_like_form_when_validated_then_rejected(
        #[case] input: &str,
        #[case] expected_fragment: &str,
    ) {
        let result = validate_rust_expr(input);
        assert!(result.is_err(), "expected '{input}' to be rejected");
        let reason = result.err().unwrap_or_default();
        assert!(
            reason.contains(expected_fragment),
            "reason for '{input}' should contain \
             '{expected_fragment}', got: {reason}"
        );
    }

    // ── Unhappy path: invalid syntax ─────────────────────────────

    #[rstest]
    #[case::garbage_syntax("not rust code %%")]
    #[case::incomplete_expr("x >")]
    #[case::empty_if("if { }")]
    fn given_invalid_syntax_when_validated_then_rejected_with_parse_error(#[case] input: &str) {
        let result = validate_rust_expr(input);
        assert!(result.is_err(), "expected '{input}' to be rejected");
        let reason = result.err().unwrap_or_default();
        assert!(
            reason.contains("is not a valid Rust expression"),
            "reason for '{input}' should mention parse failure, \
             got: {reason}"
        );
    }
}
