#!/usr/bin/env python3
"""
Enhanced script to migrate remaining tests in documents_tests.rs to use the new TestContext pattern.
"""

import re
import sys

def migrate_test_file(file_path):
    """Migrate the documents_tests.rs file to use new test patterns."""
    
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Store the original content for comparison
    original_content = content
    
    # Fix remaining documents_db references that were missed
    content = re.sub(r'        let result = documents_db', '        let result = ctx.state.db', content)
    content = re.sub(r'        let result2 = documents_db', '        let result2 = ctx.state.db', content)
    
    # Fix any remaining documents_db references in method calls
    content = re.sub(r'documents_db\n', 'ctx.state.db\n', content)
    
    # Fix variable naming from the document creation pattern
    # The regex replacement created variables like user_doc_doc, let's fix those
    content = re.sub(r'        let ([a-zA-Z0-9_]+)_doc_doc = create_test_document\(([^)]+)\);', 
                     r'        let \1_doc = create_test_document(\2);', content)
    
    # Check if we made any changes
    if content != original_content:
        return content
    else:
        return None

def main():
    file_path = '/root/repos/readur/src/tests/documents_tests.rs'
    
    print("Starting enhanced migration of documents_tests.rs...")
    
    migrated_content = migrate_test_file(file_path)
    
    if migrated_content:
        # Write the migrated content back
        with open(file_path, 'w') as f:
            f.write(migrated_content)
        print("Enhanced migration completed successfully!")
    else:
        print("No changes needed - file is already migrated or no patterns found.")

if __name__ == "__main__":
    main()