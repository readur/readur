//! Every search example in docs/advanced-search.md (and the boolean examples of
//! docs/user-guide.md), checked against the parser and a seeded search.
//!
//! `EXAMPLES` records, for each documented boolean query, the `to_tsquery` input the parser must
//! produce and exactly which documents of `CORPUS` it must find. `MODE_EXAMPLES` does the same for
//! the simple, phrase and fuzzy examples, and `API_EXAMPLES` for the example requests, which run
//! over HTTP against `API_CORPUS`. The docs are read at test time: an example added to them
//! without an entry here fails a `every_documented_*_is_covered` test, and the "The same" /
//! "Same as:" notes in the boolean code blocks are checked as written.

use std::collections::BTreeSet;

use axum::{body::Body, http::Request, http::StatusCode};
use serde_json::{json, Value};
use tower::util::ServiceExt;
use uuid::Uuid;

use readur::db::documents::boolean_to_tsquery;
use readur::test_utils::{document_helpers::create_test_document, TestContext};

/// Which documents of `CORPUS` a query finds.
enum Hits {
    Only(&'static [&'static str]),
    AllBut(&'static [&'static str]),
}

struct Example {
    query: &'static str,
    tsquery: &'static str,
    hits: Hits,
}

const fn ex(query: &'static str, tsquery: &'static str, hits: Hits) -> Example {
    Example {
        query,
        tsquery,
        hits,
    }
}

use Hits::{AllBut, Only};

/// The seeded documents: key and OCR text.
const CORPUS: &[(&str, &str)] = &[
    ("budget2024", "annual budget for 2024 approved"),
    ("budget2023", "annual budget for 2023"),
    ("financial2024", "financial statement 2024"),
    ("invoice", "invoice number 17 total due 300"),
    ("invoice_paid", "invoice paid in full"),
    ("personal_invoice", "personal invoice gym membership"),
    ("receipt_paid", "receipt paid in cash"),
    ("receipt", "receipt pending"),
    ("contract", "contract final signed"),
    ("contract_draft", "contract draft copy"),
    ("due_total", "due total in reverse"),
    ("inventory", "inventory count"),
    ("marketing_campaign", "marketing campaign plan"),
    ("marketing", "marketing overview"),
    ("project_final", "project final report"),
    ("project_draft", "project final draft"),
    ("project_status", "weekly project status update"),
    ("po_approved", "purchase order approved"),
    ("po_cancelled", "purchase order approved but cancelled"),
    ("proposal", "proposal for a client"),
    ("proposal_old", "old proposal"),
    ("email", "please send an e-mail"),
    ("warranty", "warranty card"),
];

const INVOICE_OR_RECEIPT: Hits = Only(&[
    "invoice",
    "invoice_paid",
    "personal_invoice",
    "receipt_paid",
    "receipt",
]);
const INVOICES: Hits = Only(&["invoice", "invoice_paid", "personal_invoice"]);
const INVOICE_OR_RECEIPT_PAID: Hits = Only(&[
    "invoice",
    "invoice_paid",
    "personal_invoice",
    "receipt_paid",
]);
const CONTRACT_NOT_DRAFT: Hits = Only(&["contract"]);

