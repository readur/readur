#!/bin/bash

# WebDAV Test Data Generation Script
# Generates complex directory structures for stress testing WebDAV sync functionality

set -euo pipefail

# Default values
WEBDAV_ROOT=""
STRESS_LEVEL="medium"
INCLUDE_GIT_REPOS=false
INCLUDE_PERMISSION_ISSUES=false
INCLUDE_SYMLINKS=false
INCLUDE_LARGE_DIRECTORIES=false
INCLUDE_UNICODE_NAMES=false
INCLUDE_PROBLEMATIC_FILES=false

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 --webdav-root <path> [options]

Required:
  --webdav-root <path>          Root directory for WebDAV test data

Options:
  --stress-level <level>        Stress test level: light, medium, heavy, extreme (default: medium)
  --include-git-repos           Include Git repository structures
  --include-permission-issues   Include files with permission problems
  --include-symlinks            Include symbolic links
  --include-large-directories   Include directories with many files
  --include-unicode-names       Include files with Unicode names
  --include-problematic-files   Include files with problematic names
  -h, --help                    Show this help message

Example:
  $0 --webdav-root /tmp/webdav-test --stress-level heavy --include-symlinks --include-unicode-names
EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --webdav-root)
            WEBDAV_ROOT="$2"
            shift 2
            ;;
        --stress-level)
            STRESS_LEVEL="$2"
            shift 2
            ;;
        --include-git-repos)
            INCLUDE_GIT_REPOS=true
            shift
            ;;
        --include-permission-issues)
            INCLUDE_PERMISSION_ISSUES=true
            shift
            ;;
        --include-symlinks)
            INCLUDE_SYMLINKS=true
            shift
            ;;
        --include-large-directories)
            INCLUDE_LARGE_DIRECTORIES=true
            shift
            ;;
        --include-unicode-names)
            INCLUDE_UNICODE_NAMES=true
            shift
            ;;
        --include-problematic-files)
            INCLUDE_PROBLEMATIC_FILES=true
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            echo "Error: Unknown option $1" >&2
            show_usage
            exit 1
            ;;
    esac
done

# Validate required parameters
if [[ -z "$WEBDAV_ROOT" ]]; then
    echo "Error: --webdav-root is required" >&2
    show_usage
    exit 1
fi

# Validate stress level
case "$STRESS_LEVEL" in
    light|medium|heavy|extreme)
        ;;
    *)
        echo "Error: Invalid stress level '$STRESS_LEVEL'. Must be: light, medium, heavy, or extreme" >&2
        exit 1
        ;;
esac

echo "Generating WebDAV test data..."
echo "Root directory: $WEBDAV_ROOT"
echo "Stress level: $STRESS_LEVEL"
echo "Git repos: $INCLUDE_GIT_REPOS"
echo "Permission issues: $INCLUDE_PERMISSION_ISSUES"
echo "Symlinks: $INCLUDE_SYMLINKS"
echo "Large directories: $INCLUDE_LARGE_DIRECTORIES"
echo "Unicode names: $INCLUDE_UNICODE_NAMES"
echo "Problematic files: $INCLUDE_PROBLEMATIC_FILES"

# Create root directory
mkdir -p "$WEBDAV_ROOT"
cd "$WEBDAV_ROOT"

# Set parameters based on stress level
case "$STRESS_LEVEL" in
    light)
        MAX_DEPTH=3
        FILES_PER_DIR=5
        DIRS_PER_LEVEL=3
        LARGE_DIR_SIZE=20
        ;;
    medium)
        MAX_DEPTH=5
        FILES_PER_DIR=10
        DIRS_PER_LEVEL=5
        LARGE_DIR_SIZE=50
        ;;
    heavy)
        MAX_DEPTH=8
        FILES_PER_DIR=20
        DIRS_PER_LEVEL=8
        LARGE_DIR_SIZE=100
        ;;
    extreme)
        MAX_DEPTH=12
        FILES_PER_DIR=50
        DIRS_PER_LEVEL=10
        LARGE_DIR_SIZE=500
        ;;
