#!/usr/bin/env python3
"""
Fix the distinction between models::User and TestUser objects
"""

import re
import sys

def fix_user_types(content):
    """Fix the distinction between models::User and TestUser objects"""
    
    # First, find all the places where we import or create Users vs TestUsers
    # and fix them appropriately
    
    # In the test functions, we need to identify which variables are TestUser and which are User
    # Let's look for patterns that indicate TestUser creation
    
    # Pattern 1: Variables created from auth_helper.create_test_user() are TestUser
    # Pattern 2: Variables created from auth_helper.create_admin_user() are TestUser
    # Pattern 3: Variables created from auth_helper.create_test_admin() are TestUser
    
    # Find all test functions and fix them individually
    test_functions = re.findall(r'(#\[tokio::test\].*?^    })', content, re.MULTILINE | re.DOTALL)
    
    for func in test_functions:
        # Check if this function creates TestUser objects
        if 'auth_helper.create_test_user()' in func or 'auth_helper.create_admin_user()' in func or 'auth_helper.create_test_admin()' in func:
            # This function uses TestUser objects, keep .user_response
            continue
        else:
            # This function might be using models::User objects, revert .user_response
            # But only if the variable is clearly a User object
            func_lines = func.split('\n')
            for i, line in enumerate(func_lines):
                # Look for variable declarations that create User objects
                if 'create_test_user(&' in line and 'UserRole::' in line:
                    # This creates a models::User object
                    var_match = re.search(r'let (\w+) = create_test_user\(', line)
                    if var_match:
                        var_name = var_match.group(1)
                        # Replace .user_response with direct access for this variable
                        func = func.replace(f'{var_name}.user_response.id', f'{var_name}.id')
                        func = func.replace(f'{var_name}.user_response.role', f'{var_name}.role')
    
    # Apply the fixed functions back to content
    # This is complex, so let's use a different approach
    
    # Let's be more specific about which variables are TestUser vs User
    # Look for the specific patterns in the migration
    
    # Fix models::User objects that got incorrectly converted
    # Pattern: Variables that are clearly User objects (not TestUser)
    lines = content.split('\n')
    in_test_function = False
    current_function_uses_testuser = False
    
    fixed_lines = []
    
    for line in lines:
        if '#[tokio::test]' in line:
            in_test_function = True
            current_function_uses_testuser = False
        elif in_test_function and line.strip() == '}':
            in_test_function = False
            current_function_uses_testuser = False
        elif in_test_function and ('auth_helper.create_test_user()' in line or 'auth_helper.create_admin_user()' in line or 'auth_helper.create_test_admin()' in line):
            current_function_uses_testuser = True
        elif in_test_function and not current_function_uses_testuser:
            # This function doesn't use TestUser objects, so revert .user_response
            # But only for variables that are created with the old pattern
            if 'create_test_user(&' in line and 'UserRole::' in line:
                # This line creates a models::User object
                var_match = re.search(r'let (\w+) = create_test_user\(', line)
                if var_match:
                    var_name = var_match.group(1)
                    # Mark this variable as a User object
                    # We'll fix its usage in subsequent lines
                    pass
            # Fix usage of User objects
            line = re.sub(r'(\w+)\.user_response\.id\b', r'\1.id', line)
            line = re.sub(r'(\w+)\.user_response\.role\b', r'\1.role', line)
        
        fixed_lines.append(line)
    
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
    print("Fixing User vs TestUser distinction...")
    fixed_content = fix_user_types(content)
    
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