const EXAMPLES: &[Example] = &[
    // AND: keyword, symbol or adjacency.
    ex("budget 2024", "('budget' & '2024')", Only(&["budget2024"])),
    ex(
        "budget & 2024",
        "('budget' & '2024')",
        Only(&["budget2024"]),
    ),
    ex(
        "budget AND 2024",
        "('budget' & '2024')",
        Only(&["budget2024"]),
    ),
    ex(
        "invoice paid",
        "('invoice' & 'paid')",
        Only(&["invoice_paid"]),
    ),
    ex(
        "invoice & paid",
        "('invoice' & 'paid')",
        Only(&["invoice_paid"]),
    ),
    ex(
        "invoice AND paid",
        "('invoice' & 'paid')",
        Only(&["invoice_paid"]),
    ),
    // OR.
    ex(
        "invoice | receipt",
        "('invoice' | 'receipt')",
        INVOICE_OR_RECEIPT,
    ),
    ex(
        "invoice OR receipt",
        "('invoice' | 'receipt')",
        INVOICE_OR_RECEIPT,
    ),
    // NOT, and the minus that only negates when attached.
    ex(
        "contract !draft",
        "('contract' & !'draft')",
        CONTRACT_NOT_DRAFT,
    ),
    ex(
        "contract NOT draft",
        "('contract' & !'draft')",
        CONTRACT_NOT_DRAFT,
    ),
    ex(
        "contract -draft",
        "('contract' & !'draft')",
        CONTRACT_NOT_DRAFT,
    ),
    ex(
        "contract - draft",
        "('contract' & 'draft')",
        Only(&["contract_draft"]),
    ),
    ex("e-mail", "'e-mail'", Only(&["email"])),
    ex("invoice -draft", "('invoice' & !'draft')", INVOICES),
    ex(
        "budget NOT personal",
        "('budget' & !'personal')",
        Only(&["budget2024", "budget2023"]),
    ),
    ex(
        "proposal !(archive | old)",
        "('proposal' & !('archive' | 'old'))",
        Only(&["proposal"]),
    ),
    // Grouping and precedence.
    ex(
        "(budget | financial) & 2024",
        "(('budget' | 'financial') & '2024')",
        Only(&["budget2024", "financial2024"]),
    ),
    ex(
        "marketing (campaign OR strategy)",
        "('marketing' & ('campaign' | 'strategy'))",
        Only(&["marketing_campaign"]),
    ),
    ex(
        "invoice | receipt paid",
        "('invoice' | ('receipt' & 'paid'))",
        INVOICE_OR_RECEIPT_PAID,
    ),
    ex(
        "invoice | (receipt & paid)",
        "('invoice' | ('receipt' & 'paid'))",
        INVOICE_OR_RECEIPT_PAID,
    ),
    // Phrases, and negated phrases and groups.
    ex("\"total due\"", "('total' <-> 'due')", Only(&["invoice"])),
    ex(
        "\"project status\"",
        "('project' <-> 'status')",
        Only(&["project_status"]),
    ),
    ex(
        "-\"draft copy\"",
        "!('draft' <-> 'copy')",
        AllBut(&["contract_draft"]),
    ),
    ex(
        "NOT (draft | copy)",
        "!('draft' | 'copy')",
        AllBut(&["contract_draft", "project_draft"]),
    ),
    // Prefixes.
    ex(
        "inv*",
        "'inv':*",
        Only(&["invoice", "invoice_paid", "personal_invoice", "inventory"]),
    ),
    ex(
        "inv:*",
        "'inv':*",
        Only(&["invoice", "invoice_paid", "personal_invoice", "inventory"]),
    ),
    ex(
        "proj*",
        "'proj':*",
        Only(&["project_final", "project_draft", "project_status"]),
    ),
    // Followed-by.
    ex("total <-> due", "('total' <-> 'due')", Only(&["invoice"])),
    ex("invoice <2> 17", "('invoice' <2> '17')", Only(&["invoice"])),
    // Single words the operator table mentions.
    ex("inv", "'inv'", Only(&[])),
    ex("invoice", "'invoice'", INVOICES),
    ex("total", "'total'", Only(&["invoice", "due_total"])),
    ex("due", "'due'", Only(&["invoice", "due_total"])),
    ex("17", "'17'", Only(&["invoice"])),
    // Advanced examples.
    ex(
        "project (final | completed | approved) !draft",
        "('project' & ('final' | 'completed' | 'approved') & !'draft')",
        Only(&["project_final"]),
    ),
    ex(
        "(invoice OR receipt OR budget) NOT personal",
        "(('invoice' | 'receipt' | 'budget') & !'personal')",
        Only(&[
            "invoice",
            "invoice_paid",
            "receipt_paid",
            "receipt",
            "budget2024",
            "budget2023",
        ]),
    ),
    ex(
        "\"purchase order\" (approved | signed) -cancel*",
        "(('purchase' <-> 'order') & ('approved' | 'signed') & !'cancel':*)",
        Only(&["po_approved"]),
    ),
    // Malformed input degrades.
    ex("invoice &", "'invoice'", INVOICES),
    ex(
        "(invoice | receipt",
        "('invoice' | 'receipt')",
        INVOICE_OR_RECEIPT,
    ),
    ex("\"total due", "('total' <-> 'due')", Only(&["invoice"])),
];

