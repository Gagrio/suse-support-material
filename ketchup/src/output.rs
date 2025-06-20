use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use tracing::info;

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

    /// Save individual pods to namespace/pods/ structure
    pub fn save_pods_individually(
        &self,
        output_dir: &str,
        namespace: &str,
        pods: &[Value],
        format: &str,
    ) -> Result<usize> {
        let pods_dir = format!("{}/{}/pods", output_dir, namespace);
        fs::create_dir_all(&pods_dir).context("Failed to create namespace pods directory")?;

        let mut saved_count = 0;
        for pod in pods {
            if let Some(pod_name) = pod
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
            {
                match format {
                    "json" => {
                        let filename = format!("{}/{}.json", pods_dir, pod_name);
                        let content = serde_json::to_string_pretty(pod)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "yaml" => {
                        let filename = format!("{}/{}.yaml", pods_dir, pod_name);
                        let content = serde_yaml::to_string(pod)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "both" => {
                        let json_file = format!("{}/{}.json", pods_dir, pod_name);
                        let yaml_file = format!("{}/{}.yaml", pods_dir, pod_name);

                        let json_content = serde_json::to_string_pretty(pod)?;
                        let yaml_content = serde_yaml::to_string(pod)?;

                        fs::write(&json_file, json_content)?;
                        fs::write(&yaml_file, yaml_content)?;
                        saved_count += 1;
                    }
                    _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
                }
            }
        }

        info!("Saved {} pods to {}", saved_count, pods_dir);
        Ok(saved_count)
    }

    /// Save individual services to namespace/services/ structure
    pub fn save_services_individually(
        &self,
        output_dir: &str,
        namespace: &str,
        services: &[Value],
        format: &str,
    ) -> Result<usize> {
        let services_dir = format!("{}/{}/services", output_dir, namespace);
        fs::create_dir_all(&services_dir)
            .context("Failed to create namespace services directory")?;

        let mut saved_count = 0;
        for service in services {
            if let Some(service_name) = service
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
            {
                match format {
                    "json" => {
                        let filename = format!("{}/{}.json", services_dir, service_name);
                        let content = serde_json::to_string_pretty(service)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "yaml" => {
                        let filename = format!("{}/{}.yaml", services_dir, service_name);
                        let content = serde_yaml::to_string(service)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "both" => {
                        let json_file = format!("{}/{}.json", services_dir, service_name);
                        let yaml_file = format!("{}/{}.yaml", services_dir, service_name);

                        let json_content = serde_json::to_string_pretty(service)?;
                        let yaml_content = serde_yaml::to_string(service)?;

                        fs::write(&json_file, json_content)?;
                        fs::write(&yaml_file, yaml_content)?;
                        saved_count += 1;
                    }
                    _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
                }
            }
        }

        info!("Saved {} services to {}", saved_count, services_dir);
        Ok(saved_count)
    }

    /// Save individual deployments to namespace/deployments/ structure
    pub fn save_deployments_individually(
        &self,
        output_dir: &str,
        namespace: &str,
        deployments: &[Value],
        format: &str,
    ) -> Result<usize> {
        let deployments_dir = format!("{}/{}/deployments", output_dir, namespace);
        fs::create_dir_all(&deployments_dir)
            .context("Failed to create namespace deployments directory")?;

        let mut saved_count = 0;
        for deployment in deployments {
            if let Some(deployment_name) = deployment
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
            {
                match format {
                    "json" => {
                        let filename = format!("{}/{}.json", deployments_dir, deployment_name);
                        let content = serde_json::to_string_pretty(deployment)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "yaml" => {
                        let filename = format!("{}/{}.yaml", deployments_dir, deployment_name);
                        let content = serde_yaml::to_string(deployment)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "both" => {
                        let json_file = format!("{}/{}.json", deployments_dir, deployment_name);
                        let yaml_file = format!("{}/{}.yaml", deployments_dir, deployment_name);

                        let json_content = serde_json::to_string_pretty(deployment)?;
                        let yaml_content = serde_yaml::to_string(deployment)?;

                        fs::write(&json_file, json_content)?;
                        fs::write(&yaml_file, yaml_content)?;
                        saved_count += 1;
                    }
                    _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
                }
            }
        }

        info!("Saved {} deployments to {}", saved_count, deployments_dir);
        Ok(saved_count)
    }

    /// Save individual configmaps to namespace/configmaps/ structure
    pub fn save_configmaps_individually(
        &self,
        output_dir: &str,
        namespace: &str,
        configmaps: &[Value],
        format: &str,
    ) -> Result<usize> {
        let configmaps_dir = format!("{}/{}/configmaps", output_dir, namespace);
        fs::create_dir_all(&configmaps_dir)
            .context("Failed to create namespace configmaps directory")?;

        let mut saved_count = 0;
        for configmap in configmaps {
            if let Some(configmap_name) = configmap
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
            {
                match format {
                    "json" => {
                        let filename = format!("{}/{}.json", configmaps_dir, configmap_name);
                        let content = serde_json::to_string_pretty(configmap)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "yaml" => {
                        let filename = format!("{}/{}.yaml", configmaps_dir, configmap_name);
                        let content = serde_yaml::to_string(configmap)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "both" => {
                        let json_file = format!("{}/{}.json", configmaps_dir, configmap_name);
                        let yaml_file = format!("{}/{}.yaml", configmaps_dir, configmap_name);

                        let json_content = serde_json::to_string_pretty(configmap)?;
                        let yaml_content = serde_yaml::to_string(configmap)?;

                        fs::write(&json_file, json_content)?;
                        fs::write(&yaml_file, yaml_content)?;
                        saved_count += 1;
                    }
                    _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
                }
            }
        }

        info!("Saved {} configmaps to {}", saved_count, configmaps_dir);
        Ok(saved_count)
    }

    /// Save individual secrets to namespace/secrets/ structure
    pub fn save_secrets_individually(
        &self,
        output_dir: &str,
        namespace: &str,
        secrets: &[Value],
        format: &str,
    ) -> Result<usize> {
        let secrets_dir = format!("{}/{}/secrets", output_dir, namespace);
        fs::create_dir_all(&secrets_dir).context("Failed to create namespace secrets directory")?;

        let mut saved_count = 0;
        for secret in secrets {
            if let Some(secret_name) = secret
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
            {
                match format {
                    "json" => {
                        let filename = format!("{}/{}.json", secrets_dir, secret_name);
                        let content = serde_json::to_string_pretty(secret)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "yaml" => {
                        let filename = format!("{}/{}.yaml", secrets_dir, secret_name);
                        let content = serde_yaml::to_string(secret)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "both" => {
                        let json_file = format!("{}/{}.json", secrets_dir, secret_name);
                        let yaml_file = format!("{}/{}.yaml", secrets_dir, secret_name);

                        let json_content = serde_json::to_string_pretty(secret)?;
                        let yaml_content = serde_yaml::to_string(secret)?;

                        fs::write(&json_file, json_content)?;
                        fs::write(&yaml_file, yaml_content)?;
                        saved_count += 1;
                    }
                    _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
                }
            }
        }

        info!("Saved {} secrets to {}", saved_count, secrets_dir);
        Ok(saved_count)
    }

    /// Create enhanced summary with per-namespace resource breakdown
    pub fn create_enhanced_summary(
        &self,
        output_dir: &str,
        namespace_stats: &[(String, usize, usize, usize, usize, usize)], // Updated to include secrets
    ) -> Result<()> {
        let mut total_pods = 0;
        let mut total_services = 0;
        let mut total_deployments = 0;
        let mut total_configmaps = 0;
        let mut total_secrets = 0;
        let mut namespace_details = serde_json::Map::new();

        for (
            namespace,
            pod_count,
            service_count,
            deployment_count,
            configmap_count,
            secret_count,
        ) in namespace_stats
        {
            total_pods += pod_count;
            total_services += service_count;
            total_deployments += deployment_count;
            total_configmaps += configmap_count;
            total_secrets += secret_count;

            namespace_details.insert(
                namespace.clone(),
                serde_json::json!({
                    "pods_collected": pod_count,
                    "services_collected": service_count,
                    "deployments_collected": deployment_count,
                    "configmaps_collected": configmap_count,
                    "secrets_collected": secret_count,
                    "total_resources": pod_count + service_count + deployment_count + configmap_count + secret_count
                }),
            );
        }

        let summary = serde_json::json!({
            "collection_info": {
                "timestamp": self.timestamp.to_rfc3339(),
                "tool": "ketchup",
                "version": env!("CARGO_PKG_VERSION")
            },
            "cluster_summary": {
                "total_namespaces": namespace_stats.len(),
                "total_pods": total_pods,
                "total_services": total_services,
                "total_deployments": total_deployments,
                "total_configmaps": total_configmaps,
                "total_secrets": total_secrets,
                "total_resources": total_pods + total_services + total_deployments + total_configmaps + total_secrets
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
