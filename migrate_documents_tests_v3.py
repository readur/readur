#!/usr/bin/env python3
"""
Final cleanup script to fix variable naming issues from the migration.
"""

import re
import sys

def migrate_test_file(file_path):
    """Clean up variable naming issues from the migration."""
    
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Store the original content for comparison
    original_content = content
    
    # Fix the doubled variable names created by the regex
    content = re.sub(r'        let ([a-zA-Z0-9_]+)_doc_doc = create_test_document\(([^)]+)\);', 
                     r'        let \1_doc = create_test_document(\2);', content)
    
    # Also fix any references to these variables in the same context
    content = re.sub(r'create_document\(([a-zA-Z0-9_]+)_doc_doc\)', 
                     r'create_document(\1_doc)', content)
    
    # Check if we made any changes
    if content != original_content:
        return content
    else:
        return None

def main():
    file_path = '/root/repos/readur/src/tests/documents_tests.rs'
    
    print("Starting final cleanup of documents_tests.rs...")
    
    migrated_content = migrate_test_file(file_path)
    
    if migrated_content:
        # Write the migrated content back
        with open(file_path, 'w') as f:
            f.write(migrated_content)
        print("Final cleanup completed successfully!")
    else:
        print("No changes needed - file is already clean.")

if __name__ == "__main__":
    main()