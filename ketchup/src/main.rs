use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use tracing::{debug, info, warn};

use output::{NamespaceStats, OutputManager};

mod k8s;
mod output;

#[derive(Parser, Debug)]
#[command(name = "ketchup")]
#[command(about = "Collect Kubernetes cluster configurations for recreation")]
#[command(
    long_about = "Collects all Kubernetes resources needed to recreate a cluster setup.
Custom resource collection may show API errors in resource-constrained clusters - these can be safely ignored as the tool will continue successfully."
)]
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

    /// Include custom resource instances (may show API errors that can be safely ignored)
    #[arg(short = 'C', long, default_value = "false")]
    include_custom_resources: bool,

    /// Verbose logging (progress and summaries)
    #[arg(short, long)]
    verbose: bool,

    /// Debug logging (includes HTTP requests and detailed traces)
    #[arg(short, long)]
    debug: bool,
}

async fn collect_namespaced_resources(
    kube_client: &k8s::KubeClient,
    namespaces: &[String],
    include_custom_resources: bool,
) -> Result<std::collections::HashMap<String, Vec<Value>>> {
    use std::collections::HashMap;

    warn!("🚀 Starting namespaced resource collection...");

    let mut resources = HashMap::new();

    // Core resources
    let pods = kube_client.collect_pods(namespaces).await?;
    warn!("✅ Successfully collected {} pods total", pods.len());
    resources.insert("pods".to_string(), pods);

    let services = kube_client.collect_services(namespaces).await?;
    info!(
        "🌐 Successfully collected {} services total",
        services.len()
    );
    resources.insert("services".to_string(), services);

    let deployments = kube_client.collect_deployments(namespaces).await?;
    info!(
        "🚢 Successfully collected {} deployments total",
        deployments.len()
    );
    resources.insert("deployments".to_string(), deployments);

    let configmaps = kube_client.collect_configmaps(namespaces).await?;
    info!(
        "⚙️ Successfully collected {} configmaps total",
        configmaps.len()
    );
    resources.insert("configmaps".to_string(), configmaps);

    let secrets = kube_client.collect_secrets(namespaces).await?;
    info!("🔐 Successfully collected {} secrets total", secrets.len());
    resources.insert("secrets".to_string(), secrets);

    let ingresses = kube_client.collect_ingresses(namespaces).await?;
    info!(
        "🌍 Successfully collected {} ingresses total",
        ingresses.len()
    );
    resources.insert("ingresses".to_string(), ingresses);

    let pvcs = kube_client
        .collect_persistentvolumeclaims(namespaces)
        .await?;
    info!(
        "💾 Successfully collected {} persistentvolumeclaims total",
        pvcs.len()
    );
    resources.insert("persistentvolumeclaims".to_string(), pvcs);

    let networkpolicies = kube_client.collect_networkpolicies(namespaces).await?;
    info!(
        "🛡️ Successfully collected {} networkpolicies total",
        networkpolicies.len()
    );
    resources.insert("networkpolicies".to_string(), networkpolicies);

    // Workload controllers
    let replicasets = kube_client.collect_replicasets(namespaces).await?;
    info!(
        "🔄 Successfully collected {} replicasets total",
        replicasets.len()
    );
    resources.insert("replicasets".to_string(), replicasets);

    let daemonsets = kube_client.collect_daemonsets(namespaces).await?;
    info!(
        "👹 Successfully collected {} daemonsets total",
        daemonsets.len()
    );
    resources.insert("daemonsets".to_string(), daemonsets);

    let statefulsets = kube_client.collect_statefulsets(namespaces).await?;
    info!(
        "📊 Successfully collected {} statefulsets total",
        statefulsets.len()
    );
    resources.insert("statefulsets".to_string(), statefulsets);

    let jobs = kube_client.collect_jobs(namespaces).await?;
    info!("⚡ Successfully collected {} jobs total", jobs.len());
    resources.insert("jobs".to_string(), jobs);

    let cronjobs = kube_client.collect_cronjobs(namespaces).await?;
    info!(
        "⏰ Successfully collected {} cronjobs total",
        cronjobs.len()
    );
    resources.insert("cronjobs".to_string(), cronjobs);

    // RBAC resources
    let serviceaccounts = kube_client.collect_serviceaccounts(namespaces).await?;
    info!(
        "👤 Successfully collected {} serviceaccounts total",
        serviceaccounts.len()
    );
    resources.insert("serviceaccounts".to_string(), serviceaccounts);

    let roles = kube_client.collect_roles(namespaces).await?;
    info!("🎭 Successfully collected {} roles total", roles.len());
    resources.insert("roles".to_string(), roles);

    let rolebindings = kube_client.collect_rolebindings(namespaces).await?;
    info!(
        "🔗 Successfully collected {} rolebindings total",
        rolebindings.len()
    );
    resources.insert("rolebindings".to_string(), rolebindings);

    // Resource management
    let resourcequotas = kube_client.collect_resourcequotas(namespaces).await?;
    info!(
        "📏 Successfully collected {} resourcequotas total",
        resourcequotas.len()
    );
    resources.insert("resourcequotas".to_string(), resourcequotas);

    let limitranges = kube_client.collect_limitranges(namespaces).await?;
    info!(
        "⚖️ Successfully collected {} limitranges total",
        limitranges.len()
    );
    resources.insert("limitranges".to_string(), limitranges);

    let horizontalpodautoscalers = kube_client
        .collect_horizontalpodautoscalers(namespaces)
        .await?;
    info!(
        "📈 Successfully collected {} horizontalpodautoscalers total",
        horizontalpodautoscalers.len()
    );
    resources.insert(
        "horizontalpodautoscalers".to_string(),
        horizontalpodautoscalers,
    );

    let poddisruptionbudgets = kube_client.collect_poddisruptionbudgets(namespaces).await?;
    info!(
        "🛡️ Successfully collected {} poddisruptionbudgets total",
        poddisruptionbudgets.len()
    );
    resources.insert("poddisruptionbudgets".to_string(), poddisruptionbudgets);

    // Network resources
    let endpoints = kube_client.collect_endpoints(namespaces).await?;
    info!(
        "🔌 Successfully collected {} endpoints total",
        endpoints.len()
    );
    resources.insert("endpoints".to_string(), endpoints);

    let endpointslices = kube_client.collect_endpointslices(namespaces).await?;
    info!(
        "🍰 Successfully collected {} endpointslices total",
        endpointslices.len()
    );
    resources.insert("endpointslices".to_string(), endpointslices);

    // Custom resources (with graceful error handling)
    if include_custom_resources {
        warn!("🎯 Collecting custom resource instances (API errors can be safely ignored)...");
        match kube_client.collect_all_custom_resources(namespaces).await {
            Ok(custom_resources) => {
                warn!(
                    "🎯 Successfully collected {} custom resource types",
                    custom_resources.len()
                );
                // Add each custom resource type to the resources map
                for (cr_type, cr_instances) in custom_resources {
                    resources.insert(cr_type, cr_instances);
                }
            }
            Err(e) => {
                warn!(
                    "⚠️ Custom resource collection encountered API errors: {}",
                    e
                );
                warn!(
                    "🎯 Continuing with cluster recreation files (CRDs available for custom resource types)"
                );
            }
        }
    } else {
        warn!("🎯 Skipping custom resource instances (--no-include-custom-resources specified)");
    }

    Ok(resources)
}

