//! Boolean-mode search: the parser's `to_tsquery` output against a real
//! Postgres (fuzzed), its semantics through MCP `enhanced_search` and the
//! REST `/api/search/enhanced` endpoint, and snippet highlighting.

use serde_json::{json, Value};
use uuid::Uuid;

use readur::db::documents::boolean_to_tsquery;
use readur::test_utils::{document_helpers::create_test_document, TestContext};

/// A user and a session token for them.
async fn user_with_token(ctx: &TestContext) -> (Uuid, String) {
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    (user.user_response.id, token)
}

/// Inserts a completed document whose OCR text is `ocr_text`; returns its id.
async fn seed(ctx: &TestContext, user_id: Uuid, ocr_text: &str) -> String {
    let mut doc = create_test_document(user_id);
    doc.content = None;
    doc.ocr_text = Some(ocr_text.to_string());
    doc.tags = Vec::new();
    doc.file_hash = Some(format!("{:x}", Uuid::new_v4().as_u128()));
    ctx.state()
        .db
        .create_document(doc)
        .await
        .unwrap()
        .id
        .to_string()
}

/// MCP `enhanced_search` in `mode`; asserts success.
async fn mcp_search(ctx: &TestContext, token: &str, query: &str, mode: &str) -> Value {
    let result = ctx
        .mcp_call_tool(
            token,
            "enhanced_search",
            json!({ "query": query, "search_mode": mode, "limit": 100 }),
        )
        .await;
    assert_eq!(
        result["isError"],
        json!(false),
        "{mode} {query:?}: {result}"
    );
    result["structuredContent"].clone()
}

fn ids(documents: &Value) -> Vec<String> {
    documents
        .as_array()
        .expect("no documents array")
        .iter()
        .map(|doc| doc["id"].as_str().unwrap().to_string())
        .collect()
}

/// Ids matching a boolean query via MCP.
async fn boolean_ids(ctx: &TestContext, token: &str, query: &str) -> Vec<String> {
    ids(&mcp_search(ctx, token, query, "boolean").await["documents"])
}

/// Asserts `hits` contains every id in `expected` and none in `excluded`.
fn assert_hits(query: &str, hits: &[String], expected: &[&String], excluded: &[&String]) {
    for id in expected {
        assert!(hits.contains(id), "{query:?} missed {id}: {hits:?}");
    }
    for id in excluded {
        assert!(
            !hits.contains(id),
            "{query:?} matched excluded {id}: {hits:?}"
        );
    }
}

// ------------------------------------------------------------------ fuzz

/// xorshift64*: a fixed-seed generator, so every run tests the same inputs.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

/// Query fragments: words (including stop words and keyword look-alikes),
/// every operator spelling, broken operators, quotes, escapes, punctuation
/// and multi-byte text.
const FRAGMENTS: &[&str] = &[
    "invoice",
    "receipt",
    "draft",
    "2024",
    "the",
    "a",
    "of",
    "e-mail",
    "o'brien",
    "inv",
    "AND",
    "and",
    "OR",
    "or",
    "NOT",
    "not",
    "Or",
    "&",
    "|",
    "!",
    "-",
    "(",
    ")",
    "\"",
    "'",
    "''",
    "\\",
    "\\'",
    ":",
    "*",
    ":*",
    ":A",
    "<->",
    "<2>",
    "<0>",
    "<99999>",
    "<",
    "<-",
    ">",
    "<x>",
    ",",
    ".",
    ";",
    "%",
    "_",
    "$",
    "@",
    "#",
    "~",
    "`",
    "{",
    "}",
    "[",
    "]",
    "+",
    "=",
    "?",
    "/",
    "日本語",
    "тест",
    "é",
    "İ",
    "ß",
    "🎉",
    "\u{200b}",
    "\u{0}",
    "\t",
    "\n",
];

fn fragment_query(rng: &mut Rng) -> String {
    let count = 1 + rng.below(12);
    (0..count)
        .map(|_| {
            let separator = rng.pick(&["", " ", " ", "  "]);
            format!("{}{separator}", rng.pick(FRAGMENTS))
        })
        .collect()
}

