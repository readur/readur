#[cfg(test)]
mod tests {
    use readur::test_utils::TestContext;
    use sqlx::{postgres::PgRow, Row};

    #[tokio::test]
    async fn test_database_schema_inspection() {
        let ctx = TestContext::new().await;
        let db = &ctx.state().db;

        println!("\n=== DATABASE SCHEMA INSPECTION TEST ===\n");

        // 1. Check constraints on users table (especially looking for check_role)
        println!("1. CONSTRAINTS ON USERS TABLE:");
        println!("==============================");
        
        let constraints_query = r#"
            SELECT 
                tc.constraint_name,
                tc.constraint_type,
                ccu.column_name,
                cc.check_clause
            FROM information_schema.table_constraints AS tc
            LEFT JOIN information_schema.constraint_column_usage AS ccu 
                ON tc.constraint_name = ccu.constraint_name
            LEFT JOIN information_schema.check_constraints AS cc 
                ON tc.constraint_name = cc.constraint_name
            WHERE tc.table_name = 'users'
            ORDER BY tc.constraint_type, tc.constraint_name;
        "#;

        let constraint_rows: Vec<PgRow> = sqlx::query(constraints_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query user constraints");

        if constraint_rows.is_empty() {
            println!("   No constraints found on users table");
        } else {
            for row in constraint_rows {
                let constraint_name: String = row.get("constraint_name");
                let constraint_type: String = row.get("constraint_type");
                let column_name: Option<String> = row.try_get("column_name").ok();
                let check_clause: Option<String> = row.try_get("check_clause").ok();
                
                println!("   {} ({})", constraint_name, constraint_type);
                if let Some(col) = column_name {
                    println!("     Column: {}", col);
                }
                if let Some(clause) = check_clause {
                    println!("     Check clause: {}", clause);
                }
                println!();
            }
        }

        // Specifically look for check_role constraint
        let check_role_query = r#"
            SELECT 
                constraint_name,
                check_clause
            FROM information_schema.check_constraints 
            WHERE constraint_name LIKE '%role%' 
               OR check_clause LIKE '%role%'
        "#;
        
        let check_role_rows: Vec<PgRow> = sqlx::query(check_role_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query role constraints");
            
        if !check_role_rows.is_empty() {
            println!("   ROLE-RELATED CONSTRAINTS:");
            for row in check_role_rows {
                let constraint_name: String = row.get("constraint_name");
                let check_clause: String = row.get("check_clause");
                println!("     {}: {}", constraint_name, check_clause);
            }
            println!();
        }

        // 2. Check nullability of user_id in documents table
        println!("2. USER_ID NULLABILITY IN DOCUMENTS TABLE:");
        println!("==========================================");
        
        let nullability_query = r#"
            SELECT 
                column_name,
                is_nullable,
                data_type,
                column_default
            FROM information_schema.columns 
            WHERE table_name = 'documents' 
              AND column_name = 'user_id'
        "#;

        let nullability_rows: Vec<PgRow> = sqlx::query(nullability_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query user_id nullability");

        if nullability_rows.is_empty() {
            println!("   user_id column not found in documents table");
        } else {
            for row in nullability_rows {
                let column_name: String = row.get("column_name");
                let is_nullable: String = row.get("is_nullable");
                let data_type: String = row.get("data_type");
                let column_default: Option<String> = row.try_get("column_default").ok();
                
                println!("   Column: {}", column_name);
                println!("   Type: {}", data_type);
                println!("   Nullable: {}", is_nullable);
                if let Some(default) = column_default {
                    println!("   Default: {}", default);
                } else {
                    println!("   Default: None");
                }
            }
        }
        println!();

        // 3. Check indexes on documents table (especially idx_documents_created_at)
        println!("3. INDEXES ON DOCUMENTS TABLE:");
        println!("==============================");
        
        let indexes_query = r#"
            SELECT 
                indexname,
                indexdef
            FROM pg_indexes 
            WHERE tablename = 'documents'
            ORDER BY indexname
        "#;

        let index_rows: Vec<PgRow> = sqlx::query(indexes_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query document indexes");

        if index_rows.is_empty() {
            println!("   No indexes found on documents table");
        } else {
            for row in index_rows {
                let indexname: String = row.get("indexname");
                let indexdef: String = row.get("indexdef");
                
                println!("   Index: {}", indexname);
                println!("   Definition: {}", indexdef);
                println!();
            }
        }

        // Specifically check for idx_documents_created_at
        let created_at_index_query = r#"
            SELECT 
                indexname,
                indexdef
            FROM pg_indexes 
            WHERE tablename = 'documents' 
              AND indexname LIKE '%created_at%'
        "#;
        
        let created_at_index_rows: Vec<PgRow> = sqlx::query(created_at_index_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query created_at index");
            
        if created_at_index_rows.is_empty() {
            println!("   No created_at index found");
        } else {
            println!("   CREATED_AT INDEXES:");
            for row in created_at_index_rows {
                let indexname: String = row.get("indexname");
                let indexdef: String = row.get("indexdef");
                println!("     {}: {}", indexname, indexdef);
            }
        }
        println!();

        // 4. Check functions in database (especially add_document_to_ocr_queue)
        println!("4. FUNCTIONS IN DATABASE:");
        println!("========================");
        
        let functions_query = r#"
            SELECT 
                proname as function_name,
                pg_get_function_result(oid) as return_type,
                pg_get_function_arguments(oid) as arguments
            FROM pg_proc 
            WHERE pronamespace = (
                SELECT oid FROM pg_namespace WHERE nspname = 'public'
            )
            AND prokind = 'f'
            ORDER BY proname
        "#;

        let function_rows: Vec<PgRow> = sqlx::query(functions_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query database functions");

        if function_rows.is_empty() {
            println!("   No functions found in public schema");
        } else {
            for row in function_rows {
                let function_name: String = row.get("function_name");
                let return_type: String = row.get("return_type");
                let arguments: String = row.get("arguments");
                
                println!("   Function: {}", function_name);
                println!("   Returns: {}", return_type);
                println!("   Arguments: {}", arguments);
                println!();
            }
        }

        // Specifically check for add_document_to_ocr_queue function
        let ocr_queue_function_query = r#"
            SELECT 
                proname as function_name,
                pg_get_function_result(oid) as return_type,
                pg_get_function_arguments(oid) as arguments,
                prosrc as source_code
            FROM pg_proc 
            WHERE proname LIKE '%ocr_queue%'
            ORDER BY proname
        "#;
        
        let ocr_queue_function_rows: Vec<PgRow> = sqlx::query(ocr_queue_function_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query OCR queue functions");
            
        if ocr_queue_function_rows.is_empty() {
            println!("   No OCR queue functions found");
        } else {
            println!("   OCR QUEUE FUNCTIONS:");
            for row in ocr_queue_function_rows {
                let function_name: String = row.get("function_name");
                let return_type: String = row.get("return_type");
                let arguments: String = row.get("arguments");
                let source_code: String = row.get("source_code");
                
                println!("     Function: {}", function_name);
                println!("     Returns: {}", return_type);
                println!("     Arguments: {}", arguments);
                println!("     Source code preview: {}", 
                    source_code.chars().take(200).collect::<String>() + 
                    if source_code.len() > 200 { "..." } else { "" }
                );
                println!();
            }
        }

        // 5. Additional useful schema information
        println!("5. ADDITIONAL SCHEMA INFORMATION:");
        println!("================================");
        
        // Check all tables in the database
        let tables_query = r#"
            SELECT 
                table_name,
                table_type
            FROM information_schema.tables 
            WHERE table_schema = 'public'
            ORDER BY table_name
        "#;

        let table_rows: Vec<PgRow> = sqlx::query(tables_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query tables");

        println!("   TABLES IN DATABASE:");
        for row in table_rows {
            let table_name: String = row.get("table_name");
            let table_type: String = row.get("table_type");
            println!("     {} ({})", table_name, table_type);
        }
        println!();

        // Check columns in documents table for completeness
        let documents_columns_query = r#"
            SELECT 
                column_name,
                data_type,
                is_nullable,
                column_default
            FROM information_schema.columns 
            WHERE table_name = 'documents'
            ORDER BY ordinal_position
        "#;

        let documents_columns_rows: Vec<PgRow> = sqlx::query(documents_columns_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query document columns");

        println!("   DOCUMENTS TABLE COLUMNS:");
        for row in documents_columns_rows {
            let column_name: String = row.get("column_name");
            let data_type: String = row.get("data_type");
            let is_nullable: String = row.get("is_nullable");
            let column_default: Option<String> = row.try_get("column_default").ok();
            
            println!("     {} ({}, nullable: {}{})", 
                column_name, 
                data_type, 
                is_nullable,
                if let Some(default) = column_default {
                    format!(", default: {}", default)
                } else {
                    "".to_string()
                }
            );
        }
        println!();

        // Check columns in users table for completeness
        let users_columns_query = r#"
            SELECT 
                column_name,
                data_type,
                is_nullable,
                column_default
            FROM information_schema.columns 
            WHERE table_name = 'users'
            ORDER BY ordinal_position
        "#;

        let users_columns_rows: Vec<PgRow> = sqlx::query(users_columns_query)
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query user columns");

        println!("   USERS TABLE COLUMNS:");
        for row in users_columns_rows {
            let column_name: String = row.get("column_name");
            let data_type: String = row.get("data_type");
            let is_nullable: String = row.get("is_nullable");
            let column_default: Option<String> = row.try_get("column_default").ok();
            
            println!("     {} ({}, nullable: {}{})", 
                column_name, 
                data_type, 
                is_nullable,
                if let Some(default) = column_default {
                    format!(", default: {}", default)
                } else {
                    "".to_string()
                }
            );
        }

        println!("\n=== END SCHEMA INSPECTION ===\n");
        
        // The test passes if we reach this point - it's purely informational
        assert!(true, "Schema inspection completed successfully");
    }
}