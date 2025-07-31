#\!/bin/bash

# List of test files that need Config struct fixes
files=(
"tests/integration_config_oidc_tests.rs"
"tests/integration_source_sync_cancellation_workflow_tests.rs"
"tests/integration_source_scheduler_tests.rs"
"tests/integration_webdav_hash_duplicate_tests.rs"
"tests/integration_stop_sync_functionality_tests.rs"
"tests/integration_universal_source_sync_tests.rs"
)

for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        echo "Fixing $file..."
        # Use sed to add the missing fields after watch_folder line
        sed -i.bak '/watch_folder: /a\
        user_watch_base_dir: "./user_watch".to_string(),\
        enable_per_user_watch: false,' "$file"
        
        # Remove backup file
        rm "${file}.bak" 2>/dev/null || true
        echo "Fixed $file"
    else
        echo "File not found: $file"
    fi
done
