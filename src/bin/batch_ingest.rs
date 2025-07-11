use anyhow::Result;
use clap::{Arg, Command};
use std::path::Path;
use uuid::Uuid;

use readur::{
    ingestion::batch_ingest::BatchIngester,
    config::Config,
    db::Database,
    services::file_service::FileService,
    ocr::queue::OcrQueueService,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with custom filters to reduce spam from pdf_extract crate
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new("info")
                .add_directive("pdf_extract=error".parse().unwrap()) // Suppress pdf_extract WARN spam
                .add_directive("readur=info".parse().unwrap())       // Keep our app logs at info
        });
    
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();
    
    let matches = Command::new("batch_ingest")
        .about("Batch ingest files for OCR processing")
        .arg(
            Arg::new("directory")
                .help("Directory to ingest files from")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("user-id")
                .help("User ID to assign documents to")
                .long("user-id")
                .short('u')
                .value_name("UUID")
                .required(true),
        )
        .arg(
            Arg::new("monitor")
                .help("Monitor progress after starting ingestion")
                .long("monitor")
                .short('m')
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();
    
    let directory = matches.get_one::<String>("directory").unwrap();
    let user_id_str = matches.get_one::<String>("user-id").unwrap();
    let monitor = matches.get_flag("monitor");
    
    let user_id = Uuid::parse_str(user_id_str)?;
    let dir_path = Path::new(directory);
    
    if !dir_path.exists() {
        eprintln!("Error: Directory {} does not exist", directory);
        std::process::exit(1);
    }
    
    let config = Config::from_env()?;
    let db = Database::new(&config.database_url).await?;
    let file_service = FileService::new(config.upload_path.clone());
    let queue_service = OcrQueueService::new(db.clone(), db.get_pool().clone(), 1);
    
    let ingester = BatchIngester::new(db, queue_service, file_service, config);
    
    println!("Starting batch ingestion from: {}", directory);
    // Only show the first and last character of the user ID
    let masked_user_id = format!("{}{}", &user_id.to_string()[..1], &user_id.to_string()[user_id.to_string().len() - 1..]);
    println!("User ID: {}", masked_user_id);
    
    // Start ingestion
    if let Err(e) = ingester.ingest_directory(dir_path, user_id).await {
        eprintln!("Ingestion failed: {}", e);
        std::process::exit(1);
    }
    
    println!("Batch ingestion completed successfully!");
    
    if monitor {
        println!("Monitoring OCR queue progress...");
        if let Err(e) = ingester.monitor_progress().await {
            eprintln!("Monitoring failed: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}