esac

echo "Configuration: max_depth=$MAX_DEPTH, files_per_dir=$FILES_PER_DIR, dirs_per_level=$DIRS_PER_LEVEL"

# Function to create a file with content
create_test_file() {
    local filepath="$1"
    local content="$2"
    
    mkdir -p "$(dirname "$filepath")"
    echo "$content" > "$filepath"
    echo "$(date): Test file created at $filepath" >> "$filepath"
}

# Function to create directory structure recursively
create_directory_structure() {
    local base_path="$1"
    local current_depth="$2"
    local max_depth="$3"
    local prefix="$4"
    
    if [[ $current_depth -ge $max_depth ]]; then
        return
    fi
    
    mkdir -p "$base_path"
    
    # Create files in current directory
    for ((i=1; i<=FILES_PER_DIR; i++)); do
        local filename="${prefix}_file_${i}.txt"
        create_test_file "$base_path/$filename" "Test file $i in $base_path (depth $current_depth)"
    done
    
    # Create subdirectories
    for ((i=1; i<=DIRS_PER_LEVEL; i++)); do
        local dirname="${prefix}_subdir_${i}"
        create_directory_structure "$base_path/$dirname" $((current_depth + 1)) $max_depth "${prefix}_${i}"
    done
}

# Create main structure
echo "Creating main directory structure..."
create_directory_structure "main-structure" 0 $MAX_DEPTH "main"

# Create documents structure
echo "Creating documents structure..."
mkdir -p docs-structure
create_test_file "docs-structure/README.md" "# Test Documents\nThis is a test document repository."
create_test_file "docs-structure/manual.pdf" "Fake PDF content for testing"
create_test_file "docs-structure/presentation.pptx" "Fake PowerPoint content"

# Create images structure
echo "Creating images structure..."
mkdir -p images-structure
for i in {1..10}; do
    create_test_file "images-structure/image_${i}.jpg" "Fake JPEG image content $i"
    create_test_file "images-structure/photo_${i}.png" "Fake PNG image content $i"
done

# Create potential loop trap directories
echo "Creating loop trap directories..."
mkdir -p loop-traps/deep-nesting
create_directory_structure "loop-traps/deep-nesting" 0 $((MAX_DEPTH + 2)) "loop"

# Create test repositories if requested
if [[ "$INCLUDE_GIT_REPOS" == "true" ]]; then
    echo "Creating Git repository structures..."
    
    for i in {1..3}; do
        repo_dir="test-repo-$i"
        mkdir -p "$repo_dir"
        cd "$repo_dir"
        
        # Initialize git repo (but don't actually use git to avoid dependency)
        mkdir -p .git/objects .git/refs/heads .git/refs/tags
        echo "ref: refs/heads/main" > .git/HEAD
        
        # Create typical git repo structure
        create_test_file "src/main.rs" "fn main() { println!(\"Hello, world!\"); }"
        create_test_file "Cargo.toml" "[package]\nname = \"test-repo-$i\"\nversion = \"0.1.0\""
        create_test_file "README.md" "# Test Repository $i"
        
        cd "$WEBDAV_ROOT"
    done
fi

# Create large directories if requested
if [[ "$INCLUDE_LARGE_DIRECTORIES" == "true" ]]; then
    echo "Creating large directories..."
    
    mkdir -p large-directory
    for ((i=1; i<=LARGE_DIR_SIZE; i++)); do
        create_test_file "large-directory/file_$(printf "%04d" $i).txt" "Content of file $i in large directory"
    done
fi

# Create symlinks if requested
if [[ "$INCLUDE_SYMLINKS" == "true" ]]; then
    echo "Creating symbolic links..."
    
    mkdir -p symlink-test
    create_test_file "symlink-test/target.txt" "This is the target file"
    
    # Create various types of symlinks
    cd symlink-test
    ln -sf target.txt link_to_file.txt
    ln -sf ../main-structure link_to_dir
    ln -sf nonexistent.txt broken_link.txt
    ln -sf link_to_file.txt link_to_link.txt  # Link to link
    cd "$WEBDAV_ROOT"
