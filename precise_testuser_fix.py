#!/usr/bin/env python3
"""
Precise fix for TestUser field access based on variable creation patterns
"""

import re
import sys

def fix_testuser_access(content):
    """Fix TestUser objects to use proper .user_response field access"""
    
    lines = content.split('\n')
    fixed_lines = []
    
    # Track which variables are TestUser objects within each function
    current_testuser_vars = set()
    in_function = False
    
    for line in lines:
        # Reset when entering a new function
        if re.match(r'\s*#\[tokio::test\]', line) or re.match(r'\s*async fn ', line):
            current_testuser_vars.clear()
            in_function = True
        elif re.match(r'^\s*}$', line) and in_function:
            in_function = False
            current_testuser_vars.clear()
        
        # Track TestUser variable declarations
        if in_function:
            # Variables created by auth_helper methods are TestUser
            testuser_match = re.search(r'let (\w+) = auth_helper\.(?:create_test_user|create_admin_user|create_test_admin)\(\)', line)
            if testuser_match:
                var_name = testuser_match.group(1)
                current_testuser_vars.add(var_name)
                print(f"Found TestUser variable: {var_name}")
        
        # Fix field access for known TestUser variables
        fixed_line = line
        for var_name in current_testuser_vars:
            # Replace .id with .user_response.id for TestUser objects
            fixed_line = re.sub(rf'\b{var_name}\.id\b', f'{var_name}.user_response.id', fixed_line)
            # Replace .role with .user_response.role for TestUser objects  
            fixed_line = re.sub(rf'\b{var_name}\.role\b', f'{var_name}.user_response.role', fixed_line)
        
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
    print("Applying precise TestUser field access fixes...")
    fixed_content = fix_testuser_access(content)
    
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