/// A documented section of examples in one search mode: its heading, the `search_mode`, and each
/// query with the documents of `CORPUS` it finds.
type ModeSection = (&'static str, &'static str, &'static [(&'static str, Hits)]);

/// The documented examples of the other search modes.
const MODE_EXAMPLES: &[ModeSection] = &[
    (
        "### Simple Search (All Words)",
        "simple",
        &[
            ("budget 2024", Only(&["budget2024"])),
            ("budgets approved", Only(&["budget2024"])),
            ("the invoice", INVOICES),
            ("inv", Only(&[])),
            ("contract -draft", Only(&["contract_draft"])),
        ],
    ),
    (
        "### Phrase Search (Exact Phrase)",
        "phrase",
        &[
            ("total due", Only(&["invoice"])),
            ("due total", Only(&["due_total"])),
            ("\"total due\"", Only(&["invoice"])),
            ("annual budget", Only(&["budget2024", "budget2023"])),
        ],
    ),
    (
        "### Fuzzy Search (Similar Spelling)",
        "fuzzy",
        &[
            ("waranty card", Only(&["warranty"])),
            ("warranty", Only(&["warranty"])),
            ("invoce", Only(&[])),
        ],
    ),
];

fn example(query: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|e| e.query == query)
}

// ------------------------------------------------------------ doc parsing

/// A query quoted in the docs, with the note written after it in a code block.
struct Documented {
    source: String,
    query: String,
    note: Option<String>,
}

/// The text from `heading` to the next heading of the same or a higher level.
fn section<'a>(doc: &'a str, heading: &str) -> &'a str {
    let start = doc
        .find(&format!("\n{heading}\n"))
        .unwrap_or_else(|| panic!("heading {heading:?} not found"));
    let body = &doc[start + 1 + heading.len()..];
    let level = heading.split(' ').next().unwrap().len();
    let (mut offset, mut in_fence) = (0, false);
    for line in body.lines() {
        // `#` comments inside code blocks are not headings.
        in_fence ^= line.starts_with("```");
        let hashes = line.split(' ').next().unwrap_or("");
        if !in_fence
            && !hashes.is_empty()
            && hashes.chars().all(|c| c == '#')
            && hashes.len() <= level
        {
            return &body[..offset];
        }
        offset += line.len() + 1;
    }
    body
}

/// Inline code spans on a line, with Markdown's `\|` table escape undone.
fn code_spans(line: &str) -> Vec<String> {
    line.split('`')
        .skip(1)
        .step_by(2)
        .map(|span| span.replace("\\|", "|"))
        .collect()
}

/// The queries in a section's operator tables and code blocks.
fn documented_queries(source: &str, text: &str) -> Vec<Documented> {
    let mut found = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_fence = !in_fence;
        } else if in_fence {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (query, note) = match line.split_once(" #") {
                Some((query, note)) => (query.trim(), Some(note.trim().to_string())),
                None => (line, None),
            };
            found.push(Documented {
                source: source.to_string(),
                query: query.to_string(),
                note,
            });
        } else if line.starts_with('|') {
            found.extend(code_spans(line).into_iter().map(|query| Documented {
                source: source.to_string(),
                query,
                note: None,
            }));
        }
    }
    found
}

fn all_documented() -> Vec<Documented> {
    let advanced =
        std::fs::read_to_string("docs/advanced-search.md").expect("read advanced-search.md");
    let guide = std::fs::read_to_string("docs/user-guide.md").expect("read user-guide.md");
    let mut all = Vec::new();
    for heading in [
        "### Boolean Search (Logical Operators)",
        "### Wildcard Search",
        "### Exclusion Operators",
    ] {
        all.extend(documented_queries(
            &format!("advanced-search.md {heading}"),
            section(&advanced, heading),
        ));
    }
    let heading = "### Boolean (Operators) Search";
    all.extend(documented_queries(
        &format!("user-guide.md {heading}"),
        section(&guide, heading),
    ));
    all
}

