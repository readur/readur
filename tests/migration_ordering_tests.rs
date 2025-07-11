use sqlx::PgPool;
use std::path::Path;
use std::fs;

#[cfg(test)]
mod migration_ordering_tests {
    use super::*;

    #[test]
    fn test_migration_files_have_unique_timestamps() {
        let migration_files = get_migration_files();
        let mut timestamps = Vec::new();
        
        for file in &migration_files {
            let timestamp = extract_timestamp(&file);
            assert!(
                !timestamps.contains(&timestamp),
                "Duplicate migration timestamp found: {} in file {}",
                timestamp, file
            );
            timestamps.push(timestamp);
        }
        
        println!("✅ All migration files have unique timestamps");
    }

    #[test]
    fn test_migration_files_are_chronologically_ordered() {
        let migration_files = get_migration_files();
        let mut timestamps: Vec<u64> = migration_files.iter()
            .map(|f| extract_timestamp(f).parse::<u64>().unwrap())
            .collect();
        
        let mut sorted_timestamps = timestamps.clone();
        sorted_timestamps.sort();
        
        assert_eq!(
            timestamps, sorted_timestamps,
            "Migration files are not in chronological order"
        );
        
        println!("✅ Migration files are chronologically ordered");
    }

    #[test]
    fn test_migration_naming_convention() {
        let migration_files = get_migration_files();
        
        for file in &migration_files {
            let filename = Path::new(&file).file_name().unwrap().to_str().unwrap();
            
            // Check format: TIMESTAMP_description.sql
            assert!(
                filename.ends_with(".sql"),
                "Migration file {} doesn't end with .sql",
                filename
            );
            
            let parts: Vec<&str> = filename.split('_').collect();
            assert!(
                parts.len() >= 2,
                "Migration file {} doesn't follow TIMESTAMP_description format",
                filename
            );
            
            // Check timestamp format (should be 14-17 digits)
            let timestamp = parts[0];
            assert!(
                timestamp.len() >= 14 && timestamp.len() <= 17,
                "Migration timestamp {} has invalid length in file {}",
                timestamp, filename
            );
            
            assert!(
                timestamp.chars().all(|c| c.is_numeric()),
                "Migration timestamp {} contains non-numeric characters in file {}",
                timestamp, filename
            );
            
            // Check description
            let description_parts = &parts[1..];
            let description = description_parts.join("_");
            let description_without_ext = description.trim_end_matches(".sql");
            
            assert!(
                !description_without_ext.is_empty(),
                "Migration file {} has empty description",
                filename
            );
            
            assert!(
                description_without_ext.chars().all(|c| c.is_alphanumeric() || c == '_'),
                "Migration description contains invalid characters in file {}",
                filename
            );
        }
        
        println!("✅ All migration files follow naming convention");
    }

    #[test]
    fn test_migration_dependencies() {
        let migration_files = get_migration_files();
        let migration_contents = read_all_migrations();
        
        // Check for common dependency patterns
        for (i, (file, content)) in migration_contents.iter().enumerate() {
            // Check if migration references tables that should exist
            let referenced_tables = extract_referenced_tables(&content);
            
            for table in &referenced_tables {
                // Skip system tables
                if table.starts_with("pg_") || table.starts_with("information_schema") {
                    continue;
                }
                
                // Check if table is created in current or previous migrations
                let table_exists = table_exists_before_migration(&migration_contents, i, table);
                
                // Special cases for tables that might be created in the same migration
                let creates_table = content.to_lowercase().contains(&format!("create table {}", table.to_lowercase())) ||
                                  content.to_lowercase().contains(&format!("create table if not exists {}", table.to_lowercase()));
                
                if !creates_table && !table_exists {
                    println!("Warning: Migration {} references table '{}' that may not exist", file, table);
                }
            }
        }
        
        println!("✅ Migration dependencies checked");
    }

    #[test]
    fn test_no_drop_statements_in_migrations() {
        let migration_contents = read_all_migrations();
        
        for (file, content) in &migration_contents {
            let lowercase_content = content.to_lowercase();
            
            // Check for dangerous DROP statements
            assert!(
                !lowercase_content.contains("drop table") || lowercase_content.contains("drop table if exists"),
                "Migration {} contains DROP TABLE statement without IF EXISTS",
                file
            );
            
            assert!(
                !lowercase_content.contains("drop database"),
                "Migration {} contains dangerous DROP DATABASE statement",
                file
            );
            
            assert!(
                !lowercase_content.contains("drop schema"),
                "Migration {} contains DROP SCHEMA statement",
                file
            );
        }
        
        println!("✅ No dangerous DROP statements found");
    }

    #[test]
    fn test_migration_transactions() {
        let migration_contents = read_all_migrations();
        
        for (file, content) in &migration_contents {
            let lowercase_content = content.to_lowercase();
            
            // Check that migrations don't contain explicit transaction statements
            // (SQLx handles transactions automatically)
            assert!(
                !lowercase_content.contains("begin;") && !lowercase_content.contains("begin transaction"),
                "Migration {} contains explicit BEGIN statement",
                file
            );
            
            assert!(
                !lowercase_content.contains("commit;"),
                "Migration {} contains explicit COMMIT statement",
                file
            );
            
            assert!(
                !lowercase_content.contains("rollback;"),
                "Migration {} contains explicit ROLLBACK statement",
                file
            );
        }
        
        println!("✅ Migrations don't contain explicit transaction statements");
    }

