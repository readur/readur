//! MCP label tools: listing, creating and updating labels, and assigning
//! them to documents, always scoped to the calling user.

use serde_json::{json, Value};
use uuid::Uuid;

use readur::test_utils::{document_helpers, TestContext, TestUser};

async fn user_with_token(ctx: &TestContext) -> (TestUser, String) {
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    (user, token)
}

async fn create_document(ctx: &TestContext, user: &TestUser) -> Uuid {
    let document = document_helpers::create_test_document(user.user_response.id);
    ctx.state.db.create_document(document).await.unwrap().id
}

fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap_or_default()
}

fn assert_ok(result: &Value) {
    assert_eq!(result["isError"], json!(false), "tool failed: {result}");
}

fn assert_tool_error(result: &Value) {
    assert_eq!(
        result["isError"],
        json!(true),
        "expected a tool error: {result}"
    );
}

async fn create_label(ctx: &TestContext, token: &str, name: &str) -> Uuid {
    let result = ctx
        .mcp_call_tool(token, "create_label", json!({ "name": name }))
        .await;
    assert_ok(&result);
    result["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap()
}

fn label_names(result: &Value) -> Vec<String> {
    result["structuredContent"]["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|label| label["name"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn label_tools_are_listed() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;
    let names = ctx.mcp_tool_names(&token).await;
    for tool in [
        "list_labels",
        "create_label",
        "update_label",
        "get_document_labels",
        "add_document_label",
        "remove_document_label",
    ] {
        assert!(names.iter().any(|n| n == tool), "{tool} missing: {names:?}");
    }
}

#[tokio::test]
async fn created_label_appears_in_list() {
    let ctx = TestContext::new().await;
    let (user, token) = user_with_token(&ctx).await;

    let result = ctx
        .mcp_call_tool(
            &token,
            "create_label",
            json!({ "name": "Invoices", "color": "#ff0000", "description": "Bills", "icon": "receipt" }),
        )
        .await;
    assert_ok(&result);
    let label = &result["structuredContent"];
    assert_eq!(label["name"], json!("Invoices"));
    assert_eq!(label["color"], json!("#ff0000"));
    assert_eq!(label["description"], json!("Bills"));
    assert_eq!(label["icon"], json!("receipt"));
    assert_eq!(label["user_id"], json!(user.user_response.id));

    let list = ctx.mcp_call_tool(&token, "list_labels", json!({})).await;
    assert_ok(&list);
    assert!(label_names(&list).contains(&"Invoices".to_string()));
    assert!(text(&list).contains("Invoices"));

    // Another user never sees it.
    let (_, other_token) = user_with_token(&ctx).await;
    let other_list = ctx
        .mcp_call_tool(&other_token, "list_labels", json!({}))
        .await;
    assert!(!label_names(&other_list).contains(&"Invoices".to_string()));
}

#[tokio::test]
async fn create_label_uses_default_color() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;
    let result = ctx
        .mcp_call_tool(&token, "create_label", json!({ "name": "Plain" }))
        .await;
    assert_ok(&result);
    assert_eq!(result["structuredContent"]["color"], json!("#0969da"));
}

#[tokio::test]
async fn list_labels_with_counts_reports_document_count() {
    let ctx = TestContext::new().await;
    let (user, token) = user_with_token(&ctx).await;
    let label_id = create_label(&ctx, &token, "Counted").await;
    let document_id = create_document(&ctx, &user).await;
    assert_ok(
        &ctx.mcp_call_tool(
            &token,
            "add_document_label",
            json!({ "document_id": document_id, "label_id": label_id }),
        )
        .await,
    );

    let list = ctx
        .mcp_call_tool(&token, "list_labels", json!({ "include_counts": true }))
        .await;
    assert_ok(&list);
    let counted = list["structuredContent"]["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|label| label["name"] == json!("Counted"))
        .unwrap();
    assert_eq!(counted["document_count"], json!(1));
}

#[tokio::test]
async fn create_label_rejects_invalid_input() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;

    for args in [
        json!({ "name": "Bad colour", "color": "red" }),
        json!({ "name": "Bad colour", "color": "#12345" }),
        json!({ "name": "   " }),
        json!({ "name": "a,b" }),
    ] {
        let result = ctx
            .mcp_call_tool(&token, "create_label", args.clone())
            .await;
        assert_tool_error(&result);
        assert!(!text(&result).is_empty(), "no message for {args}");
    }

    create_label(&ctx, &token, "Twice").await;
    let duplicate = ctx
        .mcp_call_tool(&token, "create_label", json!({ "name": "Twice" }))
        .await;
    assert_tool_error(&duplicate);
    assert!(text(&duplicate).contains("already exists"));
}

#[tokio::test]
async fn update_label_renames_and_keeps_other_fields() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;
    let result = ctx
        .mcp_call_tool(
            &token,
            "create_label",
            json!({ "name": "Old", "color": "#00ff00", "description": "keep me" }),
        )
        .await;
    let label_id = result["structuredContent"]["id"].as_str().unwrap();

    let updated = ctx
        .mcp_call_tool(
            &token,
            "update_label",
            json!({ "id": label_id, "name": "New" }),
        )
        .await;
    assert_ok(&updated);
    assert_eq!(updated["structuredContent"]["name"], json!("New"));
    assert_eq!(updated["structuredContent"]["color"], json!("#00ff00"));
    assert_eq!(
        updated["structuredContent"]["description"],
        json!("keep me")
    );

    let list = ctx.mcp_call_tool(&token, "list_labels", json!({})).await;
    let names = label_names(&list);
    assert!(names.contains(&"New".to_string()));
    assert!(!names.contains(&"Old".to_string()));
}