fn advanced_search_doc() -> String {
    std::fs::read_to_string("docs/advanced-search.md").expect("read advanced-search.md")
}

/// The fenced code blocks of a section, without the fence lines.
fn code_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if line.starts_with("```") {
            match current.take() {
                Some(block) => blocks.push(block),
                None => current = Some(String::new()),
            }
        } else if let Some(block) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    blocks
}

// ----------------------------------------------------------------- tests

#[test]
fn every_example_renders_as_recorded() {
    for e in EXAMPLES {
        assert_eq!(
            boolean_to_tsquery(e.query),
            e.tsquery,
            "query {:?}",
            e.query
        );
    }
}

#[test]
fn every_documented_example_is_covered() {
    let documented = all_documented();
    assert!(
        documented.len() >= 40,
        "too few examples found; did a heading change? {}",
        documented.len()
    );
    let missing: Vec<String> = documented
        .iter()
        .filter(|d| example(&d.query).is_none())
        .map(|d| format!("{}: {:?}", d.source, d.query))
        .collect();
    assert!(
        missing.is_empty(),
        "documented examples without an entry in EXAMPLES:\n{}",
        missing.join("\n")
    );
}

#[test]
fn documented_equivalences_hold() {
    let documented = all_documented();
    let mut checked = 0;
    for (i, d) in documented.iter().enumerate() {
        let Some(note) = &d.note else { continue };
        let other = if let Some(target) = note.strip_prefix("Same as:") {
            target.trim().to_string()
        } else if note.starts_with("The same") {
            documented[i - 1].query.clone()
        } else {
            continue;
        };
        assert_eq!(
            boolean_to_tsquery(&d.query),
            boolean_to_tsquery(&other),
            "{}: {:?} should equal {:?}",
            d.source,
            d.query,
            other
        );
        checked += 1;
    }
    assert!(
        checked >= 6,
        "expected the docs' equivalence notes to be checked, got {checked}"
    );
}

/// The simple, phrase and fuzzy sections document exactly the queries `MODE_EXAMPLES` checks.
#[test]
fn every_documented_mode_example_is_covered() {
    let doc = advanced_search_doc();
    for (heading, mode, examples) in MODE_EXAMPLES {
        let documented: Vec<String> = documented_queries(heading, section(&doc, heading))
            .into_iter()
            .map(|d| d.query)
            .collect();
        let recorded: Vec<String> = examples.iter().map(|(q, _)| q.to_string()).collect();
        assert_eq!(
            documented, recorded,
            "{heading} ({mode}): the documented queries and MODE_EXAMPLES differ"
        );
    }
}

/// Postgres accepts every rendered example.
#[tokio::test]
async fn every_example_is_valid_tsquery() {
    let ctx = TestContext::new().await;
    for e in EXAMPLES {
        sqlx::query("SELECT to_tsquery('english', $1)")
            .bind(e.tsquery)
            .execute(&ctx.state().db.pool)
            .await
            .unwrap_or_else(|err| panic!("{:?} -> {:?} rejected: {err}", e.query, e.tsquery));
    }
}

fn expected_keys(hits: &Hits) -> BTreeSet<&'static str> {
    match hits {
        Only(keys) => keys.iter().copied().collect(),
        AllBut(keys) => CORPUS
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| !keys.contains(k))
            .collect(),
    }
}

