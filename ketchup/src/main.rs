use anyhow::Result;
use clap::Parser;
use output::OutputManager;
use serde_json::Value;
use tracing::info;

mod k8s;
mod output;

#[derive(Parser, Debug)]
#[command(name = "ketchup")]
#[command(about = "Collect Kubernetes cluster configurations")]
#[command(version)]
struct Args {
    /// Path to kubeconfig file (required)
    #[arg(short, long)]
    kubeconfig: String,

    /// Namespaces to collect from (comma-separated)
    #[arg(short, long)]
    namespaces: Option<String>,

    /// Output directory for the archive
    #[arg(short, long, default_value = "./tmp")]
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
    info!("Using kubeconfig: {}", args.kubeconfig);

    // Connect to Kubernetes using specified kubeconfig
    let kube_client = k8s::KubeClient::new_client(&args.kubeconfig).await?;

    // Determine which namespaces to collect from
    let requested_namespaces = if let Some(ns_str) = &args.namespaces {
        ns_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec!["default".to_string()]
    };

    let verified_namespaces = kube_client.verify_namespaces(&requested_namespaces).await?;
    info!("Will collect from namespaces: {:?}", verified_namespaces);
    info!("Output directory: {}", args.output);

    // Collect pods from verified namespaces
    info!("Starting pod collection...");
    let pods = kube_client.collect_pods(&verified_namespaces).await?;
    info!("Successfully collected {} pods total", pods.len());

    // Collect services from verified namespaces
    info!("Starting service collection...");
    let services = kube_client.collect_services(&verified_namespaces).await?;
    info!("Successfully collected {} services total", services.len());

    // Create output manager and save files
    info!("Setting up file output...");
    let output_manager = OutputManager::new_output_manager(args.output);
    let output_dir = output_manager.create_output_directory()?;

    // Save resource for each namespace with chosen format
    for namespace in &verified_namespaces {
        // Save pods
        let namespace_pods: Vec<&Value> = pods
            .iter()
            .filter(|pod| {
                pod.get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .collect();

        // Save services
        let namespace_services: Vec<&Value> = services
            .iter()
            .filter(|service| {
                service
                    .get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .collect();

        let namespace_pod_values: Vec<Value> = namespace_pods.iter().map(|&p| p.clone()).collect();
        let namespace_service_values: Vec<Value> =
            namespace_services.iter().map(|&s| s.clone()).collect();

        output_manager.save_pods_json(&output_dir, namespace, &namespace_pod_values)?;
        output_manager.save_pods_yaml(&output_dir, namespace, &namespace_pod_values)?;
    }

    // Create summary files
    output_manager.save_pods_with_format(
        &output_dir,
        namespace,
        &namespace_pod_values,
        &args.format,
    )?;
    output_manager.save_services_with_format(
        &output_dir,
        namespace,
        &namespace_service_values,
        &args.format,
    )?;

    // Create compressed archive
    let archive_path = output_manager.create_archive(&output_dir)?;

    info!("Files saved to: {}", output_dir);
    info!("Archive created: {}", archive_path);

    info!("Files saved to: {}", output_dir);

    info!("Collection completed successfully");
    Ok(())
}

fn init_logging(verbose: bool) {
    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt().with_max_level(level).init();
}