/// Arbitrary characters, biased towards ASCII punctuation, with random
/// Unicode scalar values mixed in.
fn char_query(rng: &mut Rng) -> String {
    let len = rng.below(40);
    (0..len)
        .map(|_| match rng.below(4) {
            0 => char::from(b' ' + rng.below(95) as u8),
            1 => " ()&|!\"'\\:*<->-".chars().nth(rng.below(15)).unwrap(),
            2 => char::from(b'a' + rng.below(26) as u8),
            _ => char::from_u32(rng.below(0x11_0000) as u32).unwrap_or('?'),
        })
        .collect()
}

#[tokio::test]
async fn fuzzed_boolean_queries_always_render_valid_tsquery() {
    let ctx = TestContext::new().await;
    let pool = &ctx.state().db.pool;
    let mut rng = Rng(0x5EED_CAFE_F00D_D00D);
    let mut non_empty = 0;
    for i in 0..6_000 {
        let input = if i % 3 == 2 {
            char_query(&mut rng)
        } else {
            fragment_query(&mut rng)
        };
        let rendered = boolean_to_tsquery(&input);
        non_empty += usize::from(!rendered.is_empty());
        // Parse, match and rank, as the search query does.
        let result: Result<(bool, f32), _> = sqlx::query_as(
            "SELECT to_tsvector('english', $2) @@ to_tsquery('english', $1), \
             ts_rank(to_tsvector('english', $2), to_tsquery('english', $1))",
        )
        .bind(&rendered)
        .bind("the invoice receipt draft 2024 e-mail o'brien тест 日本語")
        .fetch_one(pool)
        .await;
        if let Err(e) = result {
            panic!("input {input:?} rendered {rendered:?}: {e}");
        }
    }
    // Guard against a generator that only ever produces empty queries.
    assert!(non_empty > 3_000, "only {non_empty} non-empty renderings");
}

#[tokio::test]
async fn malformed_boolean_queries_never_fail_through_mcp() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let doc = seed(&ctx, user_id, "invoice paid").await;
    for query in [
        "((invoice",
        "invoice)))",
        "invoice &",
        "| | invoice",
        "!!",
        "invoice & | paid",
        "\"unterminated",
        "NOT",
        "()",
        "-\"\"",
        "<-> invoice <->",
        "invoice:*:*",
        "\\'",
        "'); DROP TABLE documents; --",
    ] {
        mcp_search(&ctx, &token, query, "boolean").await;
    }
    // Only the trailing garbage degrades; the word still matches.
    assert_eq!(boolean_ids(&ctx, &token, "((invoice").await, vec![doc]);
}

// ------------------------------------------------------------- semantics

struct Corpus {
    inv_2024: String,
    inv_2023: String,
    receipt_2024: String,
    draft_copy: String,
    draft_apart: String,
    inventory: String,
}

async fn seed_corpus(ctx: &TestContext, user_id: Uuid) -> Corpus {
    Corpus {
        inv_2024: seed(ctx, user_id, "invoice number 17 issued 2024 total due 500").await,
        inv_2023: seed(ctx, user_id, "invoice number 9 issued 2023 total due 300").await,
        receipt_2024: seed(ctx, user_id, "receipt for lunch 2024").await,
        draft_copy: seed(ctx, user_id, "invoice 2024 draft copy pending approval").await,
        draft_apart: seed(ctx, user_id, "invoice 2024 draft notes and the final copy").await,
        inventory: seed(ctx, user_id, "inventory count for the warehouse").await,
    }
}

#[tokio::test]
async fn negated_phrase_excludes_only_the_phrase() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let c = seed_corpus(&ctx, user_id).await;
    // The minus used to be dropped, making this a positive phrase match.
    for query in [
        "invoice -\"draft copy\"",
        "invoice NOT \"draft copy\"",
        "invoice & !\"draft copy\"",
    ] {
        let hits = boolean_ids(&ctx, &token, query).await;
        assert_hits(
            query,
            &hits,
            &[&c.inv_2024, &c.inv_2023, &c.draft_apart],
            &[&c.draft_copy, &c.receipt_2024],
        );
    }
}

