use anyhow::Result;
use clap::Parser;
use output::{NamespaceStats, OutputManager};
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
    #[arg(short, long, default_value = "/tmp")]
    output: String,

    /// Output format: json, yaml, or both
    #[arg(short, long, default_value = "yaml", value_parser = ["json", "yaml", "both"])]
    format: String,

    /// Compression: compressed, uncompressed, or both
    #[arg(short = 'c', long, default_value = "compressed", value_parser = ["compressed", "uncompressed", "both"])]
    compression: String,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

async fn collect_all_resources(
    kube_client: &k8s::KubeClient,
    namespaces: &[String],
) -> Result<(
    Vec<Value>,
    Vec<Value>,
    Vec<Value>,
    Vec<Value>,
    Vec<Value>,
    Vec<Value>,
    Vec<Value>,
    Vec<Value>,
)> {
    info!("Starting resource collection...");

    let pods = kube_client.collect_pods(namespaces).await?;
    info!("Successfully collected {} pods total", pods.len());

    let services = kube_client.collect_services(namespaces).await?;
    info!("Successfully collected {} services total", services.len());

    let deployments = kube_client.collect_deployments(namespaces).await?;
    info!(
        "Successfully collected {} deployments total",
        deployments.len()
    );

    let configmaps = kube_client.collect_configmaps(namespaces).await?;
    info!(
        "Successfully collected {} configmaps total",
        configmaps.len()
    );

    let secrets = kube_client.collect_secrets(namespaces).await?;
    info!("Successfully collected {} secrets total", secrets.len());

    let ingresses = kube_client.collect_ingresses(namespaces).await?;
    info!("Successfully collected {} ingresses total", ingresses.len());

    let pvcs = kube_client
        .collect_persistentvolumeclaims(namespaces)
        .await?;
    info!(
        "Successfully collected {} persistentvolumeclaims total",
        pvcs.len()
    );

    let networkpolicies = kube_client.collect_networkpolicies(namespaces).await?;
    info!(
        "Successfully collected {} networkpolicies total",
        networkpolicies.len()
    );

    Ok((
        pods,
        services,
        deployments,
        configmaps,
        secrets,
        ingresses,
        pvcs,
        networkpolicies,
    ))
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

    // Collect all resources from verified namespaces
    let (pods, services, deployments, configmaps, secrets, ingresses, pvcs, networkpolicies) =
        collect_all_resources(&kube_client, &verified_namespaces).await?;

    // Create output manager and save files
    info!("Setting up file output...");
    info!(
        "Output format: {}, Compression: {}",
        args.format, args.compression
    );
    let output_manager = OutputManager::new_output_manager(args.output);
    let output_dir = output_manager.create_output_directory()?;

    // Save all resources for each namespace
    let mut namespace_stats = Vec::new();

    for namespace in &verified_namespaces {
        // Filter resources by namespace
        let namespace_pods: Vec<Value> = pods
            .iter()
            .filter(|pod| {
                pod.get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .cloned()
            .collect();

        let namespace_services: Vec<Value> = services
            .iter()
            .filter(|service| {
                service
                    .get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .cloned()
            .collect();

        let namespace_deployments: Vec<Value> = deployments
            .iter()
            .filter(|deployment| {
                deployment
                    .get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .cloned()
            .collect();

        let namespace_configmaps: Vec<Value> = configmaps
            .iter()
            .filter(|configmap| {
                configmap
                    .get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .cloned()
            .collect();

        let namespace_secrets: Vec<Value> = secrets
            .iter()
            .filter(|secret| {
                secret
                    .get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .cloned()
            .collect();

        let namespace_ingresses: Vec<Value> = ingresses
            .iter()
            .filter(|ingress| {
                ingress
                    .get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .cloned()
            .collect();

        let namespace_pvcs: Vec<Value> = pvcs
            .iter()
            .filter(|pvc| {
                pvc.get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .cloned()
            .collect();

        let namespace_networkpolicies: Vec<Value> = networkpolicies
            .iter()
            .filter(|networkpolicy| {
                networkpolicy
                    .get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|ns| ns.as_str())
                    == Some(namespace)
            })
            .cloned()
            .collect();

        // Save all resource types
        let pods_saved = output_manager.save_pods_individually(
            &output_dir,
            namespace,
            &namespace_pods,
            &args.format,
        )?;

        let services_saved = output_manager.save_services_individually(
            &output_dir,
            namespace,
            &namespace_services,
            &args.format,
        )?;

        let deployments_saved = output_manager.save_deployments_individually(
            &output_dir,
            namespace,
            &namespace_deployments,
            &args.format,
        )?;

        let configmaps_saved = output_manager.save_configmaps_individually(
            &output_dir,
            namespace,
            &namespace_configmaps,
            &args.format,
        )?;

        let secrets_saved = output_manager.save_secrets_individually(
            &output_dir,
            namespace,
            &namespace_secrets,
            &args.format,
        )?;

        let ingresses_saved = output_manager.save_ingresses_individually(
            &output_dir,
            namespace,
            &namespace_ingresses,
            &args.format,
        )?;

        let pvcs_saved = output_manager.save_persistentvolumeclaims_individually(
            &output_dir,
            namespace,
            &namespace_pvcs,
            &args.format,
        )?;

        let networkpolicies_saved = output_manager.save_networkpolicies_individually(
            &output_dir,
            namespace,
            &namespace_networkpolicies,
            &args.format,
        )?;

        namespace_stats.push(NamespaceStats {
            namespace: namespace.clone(),
            pods: pods_saved,
            services: services_saved,
            deployments: deployments_saved,
            configmaps: configmaps_saved,
            secrets: secrets_saved,
            ingresses: ingresses_saved,
            pvcs: pvcs_saved,
            networkpolicies: networkpolicies_saved,
        });
    }

    // Create enhanced summary
    output_manager.create_enhanced_summary(&output_dir, &namespace_stats)?;

    // Handle compression based on user preference
    if let Some(archive_path) = output_manager.handle_compression(&output_dir, &args.compression)? {
        info!("Archive created: {}", archive_path);
    }

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
