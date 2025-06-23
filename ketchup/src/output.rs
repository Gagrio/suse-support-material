use std::fs;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::info;

#[derive(Debug, Clone)]
pub struct NamespaceStats {
    pub namespace: String,
    pub pods: usize,
    pub services: usize,
    pub deployments: usize,
    pub configmaps: usize,
    pub secrets: usize,
    pub ingresses: usize,
    pub pvcs: usize,
    pub networkpolicies: usize,
    // High priority workload controllers
    pub replicasets: usize,
    pub daemonsets: usize,
    pub statefulsets: usize,
    pub jobs: usize,
    pub cronjobs: usize,
    // RBAC resources
    pub serviceaccounts: usize,
    pub roles: usize,
    pub rolebindings: usize,
    // Resource management
    pub resourcequotas: usize,
    pub limitranges: usize,
    pub horizontalpodautoscalers: usize,
    pub poddisruptionbudgets: usize,
    // Network resources
    pub endpoints: usize,
    pub endpointslices: usize,
}

impl NamespaceStats {
    pub fn total_resources(&self) -> usize {
        self.pods
            + self.services
            + self.deployments
            + self.configmaps
            + self.secrets
            + self.ingresses
            + self.pvcs
            + self.networkpolicies
            + self.replicasets
            + self.daemonsets
            + self.statefulsets
            + self.jobs
            + self.cronjobs
            + self.serviceaccounts
            + self.roles
            + self.rolebindings
            + self.resourcequotas
            + self.limitranges
            + self.horizontalpodautoscalers
            + self.poddisruptionbudgets
            + self.endpoints
            + self.endpointslices
    }
}

pub struct OutputManager {
    base_dir: String,
    timestamp: DateTime<Utc>,
}

impl OutputManager {
    pub fn new_output_manager(base_dir: String) -> Self {
        Self {
            base_dir,
            timestamp: Utc::now(),
        }
    }

    /// Create timestamped output directory
    pub fn create_output_directory(&self) -> Result<String> {
        let timestamp_str = self.timestamp.format("%Y-%m-%d-%H-%M-%S");
        let output_dir = format!("{}/ketchup-{}", self.base_dir, timestamp_str);

        info!("Creating output directory: {}", output_dir);
        fs::create_dir_all(&output_dir).context("Failed to create output directory")?;

        Ok(output_dir)
    }