#[tokio::test]
async fn update_label_rejects_invalid_color() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;
    let label_id = create_label(&ctx, &token, "Colourful").await;
    let result = ctx
        .mcp_call_tool(
            &token,
            "update_label",
            json!({ "id": label_id, "color": "blue" }),
        )
        .await;
    assert_tool_error(&result);
}

#[tokio::test]
async fn cannot_modify_another_users_label() {
    // Labels are per user (there are no shared system labels); someone
    // else's label behaves as if it did not exist.
    let ctx = TestContext::new().await;
    let (_, owner_token) = user_with_token(&ctx).await;
    let label_id = create_label(&ctx, &owner_token, "Mine").await;

    let (intruder, intruder_token) = user_with_token(&ctx).await;
    let result = ctx
        .mcp_call_tool(
            &intruder_token,
            "update_label",
            json!({ "id": label_id, "name": "Stolen" }),
        )
        .await;
    assert_tool_error(&result);
    assert!(text(&result).to_lowercase().contains("not found"));

    // Nor attach it to their own document.
    let document_id = create_document(&ctx, &intruder).await;
    let add = ctx
        .mcp_call_tool(
            &intruder_token,
            "add_document_label",
            json!({ "document_id": document_id, "label_id": label_id }),
        )
        .await;
    assert_tool_error(&add);

    let list = ctx
        .mcp_call_tool(&owner_token, "list_labels", json!({}))
        .await;
    assert!(label_names(&list).contains(&"Mine".to_string()));
}

