//! The registry of docs/api-reference.md request examples: for each example, who sends it,
//! what the server must hold first, which documented text stands for which real value, and
//! the schemas its request body is checked against. Used by
//! tests/integration_api_reference_docs_tests.rs, whose header describes the checks.

use axum::{
    body::Body,
    http::{Method, Request},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use utoipa::PartialSchema;
use uuid::Uuid;

use readur::models::{
    api_key::CreateApiKeyRequest,
    comment::CreateCommentRequest,
    label::{CreateLabel, LabelAssignment},
    shared_link::{CreateSharedLinkRequest, SharedLinkPasswordRequest},
    ChangePasswordRequest, CreateNotification, CreateSource, CreateUser, LoginRequest,
    RegisterRequest, UpdateUser, UserRole, WebDAVSourceConfig,
};
use readur::routes::documents::RetryOcrRequest;
use readur::test_utils::{document_helpers::create_test_document, TestConfigBuilder, TestContext};

/// Every example heading, with the test that checks it when that is not this registry.
pub const EXAMPLES: &[(&str, Option<&str>)] = &[
    ("Log In", None),
    ("Current User", None),
    ("Sign-in Options", None),
    ("Register", None),
    ("Change Own Password", None),
    ("Log Out", None),
    ("Error Response", None),
    ("Invalid Query Error", None),
    ("Invalid Request Body", None),
    ("Authentication Error", None),
    ("List Documents", None),
    ("Upload a Document", None),
    ("Get OCR Text", None),
    ("Retry OCR", None),
    ("Retry OCR While Already Queued", None),
    ("Delete a Document", None),
    ("Search Documents", Some(SEARCH_TEST)),
    ("Enhanced Search", Some(SEARCH_TEST)),
    ("Search Timeline", Some(SEARCH_TEST)),
    ("Search Facets", Some(SEARCH_TEST)),
    ("Create a Label", None),
    ("Set a Document's Labels", None),
    ("Notification Summary", None),
    ("Queue Statistics", None),
    ("Create a Source", None),
    ("Create a User", None),
    ("Activate a User", None),
    ("Health Check", None),
    ("Create an API Key", None),
    ("List API Keys", None),
    ("Revoke an API Key", None),
    ("Create a Shared Link", None),
    ("Get Shared Document Metadata", None),
    ("Verify a Shared Link Password", None),
    ("Create a Comment", None),
    ("Count Comments", None),
];

const SEARCH_TEST: &str = "integration_api_reference_search_docs_tests";

/// The ids the reference writes in request bodies, mapped to real ids by the setups.
pub const DOC_DOCUMENT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
pub const DOC_LABEL_ID: &str = "6f1c2a8e-3b7d-4c1e-9a2f-0d4b5e6f7a8b";

/// The documented passwords of the fixture accounts and links.
pub const ALICE_PASSWORD: &str = "alice-password-1";
pub const LINK_PASSWORD: &str = "open-sesame";

#[derive(Debug, Clone, Copy)]
pub enum Who {
    Anonymous,
    Alice,
    Admin,
}

pub struct Fixture {
    pub ctx: TestContext,
    pub alice: Uuid,
    alice_token: String,
    admin_token: String,
}

impl Fixture {
    /// A fresh server for the example under `heading`. Registration needs administrator
    /// approval there, as on every production server.
    pub async fn new(heading: &str) -> Self {
        let config =
            TestConfigBuilder::default().with_registration_requires_approval(heading == "Register");
        let ctx = TestContext::with_config(config).await;
        let db = &ctx.state().db;
        let account = |name: &str, password: &str, role| CreateUser {
            username: name.to_string(),
            email: format!("{name}@example.com"),
            password: password.to_string(),
            role: Some(role),
        };
        let alice = db
            .create_user(account("alice", ALICE_PASSWORD, UserRole::User))
            .await
            .unwrap()
            .id;
        db.create_user(account("admin", "admin-password-1", UserRole::Admin))
            .await
            .unwrap();
        let auth = ctx.auth_helper();
        let alice_token = auth.login_user("alice", ALICE_PASSWORD).await;
        let admin_token = auth.login_user("admin", "admin-password-1").await;
        Self {
            ctx,
            alice,
            alice_token,
            admin_token,
        }
    }

    pub fn token(&self, who: Who) -> Option<&str> {
        match who {
            Who::Anonymous => None,
            Who::Alice => Some(&self.alice_token),
            Who::Admin => Some(&self.admin_token),
        }
    }

    /// Sends a JSON request and returns the JSON response, asserting a 2xx status.
    async fn call(&self, who: Who, method: &str, uri: &str, body: Value) -> Value {
        let mut request = Request::builder()
            .method(Method::from_bytes(method.as_bytes()).unwrap())
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(token) = self.token(who) {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let request = request.body(Body::from(body.to_string())).unwrap();
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(
            status.is_success(),
            "setup {method} {uri}: {status} {}",
            String::from_utf8_lossy(&bytes)
        );
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    }

    /// Alice's processed PDF "invoice-2024-03.pdf".
    async fn document(&self) -> Uuid {
        let mut doc = create_test_document(self.alice);
        doc.filename = "invoice-2024-03.pdf".to_string();
        doc.original_filename = "invoice-2024-03.pdf".to_string();
        doc.file_size = 48213;
        doc.content = None;
        doc.ocr_text = Some("Invoice 2024-03 Total due: 120.00 EUR".to_string());
        doc.ocr_word_count = Some(6);
        doc.ocr_processing_time_ms = Some(840);
        doc.tags = Vec::new();
        doc.created_at = "2024-03-10T09:00:00Z".parse().unwrap();
        doc.updated_at = doc.created_at;
        doc.ocr_completed_at = Some(doc.created_at);
        doc.file_hash = Some(format!("{:064x}", Uuid::new_v4().as_u128()));
        self.ctx.state().db.create_document(doc).await.unwrap().id
    }

    /// Alice's label "Invoices".
    async fn label(&self) -> Uuid {
        let label = self
            .call(
                Who::Alice,
                "POST",
                "/api/labels",
                json!({"name": "Invoices", "color": "#0969da"}),
            )
            .await;
        label["id"].as_str().unwrap().parse().unwrap()
    }

    /// A shared link to `document`, optionally password-protected; returns its token.
    async fn shared_link(&self, document: Uuid, password: Option<&str>) -> String {
        let link = self
            .call(
                Who::Alice,
                "POST",
                "/api/shared/links",
                json!({"document_id": document, "password": password}),
            )
            .await;
        link["token"].as_str().unwrap().to_string()
    }
}

/// What an example needs (see the module docs).
pub struct Setup {
    pub who: Who,
    pub subst: Vec<(String, String)>,
    pub multipart: Option<(String, String, Vec<u8>)>,
    pub request_schemas: Vec<(&'static str, Value)>,
}

impl Setup {
    fn new(who: Who) -> Self {
        Self {
            who,
            subst: Vec::new(),
            multipart: None,
            request_schemas: Vec::new(),
        }
    }

    fn subst(mut self, from: &str, to: impl ToString) -> Self {
        self.subst.push((from.to_string(), to.to_string()));
        self
    }

    fn schema<T: PartialSchema>(mut self, pointer: &'static str) -> Self {
        self.request_schemas
            .push((pointer, serde_json::to_value(T::schema()).unwrap()));
        self
    }
}

/// The setup of the example under `heading`, after preparing the server for it.
pub async fn setup(heading: &str, fx: &Fixture) -> Setup {
    use Who::*;
    match heading {
        "Log In" | "Authentication Error" => Setup::new(Anonymous).schema::<LoginRequest>(""),
        "Current User" | "Log Out" => Setup::new(Alice),
        "Sign-in Options" | "Health Check" => Setup::new(Anonymous),
        "Register" => Setup::new(Anonymous).schema::<RegisterRequest>(""),
        "Change Own Password" => Setup::new(Alice).schema::<ChangePasswordRequest>(""),
        "Error Response" => Setup::new(Alice).subst("{id}", Uuid::new_v4()),
        "Invalid Query Error" => Setup::new(Alice),
        "Invalid Request Body" => Setup::new(Alice).schema::<CreateLabel>(""),
        "List Documents" => {
            fx.document().await;
            Setup::new(Alice)
        }
        "Upload a Document" => {
            let mut setup = Setup::new(Alice);
            setup.multipart = Some((
                "receipt.txt".into(),
                "text/plain".into(),
                b"Coffee 3.20 EUR\n".to_vec(),
            ));
            setup
        }
        "Get OCR Text" | "Delete a Document" => {
            Setup::new(Alice).subst("{id}", fx.document().await)
        }
        "Retry OCR" => Setup::new(Alice)
            .subst("{id}", fx.document().await)
            .schema::<RetryOcrRequest>(""),
        "Retry OCR While Already Queued" => {
            let id = fx.document().await;
            let uri = format!("/api/documents/{id}/ocr/retry");
            fx.call(Alice, "POST", &uri, json!({})).await;
            Setup::new(Alice)
                .subst("{id}", id)
                .schema::<RetryOcrRequest>("")
        }
        "Create a Label" => Setup::new(Alice).schema::<CreateLabel>(""),
        "Set a Document's Labels" => Setup::new(Alice)
            .subst("{document_id}", fx.document().await)
            .subst(DOC_LABEL_ID, fx.label().await)
            .schema::<LabelAssignment>(""),
        "Notification Summary" => {
            let notification = CreateNotification {
                notification_type: "success".to_string(),
                title: "Sync complete".to_string(),
                message: "Office NAS: 12 new documents".to_string(),
                action_url: Some("/sources".to_string()),
                metadata: None,
            };
            fx.ctx
                .state()
                .db
                .create_notification(fx.alice, &notification)
                .await
                .unwrap();
            Setup::new(Alice)
        }
        "Queue Statistics" => Setup::new(Admin),
        "Create a Source" => Setup::new(Alice)
            .schema::<CreateSource>("")
            .schema::<WebDAVSourceConfig>("/config"),
        "Create a User" => Setup::new(Admin).schema::<CreateUser>(""),
        "Activate a User" => {
            let user = CreateUser {
                username: "bob".to_string(),
                email: "bob@example.com".to_string(),
                password: "bob-password-1".to_string(),
                role: Some(UserRole::User),
            };
            let bob = fx
                .ctx
                .state()
                .db
                .create_user_with_status(user, false)
                .await
                .unwrap()
                .id;
            Setup::new(Admin)
                .subst("{id}", bob)
                .schema::<UpdateUser>("")
        }
        "Create an API Key" => Setup::new(Alice).schema::<CreateApiKeyRequest>(""),
        "List API Keys" => {
            fx.call(
                Alice,
                "POST",
                "/api/auth/keys",
                json!({"name": "backup-script", "expires_in_days": 90}),
            )
            .await;
            Setup::new(Alice)
        }
        "Revoke an API Key" => {
            let key = fx
                .call(
                    Alice,
                    "POST",
                    "/api/auth/keys",
                    json!({"name": "backup-script"}),
                )
                .await;
            Setup::new(Alice).subst("{id}", key["api_key"]["id"].as_str().unwrap())
        }
        "Create a Shared Link" => Setup::new(Alice)
            .subst(DOC_DOCUMENT_ID, fx.document().await)
            .schema::<CreateSharedLinkRequest>(""),
        "Get Shared Document Metadata" => {
            let token = fx.shared_link(fx.document().await, None).await;
            Setup::new(Anonymous).subst("{token}", token)
        }
        "Verify a Shared Link Password" => {
            let token = fx
                .shared_link(fx.document().await, Some(LINK_PASSWORD))
                .await;
            Setup::new(Anonymous)
                .subst("{token}", token)
                .schema::<SharedLinkPasswordRequest>("")
        }
        "Create a Comment" => Setup::new(Alice)
            .subst("{document_id}", fx.document().await)
            .schema::<CreateCommentRequest>(""),
        "Count Comments" => {
            let document = fx.document().await;
            let uri = format!("/api/comments/documents/{document}/comments");
            fx.call(Alice, "POST", &uri, json!({"content": "Paid on 12 March."}))
                .await;
            Setup::new(Alice).subst("{document_id}", document)
        }
        _ => panic!(
            "no setup for example {heading:?}; add it to tests/support/api_reference_examples.rs"
        ),
    }
}