/// Each example finds exactly the documents recorded for it, through MCP `enhanced_search` in
/// its mode (the same query builder as the REST endpoint and the web UI's search modes).
#[tokio::test]
async fn every_example_finds_exactly_the_recorded_documents() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    let user_id: Uuid = user.user_response.id;

    let mut key_by_id = std::collections::HashMap::new();
    for (key, text) in CORPUS {
        let mut doc = create_test_document(user_id);
        doc.content = None;
        doc.ocr_text = Some(text.to_string());
        doc.tags = Vec::new();
        doc.file_hash = Some(format!("{:x}", Uuid::new_v4().as_u128()));
        let id = ctx
            .state()
            .db
            .create_document(doc)
            .await
            .unwrap()
            .id
            .to_string();
        key_by_id.insert(id, *key);
    }

    let boolean = EXAMPLES.iter().map(|e| ("boolean", e.query, &e.hits));
    let others = MODE_EXAMPLES
        .iter()
        .flat_map(|(_, mode, examples)| examples.iter().map(move |(q, hits)| (*mode, *q, hits)));
    for (mode, query, hits) in boolean.chain(others) {
        let result = ctx
            .mcp_call_tool(
                &token,
                "enhanced_search",
                json!({ "query": query, "search_mode": mode, "limit": 100 }),
            )
            .await;
        assert_eq!(
            result["isError"],
            json!(false),
            "{mode} {query:?}: {result}"
        );
        let found: BTreeSet<&str> = result["structuredContent"]["documents"]
            .as_array()
            .unwrap_or_else(|| panic!("{mode} {query:?}: no documents array: {result}"))
            .iter()
            .map(|doc: &Value| key_by_id[doc["id"].as_str().unwrap()])
            .collect();
        assert_eq!(found, expected_keys(hits), "{mode} query {query:?}");
    }
}

// ------------------------------------------------------------ API examples

/// One document of `API_CORPUS`: key, MIME type, text, OCR status, creation date, source type
/// and whether it carries the "Finance" label.
type ApiDocument = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
    bool,
);

/// The library the example requests run against.
const API_CORPUS: &[ApiDocument] = &[
    (
        "pdf_invoice",
        "application/pdf",
        "invoice for march",
        "completed",
        "2024-03-10T09:00:00Z",
        None,
        true,
    ),
    (
        "png_invoice",
        "image/png",
        "scanned invoice",
        "pending",
        "2024-06-01T09:00:00Z",
        Some("webdav"),
        false,
    ),
    (
        "txt_contract",
        "text/plain",
        "contract signed",
        "failed",
        "2025-01-05T09:00:00Z",
        Some("local_folder"),
        true,
    ),
    (
        "pdf_contract",
        "application/pdf",
        "contract draft",
        "processing",
        "2023-11-20T09:00:00Z",
        Some("webdav"),
        false,
    ),
];

/// Each documented request and the documents it returns. `ordered` results are compared in
/// order (the request sorts, or pages a newest-first list); the others as a set.
const API_EXAMPLES: &[(&str, &[&str], bool)] = &[
    (
        "/api/search/enhanced?query=invoice",
        &["pdf_invoice", "png_invoice"],
        false,
    ),
    (
        "/api/search/enhanced?query=invoice&mime_types=application/pdf",
        &["pdf_invoice"],
        false,
    ),
    (
        "/api/search/enhanced?query=contract&ocr_status=failed",
        &["txt_contract"],
        false,
    ),
    (
        "/api/search/enhanced?label_ids=<label-id>",
        &["pdf_invoice", "txt_contract"],
        false,
    ),
    (
        "/api/search/enhanced?query=contract&tags=Finance",
        &["txt_contract"],
        false,
    ),
    (
        "/api/search/enhanced?query=contract&source_types=webdav",
        &["pdf_contract"],
        false,
    ),
    (
        "/api/search/enhanced?query=invoice&created_from=2024-05-01T00:00:00Z",
        &["png_invoice"],
        false,
    ),
    (
        "/api/search/enhanced?query=contract%20!draft&search_mode=boolean",
        &["txt_contract"],
        false,
    ),
    (
        "/api/search/enhanced?query=contract&sort_by=created_at&sort_order=asc",
        &["pdf_contract", "txt_contract"],
        true,
    ),
    ("/api/search?query=invoice&limit=1", &["png_invoice"], true),
];

