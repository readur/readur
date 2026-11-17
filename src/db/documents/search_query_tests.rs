//! Unit tests for the boolean search parser and its `to_tsquery` renderer.
//! That Postgres accepts every rendering is pinned by the fuzz test in
//! tests/integration_boolean_search_tests.rs.

use super::{
    document_match, single_field_match, BooleanQuery, Node, SqlPart, MAX_GROUP_DEPTH,
    PHRASETO_TSQUERY, PLAINTO_TSQUERY, TO_TSQUERY,
};

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

// ------------------------------------------------------------------- SQL

/// The SQL of `parts` with each bound value shown as `$<value>`, and the
/// bound values in order.
fn sql_shape(parts: &[SqlPart]) -> (String, Vec<String>) {
    let mut sql = String::new();
    let mut binds = Vec::new();
    for part in parts {
        match part {
            SqlPart::Sql(text) => sql.push_str(text),
            SqlPart::Bind(value) => {
                sql.push_str("$<");
                sql.push_str(value);
                sql.push('>');
                binds.push(value.clone());
            }
        }
    }
    (sql, binds)
}

/// `query` as SQL, with lexeme counts from `lexemes` (one per leaf).
fn sql(query: &str, lexemes: &[i32]) -> Option<String> {
    let parsed = BooleanQuery::parse(query);
    assert_eq!(parsed.leaves().len(), lexemes.len(), "leaves of {query:?}");
    parsed.to_sql(lexemes).map(|parts| sql_shape(&parts).0)
}

const DOC: &str = "to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''))";
const CONTENT: &str = "to_tsvector('english', COALESCE(content, ''))";
const OCR: &str = "to_tsvector('english', COALESCE(ocr_text, ''))";

fn doc_match(tsquery: &str) -> String {
    format!("({DOC} @@ to_tsquery('english', $<{tsquery}>))")
}

fn field_match(tsquery: &str, prefilter: Option<&str>) -> String {
    let fields = format!(
        "(({CONTENT} @@ to_tsquery('english', $<{tsquery}>)) OR \
         ({OCR} @@ to_tsquery('english', $<{tsquery}>)))"
    );
    match prefilter {
        Some(words) => format!("({} AND {fields})", doc_match(words)),
        None => format!("({fields})"),
    }
}

#[test]
fn leaves_are_words_phrases_and_followed_by_chains() {
    let parsed = BooleanQuery::parse("(a | \"b c\") -d e <-> f inv*");
    assert_eq!(
        parsed.leaves(),
        ["'a'", "('b' <-> 'c')", "'d'", "('e' <-> 'f')", "'inv':*"]
    );
    assert!(BooleanQuery::parse("").leaves().is_empty());
    assert!(BooleanQuery::parse("!! ()").leaves().is_empty());
}

#[test]
fn words_match_the_whole_document() {
    assert_eq!(sql("invoice", &[1]).unwrap(), doc_match("'invoice'"));
    assert_eq!(sql("inv*", &[1]).unwrap(), doc_match("'inv':*"));
    assert_eq!(
        sql("invoice -draft", &[1, 1]).unwrap(),
        format!(
            "({} AND (NOT {}))",
            doc_match("'invoice'"),
            doc_match("'draft'")
        )
    );
    assert_eq!(
        sql("(a | b) c", &[1, 1, 1]).unwrap(),
        format!(
            "(({} OR {}) AND {})",
            doc_match("'a'"),
            doc_match("'b'"),
            doc_match("'c'")
        )
    );
}

