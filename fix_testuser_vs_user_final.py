#!/usr/bin/env python3
"""
Final comprehensive fix for TestUser vs models::User distinction
"""

import re
import sys

def fix_user_object_types(content):
    """Fix the distinction between TestUser and models::User objects"""
    
    lines = content.split('\n')
    fixed_lines = []
    
    # Track which variables are TestUser vs User objects
    testuser_vars = set()
    user_vars = set()
    
    for i, line in enumerate(lines):
        # Identify TestUser variables (created by auth_helper methods)
        if re.search(r'let (\w+) = auth_helper\.create_test_user\(\)', line):
            var_name = re.search(r'let (\w+) = auth_helper\.create_test_user\(\)', line).group(1)
            testuser_vars.add(var_name)
        elif re.search(r'let (\w+) = auth_helper\.create_admin_user\(\)', line):
            var_name = re.search(r'let (\w+) = auth_helper\.create_admin_user\(\)', line).group(1)
            testuser_vars.add(var_name)
        elif re.search(r'let (\w+) = auth_helper\.create_test_admin\(\)', line):
            var_name = re.search(r'let (\w+) = auth_helper\.create_test_admin\(\)', line).group(1)
            testuser_vars.add(var_name)
        
        # Identify models::User variables (created by db.create_user)
        elif re.search(r'let (\w+) = .*db\.create_user\(', line):
            var_name = re.search(r'let (\w+) = .*db\.create_user\(', line).group(1)
            user_vars.add(var_name)
        
        # Fix the line based on variable types
        fixed_line = line
        
        # For TestUser objects, ensure they use .user_response
        for var in testuser_vars:
            # Convert .id to .user_response.id for TestUser objects
            fixed_line = re.sub(rf'\b{var}\.id\b', f'{var}.user_response.id', fixed_line)
            # Convert .role to .user_response.role for TestUser objects
            fixed_line = re.sub(rf'\b{var}\.role\b', f'{var}.user_response.role', fixed_line)
        
        # For models::User objects, ensure they use direct access
        for var in user_vars:
            # Remove .user_response for User objects
            fixed_line = re.sub(rf'\b{var}\.user_response\.id\b', f'{var}.id', fixed_line)
            fixed_line = re.sub(rf'\b{var}\.user_response\.role\b', f'{var}.role', fixed_line)
        
        fixed_lines.append(fixed_line)
    
    return '\n'.join(fixed_lines)

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
    print("Applying comprehensive TestUser vs User fixes...")
    fixed_content = fix_user_object_types(content)
    
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