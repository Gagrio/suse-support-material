use anyhow::Result;
use clap::Parser;
use tracing::{info, error};

mod k8s;

#[derive(Parser, Debug)]
#[command(name = "ketchup")]
#[command(about = "Collect Kubernetes cluster configurations")]
#[command(version)]
struct Args {
    /// Namespaces to collect from (comma-separated)
    #[arg(short, long)]
    namespaces: Option<String>,
    
    /// Output directory for the archive
    #[arg(short, long, default_value = "./output")]
    output: String,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    init_logging(args.verbose);
    
    info!("Starting Ketchup - Kubernetes Config Collector");
    
    // Connect to Kubernetes
    let kube_client = k8s::KubeClient::new().await?;
    
    // Determine which namespaces to collect from
    let requested_namespaces = if let Some(ns_str) = &args.namespaces {
        ns_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec!["default".to_string()]
    };
    
    let verified_namespaces = kube_client.verify_namespaces(&requested_namespaces).await?;
    info!("Will collect from namespaces: {:?}", verified_namespaces);
    info!("Output directory: {}", args.output);
    
    // TODO: This is where we'll add resource collection
    info!("Collection completed successfully");
    Ok(())
}

fn init_logging(verbose: bool) {
    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();
}
