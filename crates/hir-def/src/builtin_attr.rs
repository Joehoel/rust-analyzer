//! Builtin attributes resolved by nameres.
//!
//! The actual definitions were copied from rustc's `compiler/rustc_feature/src/builtin_attrs.rs`.
//!
//! It was last synchronized with upstream commit ae90dcf0207c57c3034f00b07048d63f8b2363c8.
//!
//! The macros were adjusted to only expand to the attribute name, since that is all we need to do
//! name resolution, and `BUILTIN_ATTRIBUTES` is almost entirely unchanged from the original, to
//! ease updating.

use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;

/// Ignored attribute namespaces used by tools.
pub const TOOL_MODULES: &[&str] = &["rustfmt", "clippy"];

pub struct BuiltinAttribute {
    pub name: &'static str,
    pub template: AttributeTemplate,
}

/// A template that the attribute input must match.
/// Only top-level shape (`#[attr]` vs `#[attr(...)]` vs `#[attr = ...]`) is considered now.
#[derive(Clone, Copy)]
pub struct AttributeTemplate {
    pub word: bool,
    pub list: Option<&'static str>,
    pub name_value_str: Option<&'static str>,
}

pub fn find_builtin_attr_idx(name: &str) -> Option<usize> {
    static BUILTIN_LOOKUP_TABLE: OnceCell<FxHashMap<&'static str, usize>> = OnceCell::new();
    BUILTIN_LOOKUP_TABLE
        .get_or_init(|| {
            INERT_ATTRIBUTES.iter().map(|attr| attr.name).enumerate().map(|(a, b)| (b, a)).collect()
        })
        .get(name)
        .copied()
}

// impl AttributeTemplate {
//     const DEFAULT: AttributeTemplate =
//         AttributeTemplate { word: false, list: None, name_value_str: None };
// }

/// A convenience macro for constructing attribute templates.
/// E.g., `template!(Word, List: "description")` means that the attribute
/// supports forms `#[attr]` and `#[attr(description)]`.
macro_rules! template {
    (Word) => { template!(@ true, None, None) };
    (List: $descr: expr) => { template!(@ false, Some($descr), None) };
    (NameValueStr: $descr: expr) => { template!(@ false, None, Some($descr)) };
    (Word, List: $descr: expr) => { template!(@ true, Some($descr), None) };
    (Word, NameValueStr: $descr: expr) => { template!(@ true, None, Some($descr)) };
    (List: $descr1: expr, NameValueStr: $descr2: expr) => {
        template!(@ false, Some($descr1), Some($descr2))
    };
    (Word, List: $descr1: expr, NameValueStr: $descr2: expr) => {
        template!(@ true, Some($descr1), Some($descr2))
    };
    (@ $word: expr, $list: expr, $name_value_str: expr) => {
        AttributeTemplate {
            word: $word, list: $list, name_value_str: $name_value_str
        }
    };
}

macro_rules! ungated {
    ($attr:ident, $typ:expr, $tpl:expr $(,)?) => {
        BuiltinAttribute { name: stringify!($attr), template: $tpl }
    };
}

macro_rules! gated {
    ($attr:ident, $typ:expr, $tpl:expr, $gate:ident, $msg:expr $(,)?) => {
        BuiltinAttribute { name: stringify!($attr), template: $tpl }
    };
    ($attr:ident, $typ:expr, $tpl:expr, $msg:expr $(,)?) => {
        BuiltinAttribute { name: stringify!($attr), template: $tpl }
    };
}

macro_rules! rustc_attr {
    (TEST, $attr:ident, $typ:expr, $tpl:expr $(,)?) => {
        rustc_attr!(
            $attr,
            $typ,
            $tpl,
            concat!(
                "the `#[",
                stringify!($attr),
                "]` attribute is just used for rustc unit tests \
                and will never be stable",
            ),
        )
    };
    ($attr:ident, $typ:expr, $tpl:expr, $msg:expr $(,)?) => {
        BuiltinAttribute { name: stringify!($attr), template: $tpl }
    };
}