/// The request paths in the "Example Requests" code blocks.
fn documented_requests(doc: &str) -> Vec<String> {
    code_blocks(section(doc, "### Example Requests"))
        .iter()
        .flat_map(|block| block.lines().map(str::to_string).collect::<Vec<_>>())
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let request = line
                .strip_prefix("GET ")
                .unwrap_or_else(|| panic!("example request is not a GET: {line:?}"));
            request.split_whitespace().next().unwrap().to_string()
        })
        .collect()
}

#[test]
fn every_documented_request_is_covered() {
    let documented = documented_requests(&advanced_search_doc());
    let recorded: Vec<String> = API_EXAMPLES.iter().map(|(r, _, _)| r.to_string()).collect();
    assert_eq!(
        documented, recorded,
        "the documented requests and API_EXAMPLES differ"
    );
}

async fn get_json(ctx: &TestContext, uri: &str, token: &str) -> (StatusCode, Value) {
    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn object_keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("not an object: {value}"))
        .keys()
        .cloned()
        .collect()
}

/// Each documented request returns exactly the recorded documents, and the documented response
/// shape matches a real response field for field.
#[tokio::test]
async fn every_documented_request_returns_the_recorded_documents() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    let user_id: Uuid = user.user_response.id;
    let pool = &ctx.state().db.pool;

    let (label_id,): (Uuid,) =
        sqlx::query_as("INSERT INTO labels (user_id, name) VALUES ($1, 'Finance') RETURNING id")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .unwrap();
    let mut key_by_id = std::collections::HashMap::new();
    for (key, mime, text, status, created, source_type, finance) in API_CORPUS {
        let mut doc = create_test_document(user_id);
        doc.mime_type = mime.to_string();
        doc.content = None;
        doc.ocr_text = Some(text.to_string());
        doc.ocr_status = Some(status.to_string());
        doc.created_at = created.parse().unwrap();
        doc.source_type = source_type.map(str::to_string);
        doc.tags = Vec::new();
        doc.file_hash = Some(format!("{:x}", Uuid::new_v4().as_u128()));
        let id = ctx.state().db.create_document(doc).await.unwrap().id;
        if *finance {
            sqlx::query("INSERT INTO document_labels (document_id, label_id) VALUES ($1, $2)")
                .bind(id)
                .bind(label_id)
                .execute(pool)
                .await
                .unwrap();
        }
        key_by_id.insert(id.to_string(), *key);
    }

    for (request, expected, ordered) in API_EXAMPLES {
        let uri = request.replace("<label-id>", &label_id.to_string());
        let (status, body) = get_json(&ctx, &uri, &token).await;
        assert_eq!(status, StatusCode::OK, "{request}: {body}");
        let found: Vec<&str> = body["documents"]
            .as_array()
            .unwrap_or_else(|| panic!("{request}: no documents array: {body}"))
            .iter()
            .map(|doc| key_by_id[doc["id"].as_str().unwrap()])
            .collect();
        if *ordered {
            assert_eq!(found, *expected, "{request}");
        } else {
            let found: BTreeSet<&str> = found.into_iter().collect();
            let expected: BTreeSet<&str> = expected.iter().copied().collect();
            assert_eq!(found, expected, "{request}");
        }
    }

    // The documented response: same top-level fields, and the same document fields (the OCR
    // progress fields only appear while a document is being processed).
    let doc = advanced_search_doc();
    let documented: Value = code_blocks(section(&doc, "### Response"))
        .iter()
        .find_map(|block| serde_json::from_str(block).ok())
        .expect("a JSON response example under ### Response");
    let (status, real) = get_json(
        &ctx,
        "/api/search/enhanced?query=invoice&mime_types=application/pdf",
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(object_keys(&documented), object_keys(&real));
    let real_doc = &real["documents"][0];
    assert_eq!(
        object_keys(&documented["documents"][0]),
        object_keys(real_doc)
    );
    assert_eq!(
        object_keys(&documented["documents"][0]["snippets"][0]),
        object_keys(&real_doc["snippets"][0])
    );
    assert_eq!(real["total"], 1);
    assert!(real["suggestions"].as_array().is_some());
}