#[tokio::test]
async fn parentheses_group() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let c = seed_corpus(&ctx, user_id).await;
    for query in ["(invoice | receipt) & 2024", "(invoice OR receipt) 2024"] {
        let hits = boolean_ids(&ctx, &token, query).await;
        assert_hits(
            query,
            &hits,
            &[&c.inv_2024, &c.receipt_2024, &c.draft_copy],
            &[&c.inv_2023, &c.inventory],
        );
    }
}

#[tokio::test]
async fn negated_group_excludes_every_alternative() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let c = seed_corpus(&ctx, user_id).await;
    // Without grouping only `draft` was negated, so `copy` slipped through.
    for query in ["invoice NOT (draft | copy)", "invoice -(draft OR copy)"] {
        let hits = boolean_ids(&ctx, &token, query).await;
        assert_hits(
            query,
            &hits,
            &[&c.inv_2024, &c.inv_2023],
            &[&c.draft_copy, &c.draft_apart, &c.receipt_2024],
        );
    }
    let hits = boolean_ids(&ctx, &token, "NOT (draft | copy)").await;
    assert_hits(
        "NOT (draft | copy)",
        &hits,
        &[&c.inv_2024, &c.receipt_2024, &c.inventory],
        &[&c.draft_copy, &c.draft_apart],
    );
}

#[tokio::test]
async fn operators_apply_to_the_whole_document() {
    // Content and OCR text used to be matched separately, so `!draft`
    // succeeded on the empty or draft-free column of a draft document.
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let mut doc = create_test_document(user_id);
    doc.content = Some("invoice summary".to_string());
    doc.ocr_text = Some("draft copy".to_string());
    doc.file_hash = Some(format!("{:x}", Uuid::new_v4().as_u128()));
    let split = ctx
        .state()
        .db
        .create_document(doc)
        .await
        .unwrap()
        .id
        .to_string();
    let clean = seed(&ctx, user_id, "invoice final").await;

    let hits = boolean_ids(&ctx, &token, "invoice -draft").await;
    assert_hits("invoice -draft", &hits, &[&clean], &[&split]);
    let hits = boolean_ids(&ctx, &token, "invoice & copy").await;
    assert_hits("invoice & copy", &hits, &[&split], &[&clean]);
}

#[tokio::test]
async fn prefix_matching() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let c = seed_corpus(&ctx, user_id).await;
    for query in ["inv:*", "inv*"] {
        let hits = boolean_ids(&ctx, &token, query).await;
        assert_hits(
            query,
            &hits,
            &[&c.inv_2024, &c.inv_2023, &c.draft_copy, &c.inventory],
            &[&c.receipt_2024],
        );
    }
    let hits = boolean_ids(&ctx, &token, "inv:* & !invoice").await;
    assert_eq!(hits, vec![c.inventory.clone()]);
}

#[tokio::test]
async fn followed_by_and_phrases() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let c = seed_corpus(&ctx, user_id).await;
    for query in ["total <-> due", "\"total due\""] {
        let hits = boolean_ids(&ctx, &token, query).await;
        assert_hits(query, &hits, &[&c.inv_2024, &c.inv_2023], &[&c.draft_copy]);
    }
    assert!(boolean_ids(&ctx, &token, "due <-> total").await.is_empty());
    // "invoice number 17": two positions apart.
    assert_eq!(
        boolean_ids(&ctx, &token, "invoice <2> 17").await,
        vec![c.inv_2024.clone()]
    );
    assert!(boolean_ids(&ctx, &token, "invoice <-> 17").await.is_empty());
}

// ------------------------------------------------------------------ REST

/// Ids from `GET /api/search/enhanced` in boolean mode.
async fn rest_boolean_ids(ctx: &TestContext, token: &str, query: &str) -> Vec<String> {
    let uri = format!(
        "/api/search/enhanced?search_mode=boolean&limit=100&query={}",
        urlencoding::encode(query)
    );
    let body = ctx
        .auth_helper()
        .make_authenticated_request("GET", &uri, None, token)
        .await;
    let body: Value = serde_json::from_slice(&body).unwrap_or_else(|e| {
        panic!(
            "{query:?}: not JSON ({e}): {}",
            String::from_utf8_lossy(&body)
        )
    });
    ids(&body["documents"])
}

