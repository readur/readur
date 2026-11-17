//! Unit tests for the boolean search parser and its `to_tsquery` renderer.
//! That Postgres accepts every rendering is pinned by the fuzz test in
//! tests/integration_boolean_search_tests.rs.

use super::{BooleanQuery, Node, MAX_GROUP_DEPTH};

fn ts(query: &str) -> String {
    BooleanQuery::parse(query).to_tsquery()
}

fn positive(query: &str) -> Vec<String> {
    BooleanQuery::parse(query).positive_terms()
}

fn assert_all(inputs: &[&str], expected: &str) {
    for input in inputs {
        assert_eq!(ts(input), expected, "input: {input:?}");
    }
}

fn term(text: &str) -> Node {
    Node::Term {
        text: text.to_string(),
        prefix: false,
    }
}

#[test]
fn single_word_is_a_quoted_lexeme() {
    assert_eq!(ts("invoice"), "'invoice'");
    assert_eq!(ts("  invoice  "), "'invoice'");
}

#[test]
fn and_by_symbol_keyword_or_adjacency() {
    assert_all(
        &[
            "invoice & paid",
            "invoice&paid",
            "invoice paid",
            "invoice AND paid",
            "invoice and paid",
            "invoice And paid",
            "invoice && paid",
        ],
        "('invoice' & 'paid')",
    );
    assert_eq!(ts("a b c"), "('a' & 'b' & 'c')");
}

#[test]
fn or_by_symbol_or_keyword_in_any_case() {
    assert_all(
        &[
            "invoice | receipt",
            "invoice|receipt",
            "invoice OR receipt",
            "invoice or receipt",
            "invoice || receipt",
        ],
        "('invoice' | 'receipt')",
    );
}

#[test]
fn not_by_bang_keyword_or_attached_minus() {
    assert_all(
        &[
            "invoice & !draft",
            "invoice&!draft",
            "invoice !draft",
            "invoice NOT draft",
            "invoice not draft",
            "invoice AND NOT draft",
            "invoice -draft",
            "invoice ! draft",
        ],
        "('invoice' & !'draft')",
    );
    assert_eq!(ts("-draft"), "!'draft'");
}

#[test]
fn a_detached_minus_is_ignored_and_an_inner_one_is_part_of_the_word() {
    assert_eq!(ts("invoice - draft"), "('invoice' & 'draft')");
    assert_eq!(ts("e-mail"), "'e-mail'");
    assert_eq!(ts("invoice-draft"), "'invoice-draft'");
    assert_eq!(ts("invoice -"), "'invoice'");
}

#[test]
fn and_binds_tighter_than_or() {
    assert_eq!(ts("a | b & c"), "('a' | ('b' & 'c'))");
    assert_eq!(ts("a & b | c"), "(('a' & 'b') | 'c')");
    assert_eq!(ts("a b | c d"), "(('a' & 'b') | ('c' & 'd'))");
}

#[test]
fn parentheses_group() {
    // Before this parser, parentheses were dropped and this matched every
    // invoice regardless of year.
    assert_eq!(
        ts("(invoice | receipt) & 2024"),
        "(('invoice' | 'receipt') & '2024')"
    );
    assert_eq!(
        ts("(invoice OR receipt) 2024"),
        "(('invoice' | 'receipt') & '2024')"
    );
    assert_eq!(ts("a & (b | (c & d))"), "('a' & ('b' | ('c' & 'd')))");
    assert_eq!(ts("((a))"), "'a'");
    assert_eq!(
        BooleanQuery::parse("(invoice | receipt) & 2024").root(),
        Some(&Node::And(vec![
            Node::Or(vec![term("invoice"), term("receipt")]),
            term("2024"),
        ]))
    );
}

#[test]
fn negated_groups() {
    assert_all(
        &[
            "NOT (draft | copy)",
            "!(draft | copy)",
            "-(draft | copy)",
            "not(draft|copy)",
        ],
        "!('draft' | 'copy')",
    );
    assert_eq!(
        ts("invoice NOT (draft | copy)"),
        "('invoice' & !('draft' | 'copy'))"
    );
}