    #[tokio::test]
    async fn test_migration_idempotency() {
        // This test would be run in CI to ensure migrations can be run multiple times
        // We'll create a simple check here
        let migration_contents = read_all_migrations();
        
        for (file, content) in &migration_contents {
            // Check for CREATE statements with IF NOT EXISTS
            if content.to_lowercase().contains("create table") {
                let has_if_not_exists = content.to_lowercase().contains("create table if not exists");
                if !has_if_not_exists {
                    println!("Warning: Migration {} creates table without IF NOT EXISTS", file);
                }
            }
            
            if content.to_lowercase().contains("create index") {
                let has_if_not_exists = content.to_lowercase().contains("create index if not exists");
                if !has_if_not_exists {
                    println!("Warning: Migration {} creates index without IF NOT EXISTS", file);
                }
            }
        }
        
        println!("✅ Migration idempotency patterns checked");
    }

    #[test]
    fn test_migration_comments() {
        let migration_contents = read_all_migrations();
        let mut undocumented_migrations = Vec::new();
        
        for (file, content) in &migration_contents {
            // Check if migration has comments explaining what it does
            let has_comments = content.contains("--") || content.contains("/*");
            
            if !has_comments {
                undocumented_migrations.push(file.clone());
            }
            
            // Check for specific important migrations that should have detailed comments
            if file.contains("failed_documents") {
                assert!(
                    content.contains("--") && content.len() > 200,
                    "Migration {} dealing with failed_documents should have detailed comments",
                    file
                );
            }
        }
        
        if !undocumented_migrations.is_empty() {
            println!("Warning: The following migrations lack comments: {:?}", undocumented_migrations);
        }
        
        println!("✅ Migration documentation checked");
    }

    #[test]
    fn test_migration_file_consistency() {
        let migration_files = get_migration_files();
        
        for file in &migration_files {
            let content = fs::read_to_string(&file).unwrap();
            
            // Check for consistent line endings
            assert!(
                !content.contains("\r\n") || !content.contains("\n"),
                "Migration {} has mixed line endings",
                file
            );
            
            // Check for trailing whitespace (optional check, can be disabled)
            for (line_num, line) in content.lines().enumerate() {
                if line.ends_with(' ') || line.ends_with('\t') {
                    println!("Note: Migration {} has trailing whitespace on line {} (style preference)", file, line_num + 1);
                }
            }
            
            // Check file ends with newline (optional check, can be disabled)
            if !content.ends_with('\n') {
                println!("Note: Migration {} doesn't end with newline (style preference)", file);
            }
        }
        
        println!("✅ Migration file consistency verified");
    }

    // Helper functions
    
    fn get_migration_files() -> Vec<String> {
        let migrations_dir = Path::new("migrations");
        let mut files = Vec::new();
        
        if let Ok(entries) = fs::read_dir(migrations_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                        files.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
        
        files.sort();
        files
    }
    
    fn extract_timestamp(filepath: &str) -> String {
        let filename = Path::new(filepath).file_name().unwrap().to_str().unwrap();
        filename.split('_').next().unwrap().to_string()
    }
    
    fn read_all_migrations() -> Vec<(String, String)> {
        let migration_files = get_migration_files();
        let mut contents = Vec::new();
        
        for file in migration_files {
            if let Ok(content) = fs::read_to_string(&file) {
                contents.push((file, content));
            }
        }
        
        contents
    }
    
    fn extract_referenced_tables(content: &str) -> Vec<String> {
        let mut tables = Vec::new();
        
        // Simple regex-like patterns to find table references
        let patterns = vec![
            "references ", "from ", "join ", "into ", "update ", "delete from ",
            "alter table ", "constraint.*references", "on delete", "on update"
        ];
        
        for line in content.lines() {
            let lower_line = line.to_lowercase();
            for pattern in &patterns {
                if lower_line.contains(pattern) {
                    // Extract table name (simplified - real implementation would use regex)
                    let parts: Vec<&str> = lower_line.split_whitespace().collect();
                    for (i, part) in parts.iter().enumerate() {
                        if part == &pattern.trim() && i + 1 < parts.len() {
                            let table_name = parts[i + 1].trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                            if !table_name.is_empty() && !table_name.starts_with("$") {
                                tables.push(table_name.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        tables.sort();
        tables.dedup();
        tables
    }
    
    fn table_exists_before_migration(migrations: &[(String, String)], current_index: usize, table_name: &str) -> bool {
        for i in 0..current_index {
            let (_, content) = &migrations[i];
            if content.to_lowercase().contains(&format!("create table {}", table_name.to_lowercase())) ||
               content.to_lowercase().contains(&format!("create table if not exists {}", table_name.to_lowercase())) {
                return true;
            }
        }
        
        // Check for base tables that should always exist
        let base_tables = vec!["users", "documents", "settings"];
        base_tables.contains(&table_name)
    }
}