    /// Generic method to save any resource type individually
    pub fn save_resources_individually(
        &self,
        output_dir: &str,
        namespace: &str,
        resources: &[Value],
        resource_type: &str,
        format: &str,
    ) -> Result<usize> {
        let resource_dir = format!("{}/{}/{}", output_dir, namespace, resource_type);
        fs::create_dir_all(&resource_dir)
            .with_context(|| format!("Failed to create namespace {} directory", resource_type))?;

        let mut saved_count = 0;
        for resource in resources {
            if let Some(resource_name) = resource
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
            {
                match format {
                    "json" => {
                        let filename = format!("{}/{}.json", resource_dir, resource_name);
                        let content = serde_json::to_string_pretty(resource)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "yaml" => {
                        let filename = format!("{}/{}.yaml", resource_dir, resource_name);
                        let content = serde_yaml::to_string(resource)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "both" => {
                        let json_file = format!("{}/{}.json", resource_dir, resource_name);
                        let yaml_file = format!("{}/{}.yaml", resource_dir, resource_name);

                        let json_content = serde_json::to_string_pretty(resource)?;
                        let yaml_content = serde_yaml::to_string(resource)?;

                        fs::write(&json_file, json_content)?;
                        fs::write(&yaml_file, yaml_content)?;
                        saved_count += 1;
                    }
                    _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
                }
            }
        }

        info!(
            "Saved {} {} to {}",
            saved_count, resource_type, resource_dir
        );
        Ok(saved_count)
    }

    /// Create enhanced summary with per-namespace resource breakdown and cluster resources
    pub fn create_enhanced_summary(
        &self,
        output_dir: &str,
        namespace_stats: &[NamespaceStats],
        cluster_stats: &std::collections::HashMap<String, usize>,
    ) -> Result<()> {
        let mut total_pods = 0;
        let mut total_services = 0;
        let mut total_deployments = 0;
        let mut total_configmaps = 0;
        let mut total_secrets = 0;
        let mut total_ingresses = 0;
        let mut total_pvcs = 0;
        let mut total_networkpolicies = 0;
        // Workload controllers
        let mut total_replicasets = 0;
        let mut total_daemonsets = 0;
        let mut total_statefulsets = 0;
        let mut total_jobs = 0;
        let mut total_cronjobs = 0;
        // RBAC resources
        let mut total_serviceaccounts = 0;
        let mut total_roles = 0;
        let mut total_rolebindings = 0;
        // Resource management
        let mut total_resourcequotas = 0;
        let mut total_limitranges = 0;
        let mut total_horizontalpodautoscalers = 0;
        let mut total_poddisruptionbudgets = 0;
        // Network resources
        let mut total_endpoints = 0;
        let mut total_endpointslices = 0;
        let mut namespace_details = serde_json::Map::new();

        for stats in namespace_stats {
            total_pods += stats.pods;
            total_services += stats.services;
            total_deployments += stats.deployments;
            total_configmaps += stats.configmaps;
            total_secrets += stats.secrets;
            total_ingresses += stats.ingresses;
            total_pvcs += stats.pvcs;
            total_networkpolicies += stats.networkpolicies;
            // Workload controllers
            total_replicasets += stats.replicasets;
            total_daemonsets += stats.daemonsets;
            total_statefulsets += stats.statefulsets;
            total_jobs += stats.jobs;
            total_cronjobs += stats.cronjobs;
            // RBAC resources
            total_serviceaccounts += stats.serviceaccounts;
            total_roles += stats.roles;
            total_rolebindings += stats.rolebindings;
            // Resource management
            total_resourcequotas += stats.resourcequotas;
            total_limitranges += stats.limitranges;
            total_horizontalpodautoscalers += stats.horizontalpodautoscalers;
            total_poddisruptionbudgets += stats.poddisruptionbudgets;
            // Network resources
            total_endpoints += stats.endpoints;
            total_endpointslices += stats.endpointslices;

            namespace_details.insert(
                stats.namespace.clone(),
                serde_json::json!({
                    // Core resources
                    "pods_collected": stats.pods,
                    "services_collected": stats.services,
                    "deployments_collected": stats.deployments,
                    "configmaps_collected": stats.configmaps,
                    "secrets_collected": stats.secrets,
                    "ingresses_collected": stats.ingresses,
                    "persistentvolumeclaims_collected": stats.pvcs,
                    "networkpolicies_collected": stats.networkpolicies,
                    // Workload controllers
                    "replicasets_collected": stats.replicasets,
                    "daemonsets_collected": stats.daemonsets,
                    "statefulsets_collected": stats.statefulsets,
                    "jobs_collected": stats.jobs,
                    "cronjobs_collected": stats.cronjobs,
                    // RBAC resources
                    "serviceaccounts_collected": stats.serviceaccounts,
                    "roles_collected": stats.roles,
                    "rolebindings_collected": stats.rolebindings,
                    // Resource management
                    "resourcequotas_collected": stats.resourcequotas,
                    "limitranges_collected": stats.limitranges,
                    "horizontalpodautoscalers_collected": stats.horizontalpodautoscalers,
                    "poddisruptionbudgets_collected": stats.poddisruptionbudgets,
                    // Network resources
                    "endpoints_collected": stats.endpoints,
                    "endpointslices_collected": stats.endpointslices,
                    "a_summary_of_total_resources": stats.total_resources()
                }),
            );
        }

        // Calculate cluster resource totals
        let total_clusterroles = cluster_stats.get("clusterroles").unwrap_or(&0);
        let total_clusterrolebindings = cluster_stats.get("clusterrolebindings").unwrap_or(&0);

        // Calculate grand total including cluster resources
        let namespaced_total = total_pods
            + total_services
            + total_deployments
            + total_configmaps
            + total_secrets
            + total_ingresses
            + total_pvcs
            + total_networkpolicies
            + total_replicasets
            + total_daemonsets
            + total_statefulsets
            + total_jobs
            + total_cronjobs
            + total_serviceaccounts
            + total_roles
            + total_rolebindings
            + total_resourcequotas
            + total_limitranges
            + total_horizontalpodautoscalers
            + total_poddisruptionbudgets
            + total_endpoints
            + total_endpointslices;
        let cluster_total = total_clusterroles + total_clusterrolebindings;
        let grand_total = namespaced_total + cluster_total;

        let summary = serde_json::json!({
            "collection_info": {
                "timestamp": self.timestamp.to_rfc3339(),
                "tool": "ketchup",
                "version": env!("CARGO_PKG_VERSION")
            },
            "cluster_summary": {
                "total_namespaces": namespace_stats.len(),
                // Core resources
                "total_pods": total_pods,
                "total_services": total_services,
                "total_deployments": total_deployments,
                "total_configmaps": total_configmaps,
                "total_secrets": total_secrets,
                "total_ingresses": total_ingresses,
                "total_persistentvolumeclaims": total_pvcs,
                "total_networkpolicies": total_networkpolicies,
                // Workload controllers
                "total_replicasets": total_replicasets,
                "total_daemonsets": total_daemonsets,
                "total_statefulsets": total_statefulsets,
                "total_jobs": total_jobs,
                "total_cronjobs": total_cronjobs,
                // RBAC resources
                "total_serviceaccounts": total_serviceaccounts,
                "total_roles": total_roles,
                "total_rolebindings": total_rolebindings,
                // Resource management
                "total_resourcequotas": total_resourcequotas,
                "total_limitranges": total_limitranges,
                "total_horizontalpodautoscalers": total_horizontalpodautoscalers,
                "total_poddisruptionbudgets": total_poddisruptionbudgets,
                // Network resources
                "total_endpoints": total_endpoints,
                "total_endpointslices": total_endpointslices,
                "a_summary_of_total_resources": grand_total
            },
            "cluster_resources": {
                "total_clusterroles": total_clusterroles,
                "total_clusterrolebindings": total_clusterrolebindings,
                "cluster_resources_total": cluster_total
            },
            "namespace_details": namespace_details
        });

        let filename = format!("{}/collection-summary.yaml", output_dir);
        info!("Creating enhanced collection summary: {}", filename);

        let summary_content =
            serde_yaml::to_string(&summary).context("Failed to serialize summary to YAML")?;
        fs::write(&filename, summary_content).context("Failed to write YAML summary file")?;

        Ok(())
    }

    /// Create archive based on compression preference
    pub fn handle_compression(
        &self,
        output_dir: &str,
        compression: &str,
    ) -> Result<Option<String>> {
        match compression {
            "compressed" => {
                let archive_path = self.create_archive(output_dir)?;
                Ok(Some(archive_path))
            }
            "uncompressed" => {
                info!("Skipping compression as requested");
                Ok(None)
            }
            "both" => {
                let archive_path = self.create_archive(output_dir)?;
                info!("Files available both compressed and uncompressed");
                Ok(Some(archive_path))
            }
            _ => {
                anyhow::bail!(
                    "Invalid compression: {}. Use compressed, uncompressed, or both",
                    compression
                );
            }
        }
    }

    /// Create compressed archive of the output directory
    pub fn create_archive(&self, output_dir: &str) -> Result<String> {
        let archive_name = format!("{}.tar.gz", output_dir);
        info!("Creating compressed archive: {}", archive_name);

        let tar_gz =
            std::fs::File::create(&archive_name).context("Failed to create archive file")?;
        let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);

        tar.append_dir_all(".", output_dir)
            .context("Failed to add directory to archive")?;
        tar.finish().context("Failed to finalize archive")?;
        info!("Archive created successfully: {}", archive_name);

        Ok(archive_name)
    }
}