#[test]
fn quoted_phrases_become_followed_by_chains() {
    assert_eq!(ts("\"total due\""), "('total' <-> 'due')");
    assert_eq!(
        ts("\"total   amount due\""),
        "('total' <-> 'amount' <-> 'due')"
    );
    assert_eq!(ts("\"total\""), "'total'");
    assert_eq!(ts("\"total , due\""), "('total' <-> 'due')");
    assert_eq!(
        ts("\"total due\" & invoice"),
        "(('total' <-> 'due') & 'invoice')"
    );
    assert_eq!(ts("\"unterminated phrase"), "('unterminated' <-> 'phrase')");
    // Operator characters inside quotes are words, not operators.
    assert_eq!(ts("\"a | b\""), "('a' <-> 'b')");
    assert_eq!(ts("\"(a)\""), "'(a)'");
}

#[test]
fn negated_phrases() {
    // A leading minus before a phrase used to be dropped, turning the
    // exclusion into a match.
    assert_all(
        &[
            "invoice -\"draft copy\"",
            "invoice !\"draft copy\"",
            "invoice NOT \"draft copy\"",
            "invoice & !\"draft copy\"",
        ],
        "('invoice' & !('draft' <-> 'copy'))",
    );
}

#[test]
fn prefix_matching() {
    assert_eq!(ts("inv:*"), "'inv':*");
    assert_eq!(ts("inv*"), "'inv':*");
    assert_eq!(ts("inv**"), "'inv':*");
    assert_eq!(ts("inv:* | rec*"), "('inv':* | 'rec':*)");
    assert_eq!(ts("!inv:*"), "!'inv':*");
    // A colon without `*` is just a character.
    assert_eq!(ts("inv:"), "'inv:'");
    assert_eq!(ts("10:30"), "'10:30'");
    // A bare marker has no word to match.
    assert_eq!(ts("*"), "");
    assert_eq!(ts(":*"), "");
    // Inside a phrase `*` is a literal character.
    assert_eq!(ts("\"inv* due\""), "('inv*' <-> 'due')");
}

#[test]
fn followed_by_operators() {
    assert_all(&["total <-> due", "total<->due"], "('total' <-> 'due')");
    assert_eq!(ts("a <3> b"), "('a' <3> 'b')");
    assert_eq!(ts("a <1> b"), "('a' <-> 'b')");
    assert_eq!(ts("a <0> b"), "('a' <0> 'b')");
    assert_eq!(ts("a <-> b <2> c"), "('a' <-> 'b' <2> 'c')");
    // Postgres caps distances at 16384.
    assert_eq!(ts("a <99999> b"), "('a' <16384> 'b')");
    assert_eq!(ts("a <99999999999999999999> b"), "('a' <16384> 'b')");
    // Followed-by binds tighter than AND, NOT tighter than followed-by.
    assert_eq!(ts("a <-> b & c"), "(('a' <-> 'b') & 'c')");
    assert_eq!(ts("!a <-> b"), "(!'a' <-> 'b')");
    assert_eq!(ts("(a | b) <-> c"), "(('a' | 'b') <-> 'c')");
}

#[test]
fn dangling_followed_by_degrades() {
    assert_eq!(ts("<-> a"), "'a'");
    assert_eq!(ts("a <->"), "'a'");
    assert_eq!(ts("a <-> <-> b"), "('a' <-> 'b')");
    assert_eq!(ts("<->"), "");
    // A `<` that starts no operator is ignored.
    assert_eq!(ts("a < b"), "('a' & 'b')");
    assert_eq!(ts("a <- b"), "('a' & 'b')");
    assert_eq!(ts("a <x> b"), "('a' & 'x>' & 'b')");
}

#[test]
fn negations_cancel_in_pairs() {
    assert_all(
        &["!!a", "NOT !a", "!(!a)", "--a", "NOT NOT a", "-!a"],
        "'a'",
    );
    assert_all(&["!!!a", "NOT NOT NOT a"], "!'a'");
}

#[test]
fn lexemes_are_escaped() {
    assert_eq!(ts("it's"), "'it''s'");
    assert_eq!(ts("o'brien:*"), "'o''brien':*");
    assert_eq!(ts("back\\slash"), "'back\\\\slash'");
    assert_eq!(ts("trail\\"), "'trail\\\\'");
    assert_eq!(ts("x\\'y"), "'x\\\\''y'");
    assert_eq!(ts("\"it's due\""), "('it''s' <-> 'due')");
    assert_eq!(ts("a:b:c"), "'a:b:c'");
    // Injection attempts are inert words.
    assert_eq!(
        ts("'; DROP TABLE documents; --"),
        "('DROP' & 'TABLE' & 'documents;')"
    );
    assert_eq!(ts("a' | 'b"), "('a''' | '''b')");
}