#[tokio::test]
async fn rest_boolean_search_matches_the_web_ui_examples() {
    // The examples frontend/src/features/library/SearchHelp.tsx offers.
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let paid = seed(&ctx, user_id, "invoice paid in full").await;
    let unpaid = seed(&ctx, user_id, "invoice overdue").await;
    let budget = seed(&ctx, user_id, "budget for next year").await;
    let forecast = seed(&ctx, user_id, "sales forecast").await;
    let contract = seed(&ctx, user_id, "contract signed").await;
    let contract_draft = seed(&ctx, user_id, "contract draft").await;

    let hits = rest_boolean_ids(&ctx, &token, "invoice & paid").await;
    assert_hits("invoice & paid", &hits, &[&paid], &[&unpaid]);

    let hits = rest_boolean_ids(&ctx, &token, "budget | forecast").await;
    assert_hits(
        "budget | forecast",
        &hits,
        &[&budget, &forecast],
        &[&paid, &contract],
    );

    let hits = rest_boolean_ids(&ctx, &token, "contract & !draft").await;
    assert_hits("contract & !draft", &hits, &[&contract], &[&contract_draft]);

    // Grouping, prefixes and followed-by, which raw to_tsquery offered REST
    // clients before boolean input was parsed.
    let hits = rest_boolean_ids(&ctx, &token, "(budget | forecast) & !sales").await;
    assert_hits("grouped", &hits, &[&budget], &[&forecast]);
    let hits = rest_boolean_ids(&ctx, &token, "contr:*").await;
    assert_hits("contr:*", &hits, &[&contract, &contract_draft], &[&paid]);
    let hits = rest_boolean_ids(&ctx, &token, "contract <-> draft").await;
    assert_hits(
        "contract <-> draft",
        &hits,
        &[&contract_draft],
        &[&contract],
    );

    // Malformed input degrades rather than failing the request.
    for query in ["((invoice", "invoice & | paid", "\"\"", "!!", "()"] {
        rest_boolean_ids(&ctx, &token, query).await;
    }
}

// -------------------------------------------------------------- snippets

/// The highlighted text of every snippet of the first result.
fn highlights(structured: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for snippet in structured["documents"][0]["snippets"].as_array().unwrap() {
        let text = snippet["text"].as_str().unwrap();
        for range in snippet["highlight_ranges"].as_array().unwrap() {
            let start = range["start"].as_u64().unwrap() as usize;
            let end = range["end"].as_u64().unwrap() as usize;
            out.push(text[start..end].to_lowercase());
        }
    }
    out
}

#[tokio::test]
async fn boolean_snippets_highlight_only_positive_terms() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let filler = "lorem ipsum dolor sit amet ".repeat(12);
    // "order", "notes" and "draft" would all have been highlighted when the
    // raw query was split on whitespace ("or", "not", "draft").
    seed(
        &ctx,
        user_id,
        &format!("An order was placed. {filler} The invoice arrived. {filler} Notes on the draft. {filler} A receipt was filed."),
    )
    .await;

    let structured = mcp_search(&ctx, &token, "invoice OR receipt NOT draft", "boolean").await;
    let found = highlights(&structured);
    assert!(!found.is_empty(), "no snippets: {structured}");
    for hit in &found {
        assert!(
            hit == "invoice" || hit == "receipt",
            "highlighted {hit:?}: {found:?}"
        );
    }
    assert!(found.contains(&"invoice".to_string()), "{found:?}");
    assert!(found.contains(&"receipt".to_string()), "{found:?}");
}

#[tokio::test]
async fn quoted_phrases_produce_snippets() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    seed(&ctx, user_id, "Statement: the total due is 500 dollars").await;

    // Boolean mode highlights the phrase as one range.
    let structured = mcp_search(&ctx, &token, "\"total due\"", "boolean").await;
    assert_eq!(highlights(&structured), ["total due"], "{structured}");

    // Phrase mode used to search for `"total` and `due"`, quotes included.
    let structured = mcp_search(&ctx, &token, "\"total due\"", "phrase").await;
    let found = highlights(&structured);
    assert!(!found.is_empty(), "no snippets: {structured}");
    for hit in &found {
        assert!(
            hit == "total" || hit == "due",
            "highlighted {hit:?}: {found:?}"
        );
    }
}
