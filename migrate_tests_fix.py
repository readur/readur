#!/usr/bin/env python3
"""
Enhanced migration script to fix remaining issues in documents_tests.rs
"""

import re
import sys

def migrate_remaining_issues(content):
    """Fix remaining issues from the bulk migration"""
    
    # Fix remaining pool references
    content = re.sub(r'\.execute\(&pool\)', '.execute(&ctx.state.db.pool)', content)
    
    # Fix Database::new patterns - replace with TestContext
    database_new_pattern = r'let database = Database::new\(&connection_string\)\.await\.unwrap\(\);'
    database_new_replacement = 'let ctx = TestContext::new().await;\n        let database = &ctx.state.db;'
    content = re.sub(database_new_pattern, database_new_replacement, content)
    
    # Also handle the variable name 'database' in subsequent lines
    # Replace database. with ctx.state.db. only in test functions
    content = re.sub(r'\bdatabase\.', 'ctx.state.db.', content)
    
    # Fix cases where we have ctx declared multiple times in the same function
    # This is a more complex pattern - let's fix it by ensuring we only declare ctx once per function
    
    # Find functions with multiple ctx declarations and fix them
    def fix_multiple_ctx(match):
        func_content = match.group(0)
        # Count ctx declarations
        ctx_count = len(re.findall(r'let ctx = TestContext::new\(\)\.await;', func_content))
        if ctx_count > 1:
            # Keep only the first one, replace others with comments
            first_done = False
            def replace_ctx(ctx_match):
                nonlocal first_done
                if not first_done:
                    first_done = True
                    return ctx_match.group(0)
                else:
                    return '// let ctx = TestContext::new().await; // Already declared above'
            func_content = re.sub(r'let ctx = TestContext::new\(\)\.await;', replace_ctx, func_content)
        return func_content
    
    # Apply this to each test function
    func_pattern = r'#\[tokio::test\][^}]*?(?=\n    #\[tokio::test\]|\n}\n|\Z)'
    content = re.sub(func_pattern, fix_multiple_ctx, content, flags=re.MULTILINE | re.DOTALL)
    
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
    
    # Apply additional fixes
    print("Applying additional migration fixes...")
    migrated_content = migrate_remaining_issues(content)
    
    # Write back the migrated content
    try:
        with open(file_path, 'w') as f:
            f.write(migrated_content)
        print(f"Successfully applied fixes to {file_path}")
        return 0
    except Exception as e:
        print(f"Error writing file: {e}")
        return 1

if __name__ == '__main__':
    sys.exit(main())