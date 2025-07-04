#!/usr/bin/env python3
"""
Final script to fix all remaining issues in documents_tests.rs
"""

import re
import sys

def fix_documents_tests(content):
    """Fix all remaining issues in documents_tests.rs"""
    
    # Fix 1: Replace user.id() with user.user_response.id (for TestUser objects)
    # This converts String to Uuid properly
    content = re.sub(r'(\w+)\.id\(\)', r'\1.user_response.id', content)
    
    # Fix 2: Replace user.role with user.user_response.role (for TestUser objects)
    content = re.sub(r'(\w+)\.role\b', r'\1.user_response.role', content)
    
    # Fix 3: Replace create_test_admin() with create_admin_user()
    content = re.sub(r'\.create_test_admin\(\)', '.create_admin_user()', content)
    
    # Fix 4: Fix document.id() back to document.id (documents don't have id() method)
    content = re.sub(r'(doc\w*|document\w*|result\[\d+\]|deleted_doc|found_doc\.unwrap\(\))\.user_response\.id\b', r'\1.id', content)
    
    # Fix 5: Fix response.id() to response.id for DocumentResponse
    content = re.sub(r'response\.user_response\.id\b', 'response.id', content)
    
    # Fix 6: Fix any standalone .user_response.id calls that shouldn't be there
    content = re.sub(r'\.user_response\.id\(\)', '.user_response.id', content)
    
    # Fix 7: Fix doubled "user_response" patterns
    content = re.sub(r'\.user_response\.user_response\.', '.user_response.', content)
    
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
    
    # Apply fixes
    print("Applying final fixes to documents_tests.rs...")
    fixed_content = fix_documents_tests(content)
    
    # Write back the fixed content
    try:
        with open(file_path, 'w') as f:
            f.write(fixed_content)
        print(f"Successfully applied fixes to {file_path}")
        return 0
    except Exception as e:
        print(f"Error writing file: {e}")
        return 1

if __name__ == '__main__':
    sys.exit(main())