async fn collect_cluster_resources(
    kube_client: &k8s::KubeClient,
) -> Result<std::collections::HashMap<String, Vec<Value>>> {
    use std::collections::HashMap;

    warn!("☸️ Starting cluster-scoped resource collection...");

    let mut resources = HashMap::new();

    // Cluster-scoped resources
    let clusterroles = kube_client.collect_clusterroles().await?;
    warn!(
        "🎭 Successfully collected {} clusterroles total",
        clusterroles.len()
    );
    resources.insert("clusterroles".to_string(), clusterroles);

    let clusterrolebindings = kube_client.collect_clusterrolebindings().await?;
    warn!(
        "🔗 Successfully collected {} clusterrolebindings total",
        clusterrolebindings.len()
    );
    resources.insert("clusterrolebindings".to_string(), clusterrolebindings);

    let nodes = kube_client.collect_nodes().await?;
    warn!("🖥️ Successfully collected {} nodes total", nodes.len());
    resources.insert("nodes".to_string(), nodes);

    let persistentvolumes = kube_client.collect_persistentvolumes().await?;
    warn!(
        "💽 Successfully collected {} persistentvolumes total",
        persistentvolumes.len()
    );
    resources.insert("persistentvolumes".to_string(), persistentvolumes);

    let storageclasses = kube_client.collect_storageclasses().await?;
    warn!(
        "📦 Successfully collected {} storageclasses total",
        storageclasses.len()
    );
    resources.insert("storageclasses".to_string(), storageclasses);

    let customresourcedefinitions = kube_client.collect_customresourcedefinitions().await?;
    warn!(
        "🎯 Successfully collected {} customresourcedefinitions total",
        customresourcedefinitions.len()
    );
    resources.insert(
        "customresourcedefinitions".to_string(),
        customresourcedefinitions,
    );

    Ok(resources)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(args.verbose, args.debug);

    warn!("🍅 Starting Ketchup - Kubernetes Config Collector");
    warn!("📁 Using kubeconfig: {}", args.kubeconfig);

    // Connect to Kubernetes using specified kubeconfig
    let kube_client = k8s::KubeClient::new_client(&args.kubeconfig).await?;

    // Determine which namespaces to collect from
    let requested_namespaces = if let Some(ns_str) = &args.namespaces {
        ns_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        debug!("🌍 No namespaces specified, collecting from ALL namespaces");
        kube_client.list_namespaces().await?
    };

    let verified_namespaces = kube_client.verify_namespaces(&requested_namespaces).await?;
    debug!("✅ Will collect from namespaces: {:?}", verified_namespaces);
    debug!("📂 Output directory: {}", args.output);

    // Collect resources using separate functions
    let namespaced_resources = collect_namespaced_resources(
        &kube_client,
        &verified_namespaces,
        args.include_custom_resources,
    )
    .await?;
    let cluster_resources = collect_cluster_resources(&kube_client).await?;

    // Create output manager and save files
    warn!("💾 Setting up file output...");
    debug!(
        "Output format: {}, Compression: {}",
        args.format, args.compression
    );
    let output_manager = OutputManager::new_output_manager(args.output);
    let output_dir = output_manager.create_output_directory()?;

    // Save all namespaced resources for each namespace
    let mut namespace_stats: Vec<NamespaceStats> = Vec::new();

    for namespace in &verified_namespaces {
        let mut stats = NamespaceStats {
            namespace: namespace.clone(),
            pods: 0,
            services: 0,
            deployments: 0,
            configmaps: 0,
            secrets: 0,
            ingresses: 0,
            pvcs: 0,
            networkpolicies: 0,
            // High priority workload controllers
            replicasets: 0,
            daemonsets: 0,
            statefulsets: 0,
            jobs: 0,
            cronjobs: 0,
            // RBAC resources
            serviceaccounts: 0,
            roles: 0,
            rolebindings: 0,
            // Resource management
            resourcequotas: 0,
            limitranges: 0,
            horizontalpodautoscalers: 0,
            poddisruptionbudgets: 0,
            // Network resources
            endpoints: 0,
            endpointslices: 0,
        };

        // Process each namespaced resource type
        for (resource_type, all_resources) in &namespaced_resources {
            // Filter resources by namespace
            let namespace_resources: Vec<Value> = all_resources
                .iter()
                .filter(|resource| {
                    resource
                        .get("metadata")
                        .and_then(|m| m.get("namespace"))
                        .and_then(|ns| ns.as_str())
                        == Some(namespace)
                })
                .cloned()
                .collect();

            // Save resources and update stats
            let saved_count = output_manager.save_resources_individually(
                &output_dir,
                namespace,
                &namespace_resources,
                resource_type,
                &args.format,
            )?;

            // Update the appropriate field in stats
            match resource_type.as_str() {
                "pods" => stats.pods = saved_count,
                "services" => stats.services = saved_count,
                "deployments" => stats.deployments = saved_count,
                "configmaps" => stats.configmaps = saved_count,
                "secrets" => stats.secrets = saved_count,
                "ingresses" => stats.ingresses = saved_count,
                "persistentvolumeclaims" => stats.pvcs = saved_count,
                "networkpolicies" => stats.networkpolicies = saved_count,
                // Workload controllers
                "replicasets" => stats.replicasets = saved_count,
                "daemonsets" => stats.daemonsets = saved_count,
                "statefulsets" => stats.statefulsets = saved_count,
                "jobs" => stats.jobs = saved_count,
                "cronjobs" => stats.cronjobs = saved_count,
                // RBAC resources
                "serviceaccounts" => stats.serviceaccounts = saved_count,
                "roles" => stats.roles = saved_count,
                "rolebindings" => stats.rolebindings = saved_count,
                // Resource management
                "resourcequotas" => stats.resourcequotas = saved_count,
                "limitranges" => stats.limitranges = saved_count,
                "horizontalpodautoscalers" => stats.horizontalpodautoscalers = saved_count,
                "poddisruptionbudgets" => stats.poddisruptionbudgets = saved_count,
                // Network resources
                "endpoints" => stats.endpoints = saved_count,
                "endpointslices" => stats.endpointslices = saved_count,
                // Custom resources - these don't get counted in namespace stats (they get their own category)
                _ if resource_type.contains('.') => {
                    debug!(
                        "Saved {} instances of custom resource type: {}",
                        saved_count, resource_type
                    );
                }
                _ => {} // Ignore unknown resource types
            }
        }

        namespace_stats.push(stats);
    }

    // Process cluster-scoped resources
    let mut cluster_stats = std::collections::HashMap::new();

    for (resource_type, cluster_resource_list) in &cluster_resources {
        // Save cluster resources to cluster-wide directory
        let saved_count = output_manager.save_resources_individually(
            &output_dir,
            "cluster-wide", // Use "cluster-wide" as directory name
            cluster_resource_list,
            resource_type,
            &args.format,
        )?;

        cluster_stats.insert(resource_type.clone(), saved_count);
    }

    // Create enhanced summary
    output_manager.create_enhanced_summary(&output_dir, &namespace_stats, &cluster_stats)?;

    // Handle compression based on user preference
    if let Some(archive_path) = output_manager.handle_compression(&output_dir, &args.compression)? {
        debug!("📦 Archive created: {}", archive_path);
    }

    warn!("💾 Files saved to: {}", output_dir);
    warn!("🎉 Collection completed successfully");
    Ok(())
}

fn init_logging(verbose: bool, debug: bool) {
    let level = if debug {
        tracing::Level::DEBUG
    } else if verbose {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN
    };

    tracing_subscriber::fmt().with_max_level(level).init();
}