#[test]
fn unicode_words() {
    assert_eq!(ts("日本語 | тест"), "('日本語' | 'тест')");
    assert_eq!(ts("café -Straße"), "('café' & !'Straße')");
    assert_eq!(ts("\"naïve résumé\""), "('naïve' <-> 'résumé')");
    // Symbols and emoji alone are not words.
    assert_eq!(ts("🎉 invoice"), "'invoice'");
    assert_eq!(ts("— invoice ©"), "'invoice'");
}

#[test]
fn words_without_letters_or_digits_are_dropped() {
    assert_eq!(ts("invoice , . ; ''"), "'invoice'");
    assert_eq!(ts("... --- ***"), "");
    assert_eq!(ts("invoice & ..."), "'invoice'");
}

#[test]
fn malformed_input_degrades_instead_of_failing() {
    for input in [
        "",
        "   ",
        "&",
        "|",
        "!",
        "-",
        "((",
        "))",
        "()",
        "!()",
        "| | |",
        "!!",
        "- -",
        "\"\"",
        "\"",
        "NOT",
        "AND",
        "OR",
        "AND OR NOT",
        "& | !",
        "<-> & |",
        "(|)",
        "(&)",
    ] {
        assert_eq!(ts(input), "", "input: {input:?}");
    }
    assert_eq!(ts("a & | b"), "('a' | 'b')");
    assert_eq!(ts("a | | b"), "('a' | 'b')");
    assert_eq!(ts("| a"), "'a'");
    assert_eq!(ts("a |"), "'a'");
    assert_eq!(ts("a &"), "'a'");
    assert_eq!(ts("a !"), "'a'");
    assert_eq!(ts("a (b"), "('a' & 'b')");
    assert_eq!(ts("(a | b"), "('a' | 'b')");
    assert_eq!(ts("a) | b"), "('a' | 'b')");
    assert_eq!(ts("a | b))) & c"), "('a' | ('b' & 'c'))");
    assert_eq!(ts("a () b"), "('a' & 'b')");
    assert_eq!(ts("(a | ) & b"), "('a' & 'b')");
    assert_eq!(ts("((a | b) | c) &"), "(('a' | 'b') | 'c')");
}

#[test]
fn deep_nesting_is_bounded() {
    let deep = format!("{}a{}", "(".repeat(500), ")".repeat(500));
    assert_eq!(ts(&deep), "'a'");

    // 40 nested non-trivial groups: everything below MAX_GROUP_DEPTH is
    // flattened into its parent, so the rendering nests at most that deep.
    let mut nested = "z".to_string();
    for i in 0..40 {
        nested = format!("(w{i} | {nested})");
    }
    let rendered = ts(&nested);
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    for c in rendered.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        max_depth = max_depth.max(depth);
    }
    assert_eq!(depth, 0, "{rendered}");
    assert!(max_depth <= MAX_GROUP_DEPTH, "{max_depth}: {rendered}");
    for i in 0..40 {
        assert!(
            rendered.contains(&format!("'w{i}'")),
            "lost w{i}: {rendered}"
        );
    }
}

#[test]
fn long_operator_runs_do_not_overflow_the_stack() {
    assert_eq!(ts(&format!("{}a", "!".repeat(100_000))), "'a'");
    assert_eq!(ts(&format!("{}a", "!".repeat(100_001))), "!'a'");
    let chain = vec!["a"; 5_000].join(" <-> ");
    assert!(ts(&chain).starts_with("('a' <-> 'a'"));
    let ors = vec!["a"; 5_000].join(" | ");
    assert!(ts(&ors).starts_with("('a' | 'a'"));
}

#[test]
fn positive_terms_skip_negated_terms_and_operators() {
    assert_eq!(
        positive("invoice OR receipt NOT draft"),
        ["invoice", "receipt"]
    );
    assert_eq!(
        positive("invoice | receipt & !draft"),
        ["invoice", "receipt"]
    );
    assert_eq!(positive("\"total due\" -\"draft copy\""), ["total due"]);
    assert_eq!(positive("inv:* rec*"), ["inv", "rec"]);
    assert_eq!(positive("!(a | !b)"), ["b"]);
    assert_eq!(positive("NOT (draft | copy) invoice"), ["invoice"]);
    assert_eq!(positive("a <-> b"), ["a", "b"]);
    assert_eq!(positive("invoice, invoice INVOICE"), ["invoice", "INVOICE"]);
    assert_eq!(positive("\"(total) due,\""), ["total) due"]);
    assert!(positive("").is_empty());
    assert!(positive("NOT draft").is_empty());
    assert!(positive("& | ! ( )").is_empty());
}