#[test]
fn phrases_and_followed_by_match_within_one_field() {
    // Prefiltered on the whole document by the same words, unordered.
    assert_eq!(
        sql("\"total due\"", &[3]).unwrap(),
        field_match("('total' <-> 'due')", Some("('total' & 'due')"))
    );
    assert_eq!(
        sql("total <2> due", &[3]).unwrap(),
        field_match("('total' <2> 'due')", Some("('total' & 'due')"))
    );
    assert_eq!(
        sql("(a | b) <-> c", &[5]).unwrap(),
        field_match("(('a' | 'b') <-> 'c')", Some("(('a' | 'b') & 'c')"))
    );
    // A word Postgres splits into several lexemes is matched like a phrase.
    assert_eq!(
        sql("e-mail", &[5]).unwrap(),
        field_match("'e-mail'", Some("'e-mail'"))
    );
    // A negation inside a chain implies no words, so no prefilter.
    assert_eq!(
        sql("!draft <-> copy", &[4]).unwrap(),
        field_match("(!'draft' <-> 'copy')", None)
    );
    // Mixed: the phrase per field, the negated word document-wide.
    assert_eq!(
        sql("\"total due\" -draft", &[3, 1]).unwrap(),
        format!(
            "({} AND (NOT {}))",
            field_match("('total' <-> 'due')", Some("('total' & 'due')")),
            doc_match("'draft'")
        )
    );
}

#[test]
fn a_phrase_reduced_to_one_lexeme_matches_the_whole_document() {
    assert_eq!(
        sql("\"the invoice\"", &[1]).unwrap(),
        doc_match("('the' <-> 'invoice')")
    );
}

#[test]
fn stop_word_leaves_are_dropped_like_to_tsquery_drops_them() {
    assert_eq!(
        sql("report of sales", &[1, 0, 1]).unwrap(),
        format!("({} AND {})", doc_match("'report'"), doc_match("'sales'"))
    );
    assert_eq!(
        sql("invoice | the", &[1, 0]).unwrap(),
        doc_match("'invoice'")
    );
    assert_eq!(
        sql("invoice -the", &[1, 0]).unwrap(),
        doc_match("'invoice'")
    );
    // Nothing left: the query matches nothing (not, as `NOT` of nothing
    // might suggest, everything).
    assert_eq!(sql("the", &[0]), None);
    assert_eq!(sql("-the", &[0]), None);
    assert_eq!(sql("!(the | a)", &[0, 0]), None);
    assert_eq!(sql("", &[]), None);
    // A missing count counts as no lexeme rather than panicking.
    let short = BooleanQuery::parse("a b").to_sql(&[1]).unwrap();
    assert_eq!(sql_shape(&short).0, doc_match("'a'"));
}

#[test]
fn user_text_is_only_ever_bound() {
    let hostile = "x'); DROP TABLE documents; -- \\ ' \" <-> $1 ?";
    for input in [hostile.to_string(), format!("\"{hostile}\" -{hostile}")] {
        let parsed = BooleanQuery::parse(&input);
        let lexemes = vec![3; parsed.leaves().len()];
        let parts = parsed.to_sql(&lexemes).unwrap();
        let (_, binds) = sql_shape(&parts);
        assert!(!binds.is_empty());
        for part in &parts {
            if let SqlPart::Sql(text) = part {
                assert!(!text.contains("DROP"), "{text:?} in {input:?}");
                assert!(!text.contains('$'), "{text:?} in {input:?}");
            }
        }
        // The words travel as rendered tsquery text, quotes escaped.
        assert!(binds.iter().any(|b| b.contains("x''")), "{binds:?}");
        assert!(binds.iter().any(|b| b.contains("'DROP'")), "{binds:?}");
    }
}

#[test]
fn phrase_and_simple_mode_conditions() {
    let (simple, binds) = sql_shape(&document_match(PLAINTO_TSQUERY, "total due"));
    assert_eq!(
        simple,
        format!("({DOC} @@ plainto_tsquery('english', $<total due>))")
    );
    assert_eq!(binds, ["total due"]);
    let (phrase, binds) = sql_shape(&single_field_match(
        PHRASETO_TSQUERY,
        "total due",
        Some((PLAINTO_TSQUERY, "total due")),
    ));
    assert_eq!(
        phrase,
        format!(
            "(({DOC} @@ plainto_tsquery('english', $<total due>)) AND \
             (({CONTENT} @@ phraseto_tsquery('english', $<total due>)) OR \
             ({OCR} @@ phraseto_tsquery('english', $<total due>))))"
        )
    );
    assert_eq!(binds.len(), 3);
    assert_eq!(TO_TSQUERY, "to_tsquery('english', ");
}