#[tokio::test]
async fn add_and_remove_label_on_own_document() {
    let ctx = TestContext::new().await;
    let (user, token) = user_with_token(&ctx).await;
    let label_id = create_label(&ctx, &token, "Tagged").await;
    let document_id = create_document(&ctx, &user).await;
    let ids = json!({ "document_id": document_id, "label_id": label_id });

    let empty = ctx
        .mcp_call_tool(
            &token,
            "get_document_labels",
            json!({ "document_id": document_id }),
        )
        .await;
    assert_ok(&empty);
    assert_eq!(empty["structuredContent"]["labels"], json!([]));

    assert_ok(
        &ctx.mcp_call_tool(&token, "add_document_label", ids.clone())
            .await,
    );
    // Adding again is a no-op, not an error.
    let again = ctx
        .mcp_call_tool(&token, "add_document_label", ids.clone())
        .await;
    assert_ok(&again);
    assert!(text(&again).contains("already"));

    let labels = ctx
        .mcp_call_tool(
            &token,
            "get_document_labels",
            json!({ "document_id": document_id }),
        )
        .await;
    assert_eq!(label_names(&labels), vec!["Tagged".to_string()]);
    assert!(text(&labels).contains("Tagged"));

    assert_ok(
        &ctx.mcp_call_tool(&token, "remove_document_label", ids.clone())
            .await,
    );
    let after = ctx
        .mcp_call_tool(
            &token,
            "get_document_labels",
            json!({ "document_id": document_id }),
        )
        .await;
    assert_eq!(after["structuredContent"]["labels"], json!([]));

    // remove_document_label is annotated idempotent: repeating it after the
    // label is gone succeeds (a client retrying after a timeout must not be
    // told the removal failed).
    let again = ctx
        .mcp_call_tool(&token, "remove_document_label", ids)
        .await;
    assert_ok(&again);
    assert_eq!(again["structuredContent"]["status"], json!("not_assigned"));
}

#[tokio::test]
async fn cannot_label_another_users_document() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = user_with_token(&ctx).await;
    let document_id = create_document(&ctx, &owner).await;
    let owner_label = create_label(&ctx, &owner_token, "Owner").await;
    assert_ok(
        &ctx.mcp_call_tool(
            &owner_token,
            "add_document_label",
            json!({ "document_id": document_id, "label_id": owner_label }),
        )
        .await,
    );

    let (_, intruder_token) = user_with_token(&ctx).await;
    let intruder_label = create_label(&ctx, &intruder_token, "Intruder").await;

    let add = ctx
        .mcp_call_tool(
            &intruder_token,
            "add_document_label",
            json!({ "document_id": document_id, "label_id": intruder_label }),
        )
        .await;
    assert_tool_error(&add);

    let get = ctx
        .mcp_call_tool(
            &intruder_token,
            "get_document_labels",
            json!({ "document_id": document_id }),
        )
        .await;
    assert_tool_error(&get);
    assert!(!text(&get).contains("Owner"));

    let remove = ctx
        .mcp_call_tool(
            &intruder_token,
            "remove_document_label",
            json!({ "document_id": document_id, "label_id": owner_label }),
        )
        .await;
    assert_tool_error(&remove);

    // The owner's assignment is untouched.
    let labels = ctx
        .mcp_call_tool(
            &owner_token,
            "get_document_labels",
            json!({ "document_id": document_id }),
        )
        .await;
    assert_eq!(label_names(&labels), vec!["Owner".to_string()]);
}

#[tokio::test]
async fn non_uuid_ids_are_rejected() {
    // rmcp reports argument deserialization failures as tool errors.
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;
    let result = ctx
        .mcp_call_tool(
            &token,
            "get_document_labels",
            json!({ "document_id": "not-a-uuid" }),
        )
        .await;
    assert_tool_error(&result);
    assert!(text(&result).contains("UUID"), "{result}");
}

#[tokio::test]
async fn renaming_to_a_taken_name_is_a_tool_error() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;
    create_label(&ctx, &token, "Taken").await;
    let label_id = create_label(&ctx, &token, "Free").await;

    let result = ctx
        .mcp_call_tool(
            &token,
            "update_label",
            json!({ "id": label_id, "name": "Taken" }),
        )
        .await;
    assert_tool_error(&result);
    assert!(text(&result).contains("already exists"), "{result}");
}

