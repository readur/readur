#\!/bin/bash

# Files with missing Config fields
config_files=(
"tests/integration_smart_sync_targeted_scan.rs"
"tests/integration_s3_sync_tests.rs"
"tests/unit_webdav_edge_cases_tests.rs"
"tests/unit_webdav_url_management_tests.rs"
"tests/integration_webdav_concurrency_tests.rs"
"tests/integration_smart_sync_error_handling.rs"
"tests/integration_webdav_sync_tests.rs"
"tests/unit_webdav_directory_tracking_tests.rs"
"tests/unit_basic_sync_tests.rs"
"tests/unit_webdav_unit_tests.rs"
"tests/integration_webdav_smart_scanning_tests.rs"
"tests/unit_webdav_enhanced_unit_tests.rs"
"tests/integration_webdav_scheduler_concurrency_tests.rs"
"tests/integration_smart_sync_deep_scan.rs"
"tests/integration_smart_sync_no_changes.rs"
"tests/integration_local_folder_sync_tests.rs"
"tests/webdav_production_flow_integration_tests.rs"
"tests/integration_webdav_first_time_scan_tests.rs"
"tests/unit_webdav_targeted_rescan_tests.rs"
"tests/integration_smart_sync_first_time.rs"
"tests/unit_smart_sync_service_tests.rs"
"tests/unit_webdav_smart_scan_logic_tests.rs"
)

echo "Fixing Config structs in ${#config_files[@]} files..."

for file in "${config_files[@]}"; do
    if [ -f "$file" ]; then
        echo "Processing $file..."
        # Check if file has Config struct and missing fields
        if grep -q "Config {" "$file" && \! grep -q "user_watch_base_dir" "$file"; then
            # Add the missing fields after watch_folder line
            sed -i.bak '/watch_folder:/a\
        user_watch_base_dir: "./user_watch".to_string(),\
        enable_per_user_watch: false,' "$file"
            
            # Clean up formatting
            sed -i 's/enable_per_user_watch: false,            /enable_per_user_watch: false,\n        /' "$file"
            sed -i 's/enable_per_user_watch: false,        /enable_per_user_watch: false,\n        /' "$file"
            
            # Remove backup
            rm "${file}.bak" 2>/dev/null || true
            echo "Fixed Config in $file"
        else
            echo "Skipping $file (no Config struct or already fixed)"
        fi
    else
        echo "File not found: $file"
    fi
done

echo "Config fixes completed."
