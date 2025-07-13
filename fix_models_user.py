#!/usr/bin/env python3
"""
Fix models::User objects that were incorrectly converted to use .user_response
"""

import re
import sys

def fix_models_user(content):
    """Fix models::User objects that were incorrectly converted"""
    
    # Find all lines that create models::User objects via db.create_user()
    # and track the variable names
    user_vars = set()
    
    lines = content.split('\n')
    for line in lines:
        if 'db.create_user(' in line and 'await' in line:
            # This creates a models::User object
            match = re.search(r'let (\w+) = .*db\.create_user\(', line)
            if match:
                user_vars.add(match.group(1))
    
    # Now fix all references to these variables
    for var in user_vars:
        # Revert .user_response.id back to .id
        content = content.replace(f'{var}.user_response.id', f'{var}.id')
        # Revert .user_response.role back to .role
        content = content.replace(f'{var}.user_response.role', f'{var}.role')
    
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
    print("Fixing models::User objects...")
    fixed_content = fix_models_user(content)
    
    # Write back the fixed content
    try:
        with open(file_path, 'w') as f:
            f.write(fixed_content)
        print(f"Successfully fixed {file_path}")
        return 0
    except Exception as e:
        print(f"Error writing file: {e}")
        return 1

if __name__ == '__main__':
    sys.exit(main())