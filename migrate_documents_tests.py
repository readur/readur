#!/usr/bin/env python3
"""
Script to migrate remaining tests in documents_tests.rs to use the new TestContext pattern.
"""

import re
import sys

def migrate_test_file(file_path):
    """Migrate the documents_tests.rs file to use new test patterns."""
    
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Store the original content for comparison
    original_content = content
    
    # 1. Remove #[ignore = "Requires PostgreSQL database"] annotations
    content = re.sub(r'    #\[ignore = "Requires PostgreSQL database"\]\n', '', content)
    
    # 2. Remove old database pool creation lines
    content = re.sub(r'        let pool = create_test_db_pool\(\)\.await;\n', '', content)
    
    # 3. Remove old Database struct creation lines
    content = re.sub(r'        let documents_db = Database \{ pool: pool\.clone\(\) \};\n', '', content)
    
    # 4. Replace old user creation with new pattern
    # Handle both User and Admin role patterns
    content = re.sub(
        r'        let user = create_test_user\(&pool, UserRole::User\)\.await;',
        '        let ctx = TestContext::new().await;\n        let auth_helper = TestAuthHelper::new(ctx.app.clone());\n        let user = auth_helper.create_test_user().await;',
        content
    )
    
    content = re.sub(
        r'        let admin = create_test_user\(&pool, UserRole::Admin\)\.await;',
        '        let admin = auth_helper.create_test_admin().await;',
        content
    )
    
    # Handle other variations of user creation
    content = re.sub(
        r'        let user1 = create_test_user\(&pool, UserRole::User\)\.await;',
        '        let ctx = TestContext::new().await;\n        let auth_helper = TestAuthHelper::new(ctx.app.clone());\n        let user1 = auth_helper.create_test_user().await;',
        content
    )
    
    content = re.sub(
        r'        let user2 = create_test_user\(&pool, UserRole::User\)\.await;',
        '        let user2 = auth_helper.create_test_user().await;',
        content
    )
    
    content = re.sub(
        r'        let tenant1_user1 = create_test_user\(&pool, UserRole::User\)\.await;',
        '        let ctx = TestContext::new().await;\n        let auth_helper = TestAuthHelper::new(ctx.app.clone());\n        let tenant1_user1 = auth_helper.create_test_user().await;',
        content
    )
    
    content = re.sub(
        r'        let tenant1_user2 = create_test_user\(&pool, UserRole::User\)\.await;',
        '        let tenant1_user2 = auth_helper.create_test_user().await;',
        content
    )
    
    content = re.sub(
        r'        let tenant2_user1 = create_test_user\(&pool, UserRole::User\)\.await;',
        '        let tenant2_user1 = auth_helper.create_test_user().await;',
        content
    )
    
    content = re.sub(
        r'        let tenant2_user2 = create_test_user\(&pool, UserRole::User\)\.await;',
        '        let tenant2_user2 = auth_helper.create_test_user().await;',
        content
    )
    
    # 5. Replace document creation and insertion pattern
    content = re.sub(
        r'        let ([a-zA-Z0-9_]+) = create_and_insert_test_document\(&pool, ([a-zA-Z0-9_.()]+)\)\.await;',
        r'        let \1_doc = create_test_document(\2);\n        let \1 = ctx.state.db.create_document(\1_doc).await.expect("Failed to create document");',
        content
    )
    
    # 6. Replace documents_db. with ctx.state.db.
    content = re.sub(r'documents_db\.', 'ctx.state.db.', content)
    
    # 7. Replace user.id with user.id() for TestUser instances
    # This is tricky because we need to be careful about which instances are TestUser vs regular User
    # We'll handle this pattern by pattern based on context
    
    # For delete_document calls that use user.id, user.role pattern
    content = re.sub(
        r'\.delete_document\(([^,]+), ([a-zA-Z0-9_]+)\.id, ([a-zA-Z0-9_]+)\.role\)',
        r'.delete_document(\1, \2.id(), \3.role)',
        content
    )
    
    # For bulk_delete_documents calls
    content = re.sub(
        r'\.bulk_delete_documents\(([^,]+), ([a-zA-Z0-9_]+)\.id, ([a-zA-Z0-9_]+)\.role\)',
        r'.bulk_delete_documents(\1, \2.id(), \3.role)',
        content
    )
    
    # For get_document_by_id calls
    content = re.sub(
        r'\.get_document_by_id\(([^,]+), ([a-zA-Z0-9_]+)\.id, ([a-zA-Z0-9_]+)\.role\)',
        r'.get_document_by_id(\1, \2.id(), \3.role)',
        content
    )
    
    # For create_test_document calls
    content = re.sub(
        r'create_test_document\(([a-zA-Z0-9_]+)\.id\)',
        r'create_test_document(\1.id())',
        content
    )
    
    # For bind calls in SQL
    content = re.sub(
        r'\.bind\(([a-zA-Z0-9_]+)\.id\)',
        r'.bind(\1.id())',
        content
    )
    
    # For let user_id assignments
    content = re.sub(
        r'        let user_id = ([a-zA-Z0-9_]+)\.id;',
        r'        let user_id = \1.id();',
        content
    )
    
    # Add missing imports if TestContext/TestAuthHelper aren't already imported
    # Check if the imports are present
    if 'use crate::test_utils::{TestContext, TestAuthHelper};' not in content:
        # Find the existing test_utils import and update it
        content = re.sub(
            r'use crate::test_utils::TestContext;',
            'use crate::test_utils::{TestContext, TestAuthHelper};',
            content
        )
    
    # Check if we made any changes
    if content != original_content:
        return content
    else:
        return None

def main():
    file_path = '/root/repos/readur/src/tests/documents_tests.rs'
    
    print("Starting migration of documents_tests.rs...")
    
    migrated_content = migrate_test_file(file_path)
    
    if migrated_content:
        # Write the migrated content back
        with open(file_path, 'w') as f:
            f.write(migrated_content)
        print("Migration completed successfully!")
    else:
        print("No changes needed - file is already migrated or no patterns found.")

if __name__ == "__main__":
    main()