/// "Inert" built-in attributes that have a special meaning to rustc or rustdoc.
#[rustfmt::skip]
pub const INERT_ATTRIBUTES: &[BuiltinAttribute] = &[
    // ==========================================================================
    // Stable attributes:
    // ==========================================================================

    // Conditional compilation:
    ungated!(cfg, Normal, template!(List: "predicate")),
    ungated!(cfg_attr, Normal, template!(List: "predicate, attr1, attr2, ...")),

    // Testing:
    ungated!(ignore, Normal, template!(Word, NameValueStr: "reason")),
    ungated!(
        should_panic, Normal,
        template!(Word, List: r#"expected = "reason"#, NameValueStr: "reason"),
    ),
    // FIXME(Centril): This can be used on stable but shouldn't.
    ungated!(reexport_test_harness_main, Normal, template!(NameValueStr: "name")),

    // Macros:
    ungated!(automatically_derived, Normal, template!(Word)),
    // FIXME(#14407)
    ungated!(macro_use, Normal, template!(Word, List: "name1, name2, ...")),
    ungated!(macro_escape, Normal, template!(Word)), // Deprecated synonym for `macro_use`.
    ungated!(macro_export, Normal, template!(Word, List: "local_inner_macros")),
    ungated!(proc_macro, Normal, template!(Word)),
    ungated!(
        proc_macro_derive, Normal,
        template!(List: "TraitName, /*opt*/ attributes(name1, name2, ...)"),
    ),
    ungated!(proc_macro_attribute, Normal, template!(Word)),

    // Lints:
    ungated!(warn, Normal, template!(List: r#"lint1, lint2, ..., /*opt*/ reason = "...""#)),
    ungated!(allow, Normal, template!(List: r#"lint1, lint2, ..., /*opt*/ reason = "...""#)),
    ungated!(forbid, Normal, template!(List: r#"lint1, lint2, ..., /*opt*/ reason = "...""#)),
    ungated!(deny, Normal, template!(List: r#"lint1, lint2, ..., /*opt*/ reason = "...""#)),
    ungated!(must_use, AssumedUsed, template!(Word, NameValueStr: "reason")),
    // FIXME(#14407)
    ungated!(
        deprecated, Normal,
        template!(
            Word,
            List: r#"/*opt*/ since = "version", /*opt*/ note = "reason""#,
            NameValueStr: "reason"
        ),
    ),

    // Crate properties:
    ungated!(crate_name, CrateLevel, template!(NameValueStr: "name")),
    ungated!(crate_type, CrateLevel, template!(NameValueStr: "bin|lib|...")),
    ungated!(crate_id, CrateLevel, template!(NameValueStr: "ignored")),

    // ABI, linking, symbols, and FFI
    ungated!(
        link, AssumedUsed,
        template!(List: r#"name = "...", /*opt*/ kind = "dylib|static|...", /*opt*/ wasm_import_module = "...""#),
    ),
    ungated!(link_name, AssumedUsed, template!(NameValueStr: "name")),
    ungated!(no_link, AssumedUsed, template!(Word)),
    ungated!(repr, AssumedUsed, template!(List: "C")),
    ungated!(export_name, AssumedUsed, template!(NameValueStr: "name")),
    ungated!(link_section, AssumedUsed, template!(NameValueStr: "name")),
    ungated!(no_mangle, AssumedUsed, template!(Word)),
    ungated!(used, AssumedUsed, template!(Word)),

    // Limits:
    ungated!(recursion_limit, CrateLevel, template!(NameValueStr: "N")),
    ungated!(type_length_limit, CrateLevel, template!(NameValueStr: "N")),
    gated!(
        const_eval_limit, CrateLevel, template!(NameValueStr: "N"), const_eval_limit,
        experimental!(const_eval_limit)
    ),
    gated!(
        move_size_limit, CrateLevel, template!(NameValueStr: "N"), large_assignments,
        experimental!(move_size_limit)
    ),

    // Entry point:
    ungated!(main, Normal, template!(Word)),
    ungated!(start, Normal, template!(Word)),
    ungated!(no_start, CrateLevel, template!(Word)),
    ungated!(no_main, CrateLevel, template!(Word)),

    // Modules, prelude, and resolution:
    ungated!(path, Normal, template!(NameValueStr: "file")),
    ungated!(no_std, CrateLevel, template!(Word)),
    ungated!(no_implicit_prelude, Normal, template!(Word)),
    ungated!(non_exhaustive, AssumedUsed, template!(Word)),

    // Runtime
    ungated!(windows_subsystem, AssumedUsed, template!(NameValueStr: "windows|console")),
    ungated!(panic_handler, Normal, template!(Word)), // RFC 2070

    // Code generation:
    ungated!(inline, AssumedUsed, template!(Word, List: "always|never")),
    ungated!(cold, AssumedUsed, template!(Word)),
    ungated!(no_builtins, AssumedUsed, template!(Word)),
    ungated!(target_feature, AssumedUsed, template!(List: r#"enable = "name""#)),
    ungated!(track_caller, AssumedUsed, template!(Word)),
    gated!(
        no_sanitize, AssumedUsed,
        template!(List: "address, memory, thread"),
        experimental!(no_sanitize)
    ),
    gated!(no_coverage, AssumedUsed, template!(Word), experimental!(no_coverage)),

    // FIXME: #14408 assume docs are used since rustdoc looks at them.
    ungated!(doc, AssumedUsed, template!(List: "hidden|inline|...", NameValueStr: "string")),

    // ==========================================================================
    // Unstable attributes:
    // ==========================================================================

    // Linking:
    gated!(naked, AssumedUsed, template!(Word), naked_functions, experimental!(naked)),
    gated!(
        link_ordinal, AssumedUsed, template!(List: "ordinal"), raw_dylib,
        experimental!(link_ordinal)
    ),

    // Plugins:
    // XXX Modified for use in rust-analyzer
    gated!(plugin_registrar, Normal, template!(Word), experimental!()),
    gated!(plugin, CrateLevel, template!(Word), experimental!()),

    // Testing:
    gated!(allow_fail, Normal, template!(Word), experimental!(allow_fail)),
    gated!(
        test_runner, CrateLevel, template!(List: "path"), custom_test_frameworks,
        "custom test frameworks are an unstable feature",
    ),
    // RFC #1268
    gated!(marker, AssumedUsed, template!(Word), marker_trait_attr, experimental!(marker)),
    gated!(
        thread_local, AssumedUsed, template!(Word),
        "`#[thread_local]` is an experimental feature, and does not currently handle destructors",
    ),
    gated!(no_core, CrateLevel, template!(Word), experimental!(no_core)),
    // RFC 2412
    gated!(
        optimize, AssumedUsed, template!(List: "size|speed"), optimize_attribute,
        experimental!(optimize),
    ),
    // RFC 2867
    gated!(instruction_set, AssumedUsed, template!(List: "set"), isa_attribute, experimental!(instruction_set)),

    gated!(ffi_returns_twice, AssumedUsed, template!(Word), experimental!(ffi_returns_twice)),
    gated!(ffi_pure, AssumedUsed, template!(Word), experimental!(ffi_pure)),
    gated!(ffi_const, AssumedUsed, template!(Word), experimental!(ffi_const)),
    gated!(
        register_attr, CrateLevel, template!(List: "attr1, attr2, ..."),
        experimental!(register_attr),
    ),
    gated!(
        register_tool, CrateLevel, template!(List: "tool1, tool2, ..."),
        experimental!(register_tool),
    ),

    gated!(cmse_nonsecure_entry, AssumedUsed, template!(Word), experimental!(cmse_nonsecure_entry)),
    // RFC 2632
    gated!(
        default_method_body_is_const, AssumedUsed, template!(Word), const_trait_impl,
        "`default_method_body_is_const` is a temporary placeholder for declaring default bodies \
        as `const`, which may be removed or renamed in the future."
    ),

    // ==========================================================================
    // Internal attributes: Stability, deprecation, and unsafe:
    // ==========================================================================

    ungated!(feature, CrateLevel, template!(List: "name1, name1, ...")),
    // FIXME(#14407) -- only looked at on-demand so we can't
    // guarantee they'll have already been checked.
    // FIXME(jhpratt) remove this when #[rustc_deprecated] is fully removed from the compiler
    ungated!(
        rustc_deprecated, AssumedUsed,
        template!(List: r#"since = "version", reason = "...""#)
    ),
    // FIXME(#14407)
    ungated!(stable, AssumedUsed, template!(List: r#"feature = "name", since = "version""#)),
    // FIXME(#14407)
    ungated!(
        unstable, AssumedUsed,
        template!(List: r#"feature = "name", reason = "...", issue = "N""#),
    ),
    // FIXME(#14407)
    ungated!(rustc_const_unstable, AssumedUsed, template!(List: r#"feature = "name""#)),
    // FIXME(#14407)
    ungated!(rustc_const_stable, AssumedUsed, template!(List: r#"feature = "name""#)),
    gated!(
        allow_internal_unstable, AssumedUsed, template!(Word, List: "feat1, feat2, ..."),
        "allow_internal_unstable side-steps feature gating and stability checks",
    ),
    gated!(
        rustc_allow_const_fn_unstable, AssumedUsed, template!(Word, List: "feat1, feat2, ..."),
        "rustc_allow_const_fn_unstable side-steps feature gating and stability checks"
    ),
    gated!(
        allow_internal_unsafe, Normal, template!(Word),
        "allow_internal_unsafe side-steps the unsafe_code lint",
    ),

    // ==========================================================================
    // Internal attributes: Type system related:
    // ==========================================================================

    gated!(fundamental, AssumedUsed, template!(Word), experimental!(fundamental)),
    gated!(
        may_dangle, Normal, template!(Word), dropck_eyepatch,
        "`may_dangle` has unstable semantics and may be removed in the future",
    ),

    // ==========================================================================
    // Internal attributes: Runtime related:
    // ==========================================================================

    rustc_attr!(rustc_allocator, AssumedUsed, template!(Word), IMPL_DETAIL),
    rustc_attr!(rustc_allocator_nounwind, AssumedUsed, template!(Word), IMPL_DETAIL),
    gated!(alloc_error_handler, Normal, template!(Word), experimental!(alloc_error_handler)),
    gated!(
        default_lib_allocator, AssumedUsed, template!(Word), allocator_internals,
        experimental!(default_lib_allocator),
    ),
    gated!(
        needs_allocator, Normal, template!(Word), allocator_internals,
        experimental!(needs_allocator),
    ),
    gated!(panic_runtime, AssumedUsed, template!(Word), experimental!(panic_runtime)),
    gated!(needs_panic_runtime, AssumedUsed, template!(Word), experimental!(needs_panic_runtime)),
    gated!(
        compiler_builtins, AssumedUsed, template!(Word),
        "the `#[compiler_builtins]` attribute is used to identify the `compiler_builtins` crate \
        which contains compiler-rt intrinsics and will never be stable",
    ),
    gated!(
        profiler_runtime, AssumedUsed, template!(Word),
        "the `#[profiler_runtime]` attribute is used to identify the `profiler_builtins` crate \
        which contains the profiler runtime and will never be stable",
    ),

    // ==========================================================================
    // Internal attributes, Linkage:
    // ==========================================================================

    gated!(
        linkage, AssumedUsed, template!(NameValueStr: "external|internal|..."),
        "the `linkage` attribute is experimental and not portable across platforms",
    ),
    rustc_attr!(rustc_std_internal_symbol, AssumedUsed, template!(Word), INTERNAL_UNSTABLE),

    // ==========================================================================
    // Internal attributes, Macro related:
    // ==========================================================================

    rustc_attr!(
        rustc_builtin_macro, AssumedUsed,
        template!(Word, List: "name, /*opt*/ attributes(name1, name2, ...)"),
        IMPL_DETAIL,
    ),
    rustc_attr!(rustc_proc_macro_decls, Normal, template!(Word), INTERNAL_UNSTABLE),
    rustc_attr!(
        rustc_macro_transparency, AssumedUsed,
        template!(NameValueStr: "transparent|semitransparent|opaque"),
        "used internally for testing macro hygiene",
    ),

    // ==========================================================================
    // Internal attributes, Diagnostics related:
    // ==========================================================================

    rustc_attr!(
        rustc_on_unimplemented, AssumedUsed,
        template!(
            List: r#"/*opt*/ message = "...", /*opt*/ label = "...", /*opt*/ note = "...""#,
            NameValueStr: "message"
        ),
        INTERNAL_UNSTABLE
    ),
    // Enumerates "identity-like" conversion methods to suggest on type mismatch.
    rustc_attr!(rustc_conversion_suggestion, AssumedUsed, template!(Word), INTERNAL_UNSTABLE),

    // ==========================================================================
    // Internal attributes, Const related:
    // ==========================================================================

    rustc_attr!(rustc_promotable, AssumedUsed, template!(Word), IMPL_DETAIL),
    rustc_attr!(rustc_legacy_const_generics, AssumedUsed, template!(List: "N"), INTERNAL_UNSTABLE),

    // ==========================================================================
    // Internal attributes, Layout related:
    // ==========================================================================

    rustc_attr!(
        rustc_layout_scalar_valid_range_start, AssumedUsed, template!(List: "value"),
        "the `#[rustc_layout_scalar_valid_range_start]` attribute is just used to enable \
        niche optimizations in libcore and will never be stable",
    ),
    rustc_attr!(
        rustc_layout_scalar_valid_range_end, AssumedUsed, template!(List: "value"),
        "the `#[rustc_layout_scalar_valid_range_end]` attribute is just used to enable \
        niche optimizations in libcore and will never be stable",
    ),
    rustc_attr!(
        rustc_nonnull_optimization_guaranteed, AssumedUsed, template!(Word),
        "the `#[rustc_nonnull_optimization_guaranteed]` attribute is just used to enable \
        niche optimizations in libcore and will never be stable",
    ),

    // ==========================================================================
    // Internal attributes, Misc:
    // ==========================================================================
    gated!(
        lang, Normal, template!(NameValueStr: "name"), lang_items,
        "language items are subject to change",
    ),
    gated!(rustc_diagnostic_item, Normal, template!(NameValueStr: "name"), experimental!()), // XXX Modified for use in rust-analyzer
    gated!(
        // Used in resolve:
        prelude_import, AssumedUsed, template!(Word),
        "`#[prelude_import]` is for use by rustc only",
    ),
    gated!(
        rustc_paren_sugar, Normal, template!(Word), unboxed_closures,
        "unboxed_closures are still evolving",
    ),
    rustc_attr!(
        rustc_inherit_overflow_checks, AssumedUsed, template!(Word),
        "the `#[rustc_inherit_overflow_checks]` attribute is just used to control \
        overflow checking behavior of several libcore functions that are inlined \
        across crates and will never be stable",
    ),
    rustc_attr!(rustc_reservation_impl, Normal, template!(NameValueStr: "reservation message"),
                "the `#[rustc_reservation_impl]` attribute is internally used \
                 for reserving for `for<T> From<!> for T` impl"
    ),
    rustc_attr!(
        rustc_test_marker, Normal, template!(Word),
        "the `#[rustc_test_marker]` attribute is used internally to track tests",
    ),
    rustc_attr!(
        rustc_unsafe_specialization_marker, Normal, template!(Word),
        "the `#[rustc_unsafe_specialization_marker]` attribute is used to check specializations"
    ),
    rustc_attr!(
        rustc_specialization_trait, Normal, template!(Word),
        "the `#[rustc_specialization_trait]` attribute is used to check specializations"
    ),
    rustc_attr!(
        rustc_main, Normal, template!(Word),
        "the `#[rustc_main]` attribute is used internally to specify test entry point function",
    ),
    rustc_attr!(
        rustc_skip_array_during_method_dispatch, Normal, template!(Word),
        "the `#[rustc_skip_array_during_method_dispatch]` attribute is used to exclude a trait \
        from method dispatch when the receiver is an array, for compatibility in editions < 2021."
    ),

    // ==========================================================================
    // Internal attributes, Testing:
    // ==========================================================================

    rustc_attr!(TEST, rustc_outlives, Normal, template!(Word)),
    rustc_attr!(TEST, rustc_capture_analysis, Normal, template!(Word)),
    rustc_attr!(TEST, rustc_insignificant_dtor, Normal, template!(Word)),
    rustc_attr!(TEST, rustc_variance, Normal, template!(Word)),
    rustc_attr!(TEST, rustc_layout, Normal, template!(List: "field1, field2, ...")),
    rustc_attr!(TEST, rustc_regions, Normal, template!(Word)),
    rustc_attr!(
        TEST, rustc_error, AssumedUsed,
        template!(Word, List: "delay_span_bug_from_inside_query")
    ),
    rustc_attr!(TEST, rustc_dump_user_substs, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_evaluate_where_clauses, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_if_this_changed, AssumedUsed, template!(Word, List: "DepNode")),
    rustc_attr!(TEST, rustc_then_this_would_need, AssumedUsed, template!(List: "DepNode")),
    rustc_attr!(
        TEST, rustc_clean, AssumedUsed,
        template!(List: r#"cfg = "...", /*opt*/ label = "...", /*opt*/ except = "...""#),
    ),
    rustc_attr!(
        TEST, rustc_partition_reused, AssumedUsed,
        template!(List: r#"cfg = "...", module = "...""#),
    ),
    rustc_attr!(
        TEST, rustc_partition_codegened, AssumedUsed,
        template!(List: r#"cfg = "...", module = "...""#),
    ),
    rustc_attr!(
        TEST, rustc_expected_cgu_reuse, AssumedUsed,
        template!(List: r#"cfg = "...", module = "...", kind = "...""#),
    ),
    rustc_attr!(TEST, rustc_synthetic, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_symbol_name, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_polymorphize_error, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_def_path, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_mir, AssumedUsed, template!(List: "arg1, arg2, ...")),
    rustc_attr!(TEST, rustc_dump_program_clauses, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_dump_env_program_clauses, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_object_lifetime_default, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_dump_vtable, AssumedUsed, template!(Word)),
    rustc_attr!(TEST, rustc_dummy, Normal, template!(Word /* doesn't matter*/)),
    gated!(
        omit_gdb_pretty_printer_section, AssumedUsed, template!(Word),
        "the `#[omit_gdb_pretty_printer_section]` attribute is just used for the Rust test suite",
    ),
];
