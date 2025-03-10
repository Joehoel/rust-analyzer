//! Completes keywords, except:
//! - `self`, `super` and `crate`, as these are considered part of path completions.
//! - `await`, as this is a postfix completion we handle this in the postfix completions.

use syntax::T;

use crate::{
    context::{NameRefContext, PathKind},
    CompletionContext, CompletionItem, CompletionItemKind, Completions,
};

pub(crate) fn complete_expr_keyword(acc: &mut Completions, ctx: &CompletionContext) {
    if matches!(ctx.nameref_ctx(), Some(NameRefContext { record_expr: Some(_), .. })) {
        cov_mark::hit!(no_keyword_completion_in_record_lit);
        return;
    }
    if ctx.is_non_trivial_path() {
        cov_mark::hit!(no_keyword_completion_in_non_trivial_path);
        return;
    }
    if ctx.pattern_ctx.is_some() {
        return;
    }

    let mut add_keyword = |kw, snippet| add_keyword(acc, ctx, kw, snippet);

    let expects_assoc_item = ctx.expects_assoc_item();
    let has_block_expr_parent = ctx.has_block_expr_parent();
    let expects_item = ctx.expects_item();

    if let Some(PathKind::Vis { .. }) = ctx.path_kind() {
        return;
    }
    if ctx.has_unfinished_impl_or_trait_prev_sibling() {
        add_keyword("where", "where");
        if ctx.has_impl_prev_sibling() {
            add_keyword("for", "for");
        }
        return;
    }
    if ctx.previous_token_is(T![unsafe]) {
        if expects_item || expects_assoc_item || has_block_expr_parent {
            add_keyword("fn", "fn $1($2) {\n    $0\n}")
        }

        if expects_item || has_block_expr_parent {
            add_keyword("trait", "trait $1 {\n    $0\n}");
            add_keyword("impl", "impl $1 {\n    $0\n}");
        }

        return;
    }

    if !ctx.has_visibility_prev_sibling()
        && (expects_item || ctx.expects_non_trait_assoc_item() || ctx.expect_field())
    {
        add_keyword("pub(crate)", "pub(crate)");
        add_keyword("pub(super)", "pub(super)");
        add_keyword("pub", "pub");
    }

    if expects_item || expects_assoc_item || has_block_expr_parent {
        add_keyword("unsafe", "unsafe");
        add_keyword("fn", "fn $1($2) {\n    $0\n}");
        add_keyword("const", "const $0");
        add_keyword("type", "type $0");
    }

    if expects_item || has_block_expr_parent {
        if !ctx.has_visibility_prev_sibling() {
            add_keyword("impl", "impl $1 {\n    $0\n}");
            add_keyword("extern", "extern $0");
        }
        add_keyword("use", "use $0");
        add_keyword("trait", "trait $1 {\n    $0\n}");
        add_keyword("static", "static $0");
        add_keyword("mod", "mod $0");
    }

    if expects_item || has_block_expr_parent {
        add_keyword("enum", "enum $1 {\n    $0\n}");
        add_keyword("struct", "struct $0");
        add_keyword("union", "union $1 {\n    $0\n}");
    }
}

pub(super) fn add_keyword(acc: &mut Completions, ctx: &CompletionContext, kw: &str, snippet: &str) {
    let mut item = CompletionItem::new(CompletionItemKind::Keyword, ctx.source_range(), kw);

    match ctx.config.snippet_cap {
        Some(cap) => {
            if snippet.ends_with('}') && ctx.incomplete_let {
                // complete block expression snippets with a trailing semicolon, if inside an incomplete let
                cov_mark::hit!(let_semi);
                item.insert_snippet(cap, format!("{};", snippet));
            } else {
                item.insert_snippet(cap, snippet);
            }
        }
        None => {
            item.insert_text(if snippet.contains('$') { kw } else { snippet });
        }
    };
    item.add_to(acc);
}

#[cfg(test)]
mod tests {
    use expect_test::{expect, Expect};

    use crate::tests::{check_edit, completion_list};

    fn check(ra_fixture: &str, expect: Expect) {
        let actual = completion_list(ra_fixture);
        expect.assert_eq(&actual)
    }

    #[test]
    fn test_else_edit_after_if() {
        check_edit(
            "else",
            r#"fn quux() { if true { () } $0 }"#,
            r#"fn quux() { if true { () } else {
    $0
} }"#,
        );
    }

    #[test]
    fn test_keywords_after_unsafe_in_block_expr() {
        check(
            r"fn my_fn() { unsafe $0 }",
            expect![[r#"
                kw fn
                kw impl
                kw trait
                sn pd
                sn ppd
            "#]],
        );
    }

    #[test]
    fn test_completion_await_impls_future() {
        check(
            r#"
//- minicore: future
use core::future::*;
struct A {}
impl Future for A {}
fn foo(a: A) { a.$0 }
"#,
            expect![[r#"
                kw await expr.await
                sn box   Box::new(expr)
                sn call  function(expr)
                sn dbg   dbg!(expr)
                sn dbgr  dbg!(&expr)
                sn let   let
                sn letm  let mut
                sn match match expr {}
                sn ref   &expr
                sn refm  &mut expr
            "#]],
        );

        check(
            r#"
//- minicore: future
use std::future::*;
fn foo() {
    let a = async {};
    a.$0
}
"#,
            expect![[r#"
                kw await expr.await
                sn box   Box::new(expr)
                sn call  function(expr)
                sn dbg   dbg!(expr)
                sn dbgr  dbg!(&expr)
                sn let   let
                sn letm  let mut
                sn match match expr {}
                sn ref   &expr
                sn refm  &mut expr
            "#]],
        )
    }

    #[test]
    fn let_semi() {
        cov_mark::check!(let_semi);
        check_edit(
            "match",
            r#"
fn main() { let x = $0 }
"#,
            r#"
fn main() { let x = match $1 {
    $0
}; }
"#,
        );

        check_edit(
            "if",
            r#"
fn main() {
    let x = $0
    let y = 92;
}
"#,
            r#"
fn main() {
    let x = if $1 {
    $0
};
    let y = 92;
}
"#,
        );

        check_edit(
            "loop",
            r#"
fn main() {
    let x = $0
    bar();
}
"#,
            r#"
fn main() {
    let x = loop {
    $0
};
    bar();
}
"#,
        );
    }
}