#[tokio::test]
async fn add_document_label_reports_every_outcome() {
    let ctx = TestContext::new().await;
    let (user, token) = user_with_token(&ctx).await;
    let label_id = create_label(&ctx, &token, "Mine").await;
    let document_id = create_document(&ctx, &user).await;

    let (owner, owner_token) = user_with_token(&ctx).await;
    let foreign_label = create_label(&ctx, &owner_token, "Theirs").await;
    let foreign_document = create_document(&ctx, &owner).await;

    let ids = json!({ "document_id": document_id, "label_id": label_id });
    let added = ctx
        .mcp_call_tool(&token, "add_document_label", ids.clone())
        .await;
    assert_ok(&added);
    assert_eq!(added["structuredContent"]["status"], json!("added"));
    let again = ctx.mcp_call_tool(&token, "add_document_label", ids).await;
    assert_ok(&again);
    assert_eq!(
        again["structuredContent"]["status"],
        json!("already_assigned")
    );

    // A missing or foreign document is reported as the document, even when the
    // label is foreign too.
    for (doc, label) in [
        (Uuid::new_v4(), label_id),
        (foreign_document, label_id),
        (foreign_document, foreign_label),
    ] {
        let result = ctx
            .mcp_call_tool(
                &token,
                "add_document_label",
                json!({ "document_id": doc, "label_id": label }),
            )
            .await;
        assert_tool_error(&result);
        assert!(
            text(&result).contains(&format!("Document {doc} not found")),
            "{result}"
        );
    }

    for label in [Uuid::new_v4(), foreign_label] {
        let result = ctx
            .mcp_call_tool(
                &token,
                "add_document_label",
                json!({ "document_id": document_id, "label_id": label }),
            )
            .await;
        assert_tool_error(&result);
        assert!(
            text(&result).contains(&format!("Label {label} not found")),
            "{result}"
        );
    }

    let labels = ctx.state.db.get_document_labels(document_id).await.unwrap();
    assert_eq!(
        labels.iter().map(|l| l.id).collect::<Vec<_>>(),
        vec![label_id]
    );
    assert!(ctx
        .state
        .db
        .get_document_labels(foreign_document)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn remove_document_label_reports_every_outcome() {
    let ctx = TestContext::new().await;
    let (user, token) = user_with_token(&ctx).await;
    let label_id = create_label(&ctx, &token, "Mine").await;
    let document_id = create_document(&ctx, &user).await;
    let ids = json!({ "document_id": document_id, "label_id": label_id });
    assert_ok(
        &ctx.mcp_call_tool(&token, "add_document_label", ids.clone())
            .await,
    );

    let missing = ctx
        .mcp_call_tool(
            &token,
            "remove_document_label",
            json!({ "document_id": Uuid::new_v4(), "label_id": label_id }),
        )
        .await;
    assert_tool_error(&missing);
    assert!(text(&missing).contains("not found"), "{missing}");

    let removed = ctx
        .mcp_call_tool(&token, "remove_document_label", ids.clone())
        .await;
    assert_ok(&removed);
    assert_eq!(removed["structuredContent"]["status"], json!("removed"));

    let not_assigned = ctx
        .mcp_call_tool(&token, "remove_document_label", ids)
        .await;
    assert_ok(&not_assigned);
    assert_eq!(
        not_assigned["structuredContent"]["status"],
        json!("not_assigned")
    );
}

/// Simultaneous add_document_label calls for one pair: exactly one adds the
/// label, the others report it already assigned, and none fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_add_document_label_adds_exactly_once() {
    const CALLS: usize = 16;
    let ctx = TestContext::new().await;
    let (user, token) = user_with_token(&ctx).await;
    let label_id = create_label(&ctx, &token, "Raced").await;
    let document_id = create_document(&ctx, &user).await;
    let ids = json!({ "document_id": document_id, "label_id": label_id });

    let results = futures::future::join_all(
        (0..CALLS).map(|_| ctx.mcp_call_tool(&token, "add_document_label", ids.clone())),
    )
    .await;
    let statuses: Vec<String> = results
        .iter()
        .map(|result| {
            assert_ok(result);
            result["structuredContent"]["status"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(
        statuses.iter().filter(|s| *s == "added").count(),
        1,
        "{statuses:?}"
    );
    assert_eq!(
        statuses.iter().filter(|s| *s == "already_assigned").count(),
        CALLS - 1,
        "{statuses:?}"
    );
}
