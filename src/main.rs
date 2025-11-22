mod api;
mod cli;
mod db;
mod domain;
mod service;

use clap::{Parser, Subcommand};
use cli::Repl;
use db::SqliteDatabase;
use service::CazinoService;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "cazino")]
#[command(about = "Family betting game server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run interactive CLI for local testing
    Cli,

    /// Run HTTP + WebSocket API server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Database URL
        #[arg(short, long, default_value = "sqlite://cazino.db?mode=rwc")]
        database: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with better formatting
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .with_ansi(true)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Cli => run_cli().await?,
        Commands::Serve { port, database } => run_server(port, database).await?,
    }

    Ok(())
}

async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting Cazino CLI");

    // Connect to SQLite database
    let db = SqliteDatabase::new("sqlite://cazino.db?mode=rwc").await?;
    db.run_migrations().await?;

    // Create service
    let service = CazinoService::new(Arc::new(db));

    // Start REPL
    let mut repl = Repl::new(service);
    repl.run().await;

    Ok(())
}

async fn run_server(port: u16, database: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Initializing Cazino API server...");
    println!("   Port: {}", port);
    println!("   Database: {}", database);

    tracing::info!("Starting Cazino API server on port {}", port);
    tracing::info!("Database: {}", database);

    // Connect to database
    println!("📦 Connecting to database...");
    let db = SqliteDatabase::new(&database).await?;
    println!("🔨 Running migrations...");
    db.run_migrations().await?;
    println!("✅ Database ready!");

    // Create service
    let service = CazinoService::new(Arc::new(db));

    // Start server
    api::run_server(service, port).await?;

    Ok(())
}
