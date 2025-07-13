#!/usr/bin/env python3
"""
Bulk migration script to convert old test patterns to new TestContext/TestAuthHelper patterns
in documents_tests.rs
"""

import re
import sys

def migrate_test_patterns(content):
    """Apply all migration patterns to the content"""
    
    # Remove #[ignore] attributes
    content = re.sub(r'\s*#\[ignore = "Requires PostgreSQL database"\]', '', content)
    
    # Pattern 1: Replace basic test setup
    # Old pattern:
    # let pool = create_test_db_pool().await;
    # let documents_db = Database { pool: pool.clone() };
    # New pattern:
    # let ctx = TestContext::new().await;
    # let auth_helper = TestAuthHelper::new(ctx.app.clone());
    
    pool_db_pattern = r'let pool = create_test_db_pool\(\)\.await;\s*let documents_db = Database \{ pool: pool\.clone\(\) \};'
    pool_db_replacement = 'let ctx = TestContext::new().await;\n        let auth_helper = TestAuthHelper::new(ctx.app.clone());'
    content = re.sub(pool_db_pattern, pool_db_replacement, content, flags=re.MULTILINE)
    
    # Pattern 2: Replace user creation
    # let user = create_test_user(&pool, UserRole::User).await;
    # -> let user = auth_helper.create_test_user().await;
    user_pattern = r'let (\w+) = create_test_user\(&pool, UserRole::User\)\.await;'
    user_replacement = r'let \1 = auth_helper.create_test_user().await;'
    content = re.sub(user_pattern, user_replacement, content)
    
    # Pattern 3: Replace admin creation
    # let admin = create_test_user(&pool, UserRole::Admin).await;
    # -> let admin = auth_helper.create_test_admin().await;
    admin_pattern = r'let (\w+) = create_test_user\(&pool, UserRole::Admin\)\.await;'
    admin_replacement = r'let \1 = auth_helper.create_test_admin().await;'
    content = re.sub(admin_pattern, admin_replacement, content)
    
    # Pattern 4: Replace document creation and insertion
    # let doc = create_and_insert_test_document(&pool, user.id).await;
    # -> let doc = create_test_document(user.id());
    #    let doc = ctx.state.db.create_document(doc).await.expect("Failed to create document");
    doc_pattern = r'let (\w+) = create_and_insert_test_document\(&pool, (\w+)\.id\)\.await;'
    def doc_replacement(match):
        doc_name = match.group(1)
        user_name = match.group(2)
        return f'let {doc_name} = create_test_document({user_name}.id());\n        let {doc_name} = ctx.state.db.create_document({doc_name}).await.expect("Failed to create document");'
    content = re.sub(doc_pattern, doc_replacement, content)
    
    # Pattern 5: Replace documents_db. with ctx.state.db.
    content = re.sub(r'documents_db\.', 'ctx.state.db.', content)
    
    # Pattern 6: Replace .id with .id() for user objects (be careful with document.id)
    # Only replace when it's clearly a user/admin object
    content = re.sub(r'(\w+)\.id(?![().])', r'\1.id()', content)
    
    # Fix document.id() back to document.id (documents don't have id() method)
    content = re.sub(r'(doc\w*)\.id\(\)', r'\1.id', content)
    content = re.sub(r'(document\w*)\.id\(\)', r'\1.id', content)
    content = re.sub(r'(\w*_doc\w*)\.id\(\)', r'\1.id', content)
    content = re.sub(r'(result\[\d+\])\.id\(\)', r'\1.id', content)
    content = re.sub(r'(deleted_doc)\.id\(\)', r'\1.id', content)
    content = re.sub(r'(found_doc\.unwrap\(\))\.id\(\)', r'\1.id', content)
    
    return content

def main():
    file_path = '/root/repos/readur/src/tests/documents_tests.rs'
    
    # Read the file
    try:
        with open(file_path, 'r') as f:
            content = f.read()
    except FileNotFoundError:
        print(f"Error: Could not find file {file_path}")
        return 1
    
    # Apply migrations
    print("Applying migration patterns...")
    migrated_content = migrate_test_patterns(content)
    
    # Write back the migrated content
    try:
        with open(file_path, 'w') as f:
            f.write(migrated_content)
        print(f"Successfully migrated {file_path}")
        return 0
    except Exception as e:
        print(f"Error writing file: {e}")
        return 1

if __name__ == '__main__':
    sys.exit(main())