fi

# Create Unicode filenames if requested
if [[ "$INCLUDE_UNICODE_NAMES" == "true" ]]; then
    echo "Creating files with Unicode names..."
    
    mkdir -p unicode-test
    create_test_file "unicode-test/café.txt" "French café file"
    create_test_file "unicode-test/résumé.pdf" "French résumé file"
    create_test_file "unicode-test/日本語.txt" "Japanese filename"
    create_test_file "unicode-test/emoji_😀.txt" "File with emoji"
    create_test_file "unicode-test/математика.doc" "Russian filename"
fi

# Create problematic files if requested
if [[ "$INCLUDE_PROBLEMATIC_FILES" == "true" ]]; then
    echo "Creating problematic files..."
    
    mkdir -p problematic-files
    
    # Files with special characters (properly escaped)
    create_test_file "problematic-files/file with spaces.txt" "File with spaces in name"
    create_test_file "problematic-files/file&with&ampersands.txt" "File with ampersands"
    create_test_file "problematic-files/file[with]brackets.txt" "File with brackets"
    create_test_file "problematic-files/file'with'quotes.txt" "File with single quotes"
    create_test_file 'problematic-files/file"with"doublequotes.txt' "File with double quotes"
    
    # Very long filename
    long_name=$(printf 'very_long_filename_%.0s' {1..20})
    create_test_file "problematic-files/${long_name}.txt" "File with very long name"
    
    # File with just dots
    create_test_file "problematic-files/...txt" "File starting with dots"
fi

# Create restricted access files if requested
if [[ "$INCLUDE_PERMISSION_ISSUES" == "true" ]]; then
    echo "Creating permission test files..."
    
    mkdir -p restricted-access
    create_test_file "restricted-access/readonly.txt" "Read-only file"
    create_test_file "restricted-access/normal.txt" "Normal file"
    
    # Make one file read-only
    chmod 444 "restricted-access/readonly.txt"
    
    # Create a directory with restricted permissions
    mkdir -p restricted-access/restricted-dir
    create_test_file "restricted-access/restricted-dir/hidden.txt" "Hidden file"
    chmod 700 "restricted-access/restricted-dir"
fi

# Create summary file
echo "Creating test data summary..."
create_test_file "TEST_DATA_SUMMARY.txt" "WebDAV Test Data Summary
Generated: $(date)
Stress Level: $STRESS_LEVEL
Configuration:
- Max Depth: $MAX_DEPTH
- Files per Directory: $FILES_PER_DIR
- Directories per Level: $DIRS_PER_LEVEL
- Large Directory Size: $LARGE_DIR_SIZE

Features Included:
- Git Repos: $INCLUDE_GIT_REPOS
- Permission Issues: $INCLUDE_PERMISSION_ISSUES
- Symlinks: $INCLUDE_SYMLINKS
- Large Directories: $INCLUDE_LARGE_DIRECTORIES
- Unicode Names: $INCLUDE_UNICODE_NAMES
- Problematic Files: $INCLUDE_PROBLEMATIC_FILES

Total files created: $(find . -type f | wc -l)
Total directories created: $(find . -type d | wc -l)
"

echo "WebDAV test data generation completed!"
echo "Root directory: $WEBDAV_ROOT"
echo "Total files: $(find "$WEBDAV_ROOT" -type f | wc -l)"
echo "Total directories: $(find "$WEBDAV_ROOT" -type d | wc -l)"

# Display directory structure summary
echo ""
echo "Directory structure summary:"
find "$WEBDAV_ROOT" -type d | head -20
if [[ $(find "$WEBDAV_ROOT" -type d | wc -l) -gt 20 ]]; then
    echo "... and $(($(find "$WEBDAV_ROOT" -type d | wc -l) - 20)) more directories"
fi