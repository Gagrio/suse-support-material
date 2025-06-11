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

    /// Save pods to JSON file organized by namespace
    pub fn save_pods_json(&self, output_dir: &str, namespace: &str, pods: &[Value]) -> Result<()> {
        let filename = format!("{}/{}-pods.json", output_dir, namespace);
        let json_content =
            serde_json::to_string_pretty(pods).context("Failed to serialize pods to JSON")?;

        info!(
            "Saving {} pods from namespace '{}' to {}",
            pods.len(),
            namespace,
            filename
        );
        fs::write(&filename, json_content).context("Failed to write JSON file")?;

        Ok(())
    }

    /// Save pods to YAML file organized by namespace  
    pub fn save_pods_yaml(&self, output_dir: &str, namespace: &str, pods: &[Value]) -> Result<()> {
        let filename = format!("{}/{}-pods.yaml", output_dir, namespace);
        let yaml_content =
            serde_yaml::to_string(pods).context("Failed to serialize pods to YAML")?;

        info!(
            "Saving {} pods from namespace '{}' to {}",
            pods.len(),
            namespace,
            filename
        );
        fs::write(&filename, yaml_content).context("Failed to write YAML file")?;

        Ok(())
    }

    /// Create a summary file with collection metadata
    pub fn create_summary(
        &self,
        output_dir: &str,
        namespaces: &[String],
        total_pods: usize,
    ) -> Result<()> {
        let summary = serde_json::json!({
            "collection_info": {
                "timestamp": self.timestamp.to_rfc3339(),
                "tool": "ketchup",
                "version": env!("CARGO_PKG_VERSION")
            },
            "cluster_info": {
                "namespaces_requested": namespaces,
                "namespaces_collected": namespaces.len(),
                "total_pods_collected": total_pods
            },
            "files_created": {
                "json_files": namespaces.iter().map(|ns| format!("{}-pods.json", ns)).collect::<Vec<_>>(),
                "yaml_files": namespaces.iter().map(|ns| format!("{}-pods.yaml", ns)).collect::<Vec<_>>()
            }
        });

        let filename = format!("{}/collection-summary.json", output_dir);
        info!("Creating collection summary: {}", filename);

        let summary_content =
            serde_json::to_string_pretty(&summary).context("Failed to serialize summary")?;

        fs::write(&filename, summary_content).context("Failed to write summary file")?;

        Ok(())
    }

    /// Create a summary file in YAML format
    pub fn create_summary_yaml(
        &self,
        output_dir: &str,
        namespaces: &[String],
        total_pods: usize,
    ) -> Result<()> {
        let summary = serde_json::json!({
            "collection_info": {
                "timestamp": self.timestamp.to_rfc3339(),
                "tool": "ketchup",
                "version": env!("CARGO_PKG_VERSION")
            },
            "cluster_info": {
                "namespaces_requested": namespaces,
                "namespaces_collected": namespaces.len(),
                "total_pods_collected": total_pods
            },
            "files_created": {
                "json_files": namespaces.iter().map(|ns| format!("{}-pods.json", ns)).collect::<Vec<_>>(),
                "yaml_files": namespaces.iter().map(|ns| format!("{}-pods.yaml", ns)).collect::<Vec<_>>()
            }
        });

        let filename = format!("{}/collection-summary.yaml", output_dir);
        info!("Creating YAML collection summary: {}", filename);

        let summary_content =
            serde_yaml::to_string(&summary).context("Failed to serialize summary to YAML")?;

        fs::write(&filename, summary_content).context("Failed to write YAML summary file")?;

        Ok(())
    }

    /// Create compressed archive of the output directory
    pub fn create_archive(&self, output_dir: &str) -> Result<String> {
        let archive_name = format!("{}.tar.gz", output_dir);
        info!("Creating compressed archive: {}", archive_name);

        let tar_gz =
            std::fs::File::create(&archive_name).context("Failed to create archive file")?;
        let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);

        // Add the entire output directory to the archive
        tar.append_dir_all(".", output_dir)
            .context("Failed to add directory to archive")?;

        tar.finish().context("Failed to finalize archive")?;
        info!("Archive created successfully: {}", archive_name);

        Ok(archive_name)
    }
}
