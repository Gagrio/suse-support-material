use std::fs;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::{debug, info, warn};

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

#[derive(Debug, Clone, Default)]
pub struct SanitizationStats {
    pub total_processed: usize,
    pub total_sanitized: usize,
    pub total_skipped: usize,
    pub skipped_resources: Vec<String>,
}

impl SanitizationStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, other: &SanitizationStats) {
        self.total_processed += other.total_processed;
        self.total_sanitized += other.total_sanitized;
        self.total_skipped += other.total_skipped;
        self.skipped_resources
            .extend(other.skipped_resources.clone());
    }

    pub fn record_sanitized(&mut self) {
        self.total_processed += 1;
        self.total_sanitized += 1;
    }

    pub fn record_skipped(&mut self, resource_identifier: String) {
        self.total_processed += 1;
        self.total_skipped += 1;
        self.skipped_resources.push(resource_identifier);
    }

    pub fn record_raw(&mut self) {
        self.total_processed += 1;
        // Raw resources are neither sanitized nor skipped
    }
}

// ===== NEW: SUSE Edge Analysis Structs =====

#[derive(Debug, Clone)]
pub struct SuseEdgeAnalysis {
    pub components: Vec<SuseEdgeComponent>,
    pub total_components: usize,
    pub confidence: String,
    pub edge_version_detected: Option<String>,
    pub deployment_type: String, // "Management Cluster", "Downstream Cluster", "Standalone"
    pub kubernetes_distribution: Option<String>, // "RKE2", "K3s", or "Unknown"
}

#[derive(Debug, Clone)]
pub struct SuseEdgeComponent {
    pub name: String,
    pub version: Option<String>,
    pub chart_version: Option<String>,
    pub found_in: Vec<String>,
    pub category: String, // "Core", "Storage", "Security", "Networking", "Virtualization", "Tools"
}

// Helper structs for resource categorization
#[derive(Default)]
struct WorkloadResources {
    total: usize,
    pods: usize,
    deployments: usize,
    jobs: usize,
    daemonsets: usize,
    statefulsets: usize,
    cronjobs: usize,
    replicasets: usize,
}

#[derive(Default)]
struct SecurityResources {
    total: usize,
    service_accounts: usize,
    roles: usize,
    rolebindings: usize,
}

#[derive(Default)]
struct ConfigurationResources {
    total: usize,
    configmaps: usize,
    secrets: usize,
}

#[derive(Default)]
struct NetworkingResources {
    total: usize,
    services: usize,
    endpoints: usize,
    ingresses: usize,
    networkpolicies: usize,
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

        debug!("Creating output directory: {}", output_dir);
        fs::create_dir_all(&output_dir).context("Failed to create output directory")?;

        Ok(output_dir)
    }

    /// Sanitize a Kubernetes resource for kubectl apply readiness
    fn sanitize_resource_for_apply(&self, resource: &mut Value) -> Result<()> {
        if let Some(obj) = resource.as_object_mut() {
            // Remove status section entirely
            obj.remove("status");

            // Clean metadata of cluster-specific fields
            if let Some(metadata) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) {
                metadata.remove("uid");
                metadata.remove("resourceVersion");
                metadata.remove("creationTimestamp");
                metadata.remove("generation");
                metadata.remove("managedFields");
                metadata.remove("selfLink");

                // Clean problematic annotations
                if let Some(annotations) = metadata
                    .get_mut("annotations")
                    .and_then(|a| a.as_object_mut())
                {
                    annotations.retain(|key, _| {
                        !key.starts_with("kubectl.kubernetes.io/")
                            && !key.starts_with("deployment.kubernetes.io/")
                            && key != "control-plane.alpha.kubernetes.io/leader"
                    });

                    // Remove empty annotations object
                    if annotations.is_empty() {
                        metadata.remove("annotations");
                    }
                }

                // Clean finalizers that might cause issues
                if let Some(finalizers) = metadata
                    .get_mut("finalizers")
                    .and_then(|f| f.as_array_mut())
                {
                    finalizers.retain(|finalizer| {
                        if let Some(finalizer_str) = finalizer.as_str() {
                            // Keep custom finalizers but remove system ones that might cause issues
                            !finalizer_str.starts_with("kubernetes.io/")
                        } else {
                            true
                        }
                    });

                    // Remove empty finalizers array
                    if finalizers.is_empty() {
                        metadata.remove("finalizers");
                    }
                }
            }

            // Resource-specific sanitization
            match obj.get("kind").and_then(|k| k.as_str()) {
                Some("Node") => {
                    // Nodes are infrastructure - remove most dynamic fields
                    if let Some(spec) = obj.get_mut("spec").and_then(|s| s.as_object_mut()) {
                        // Keep only essential node configuration
                        spec.retain(|key, _| {
                            matches!(key.as_str(), "podCIDR" | "podCIDRs" | "taints")
                        });
                    }
                }
                Some("Service") => {
                    if let Some(spec) = obj.get_mut("spec").and_then(|s| s.as_object_mut()) {
                        // Remove cluster-assigned fields
                        spec.remove("clusterIP");
                        spec.remove("clusterIPs");

                        // Handle NodePort services
                        if let Some(ports) = spec.get_mut("ports").and_then(|p| p.as_array_mut()) {
                            for port in ports {
                                if let Some(port_obj) = port.as_object_mut() {
                                    // Remove auto-assigned node ports unless explicitly set
                                    if let Some(node_port) = port_obj.get("nodePort") {
                                        if node_port.as_u64().unwrap_or(0) >= 30000 {
                                            port_obj.remove("nodePort");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Some("PersistentVolume") => {
                    if let Some(spec) = obj.get_mut("spec").and_then(|s| s.as_object_mut()) {
                        // Remove claim reference to make PV reusable
                        spec.remove("claimRef");
                    }
                }
                Some("PersistentVolumeClaim") => {
                    if let Some(spec) = obj.get_mut("spec").and_then(|s| s.as_object_mut()) {
                        // Remove volume name to allow dynamic provisioning
                        spec.remove("volumeName");
                    }
                }
                _ => {} // No special handling for other resource types
            }
        }

        Ok(())
    }

    /// Get a human-readable resource identifier
    fn get_resource_identifier(&self, resource: &Value, resource_type: &str) -> String {
        let name = resource
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        let namespace = resource
            .get("metadata")
            .and_then(|m| m.get("namespace"))
            .and_then(|ns| ns.as_str());

        if let Some(ns) = namespace {
            format!("{}/{} ({})", resource_type, name, ns)
        } else {
            format!("{}/{}", resource_type, name)
        }
    }

    /// Generic method to save any resource type individually with optional sanitization
    pub fn save_resources_individually(
        &self,
        output_dir: &str,
        namespace: &str,
        resources: &[Value],
        resource_type: &str,
        format: &str,
        sanitize: bool,
    ) -> Result<(usize, SanitizationStats)> {
        // Skip empty resources - don't create directories for 0 resources
        if resources.is_empty() {
            return Ok((0, SanitizationStats::new()));
        }

        // Determine if this is a custom resource (contains '.')
        let is_custom_resource = resource_type.contains('.');

        let resource_dir = if is_custom_resource {
            format!(
                "{}/{}/custom-resources/{}",
                output_dir, namespace, resource_type
            )
        } else {
            format!("{}/{}/{}", output_dir, namespace, resource_type)
        };

        fs::create_dir_all(&resource_dir).with_context(|| {
            format!(
                "Failed to create {} directory for {}",
                resource_type, namespace
            )
        })?;

        let mut saved_count = 0;
        let mut sanitization_stats = SanitizationStats::new();

        for resource in resources {
            if let Some(resource_name) = resource
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
            {
                // Prepare the resource (sanitize if requested)
                let final_resource = if sanitize {
                    let mut resource_copy = resource.clone();
                    match self.sanitize_resource_for_apply(&mut resource_copy) {
                        Ok(()) => {
                            sanitization_stats.record_sanitized();
                            resource_copy
                        }
                        Err(e) => {
                            let resource_id = self.get_resource_identifier(resource, resource_type);
                            warn!("⚠️  Skipping {} - sanitization failed: {}", resource_id, e);
                            warn!(
                                "💡 Consider using --raw flag to collect original resource, then manually sanitize for kubectl apply"
                            );

                            sanitization_stats.record_skipped(resource_id);
                            continue; // Skip this resource
                        }
                    }
                } else {
                    sanitization_stats.record_raw();
                    resource.clone()
                };

                // Save the resource in requested format(s)
                match format {
                    "json" => {
                        let filename = format!("{}/{}.json", resource_dir, resource_name);
                        let content = serde_json::to_string_pretty(&final_resource)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "yaml" => {
                        let filename = format!("{}/{}.yaml", resource_dir, resource_name);
                        let content = serde_yaml::to_string(&final_resource)?;
                        fs::write(&filename, content)?;
                        saved_count += 1;
                    }
                    "both" => {
                        let json_file = format!("{}/{}.json", resource_dir, resource_name);
                        let yaml_file = format!("{}/{}.yaml", resource_dir, resource_name);

                        let json_content = serde_json::to_string_pretty(&final_resource)?;
                        let yaml_content = serde_yaml::to_string(&final_resource)?;

                        fs::write(&json_file, json_content)?;
                        fs::write(&yaml_file, yaml_content)?;
                        saved_count += 1;
                    }
                    _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
                }
            }
        }

        if saved_count > 0 {
            let sanitization_info = if sanitize {
                format!(" (sanitized for kubectl apply)")
            } else {
                format!(" (raw)")
            };

            let location_info = if is_custom_resource {
                format!(" to {}/custom-resources/", namespace)
            } else {
                format!(" to {}/", namespace)
            };

            info!(
                "💾 Saved {} {}{} {}",
                saved_count, resource_type, sanitization_info, location_info
            );
        }

        Ok((saved_count, sanitization_stats))
    }

    /// Enhanced summary creation that includes optional SUSE Edge analysis
    pub fn create_enhanced_summary(
        &self,
        output_dir: &str,
        namespace_stats: &[NamespaceStats],
        cluster_stats: &std::collections::HashMap<String, usize>,
        sanitization_stats: &SanitizationStats,
        raw_mode: bool,
        suse_edge_analysis: Option<&SuseEdgeAnalysis>, // NEW parameter
    ) -> Result<()> {
        // Calculate totals for cluster overview (existing logic)
        let mut total_namespaced_resources = 0;
        let mut active_namespaces = Vec::new();

        for stats in namespace_stats {
            let namespace_total = stats.total_resources();
            if namespace_total > 0 {
                total_namespaced_resources += namespace_total;
                active_namespaces.push((stats.namespace.clone(), namespace_total));
            }
        }

        let mut cluster_resource_map = std::collections::HashMap::new();
        let mut total_cluster_resources = 0;

        for (resource_type, count) in cluster_stats {
            if *count > 0 {
                cluster_resource_map.insert(resource_type.clone(), *count);
                total_cluster_resources += count;
            }
        }

        let grand_total = total_namespaced_resources + total_cluster_resources;

        // Build namespace details
        let mut namespace_details = serde_json::Map::new();
        for stats in namespace_stats {
            if stats.total_resources() > 0 {
                let primary_purpose = Self::determine_namespace_purpose(&stats.namespace);
                namespace_details.insert(
                    stats.namespace.clone(),
                    serde_json::json!({
                        "resources": stats.total_resources(),
                        "primary": primary_purpose
                    }),
                );
            }
        }

        // Calculate resource highlights (existing logic)
        let (workload_resources, security_resources, configuration_resources, networking_resources) =
            self.calculate_resource_highlights(namespace_stats);

        let mut resource_highlights = serde_json::Map::new();

        // Only include categories with resources
        if workload_resources.total > 0 {
            let mut workloads = serde_json::Map::new();
            if workload_resources.pods > 0 {
                workloads.insert("pods".to_string(), workload_resources.pods.into());
            }
            if workload_resources.deployments > 0 {
                workloads.insert(
                    "deployments".to_string(),
                    workload_resources.deployments.into(),
                );
            }
            if workload_resources.jobs > 0 {
                workloads.insert("jobs".to_string(), workload_resources.jobs.into());
            }
            if workload_resources.daemonsets > 0 {
                workloads.insert(
                    "daemon_sets".to_string(),
                    workload_resources.daemonsets.into(),
                );
            }
            if workload_resources.statefulsets > 0 {
                workloads.insert(
                    "stateful_sets".to_string(),
                    workload_resources.statefulsets.into(),
                );
            }
            if workload_resources.cronjobs > 0 {
                workloads.insert("cron_jobs".to_string(), workload_resources.cronjobs.into());
            }
            if workload_resources.replicasets > 0 {
                workloads.insert(
                    "replica_sets".to_string(),
                    workload_resources.replicasets.into(),
                );
            }

            if !workloads.is_empty() {
                resource_highlights.insert(
                    "workloads".to_string(),
                    serde_json::Value::Object(workloads),
                );
            }
        }

        if security_resources.total > 0 {
            let mut security = serde_json::Map::new();
            if security_resources.service_accounts > 0 {
                security.insert(
                    "service_accounts".to_string(),
                    security_resources.service_accounts.into(),
                );
            }
            security.insert(
                "total_rbac_resources".to_string(),
                security_resources.total.into(),
            );
            resource_highlights.insert("security".to_string(), serde_json::Value::Object(security));
        }

        if configuration_resources.total > 0 {
            let mut config = serde_json::Map::new();
            if configuration_resources.configmaps > 0 {
                config.insert(
                    "config_maps".to_string(),
                    configuration_resources.configmaps.into(),
                );
            }
            if configuration_resources.secrets > 0 {
                config.insert(
                    "secrets".to_string(),
                    configuration_resources.secrets.into(),
                );
            }
            resource_highlights.insert(
                "configuration".to_string(),
                serde_json::Value::Object(config),
            );
        }

        if networking_resources.total > 0 {
            let mut networking = serde_json::Map::new();
            if networking_resources.services > 0 {
                networking.insert("services".to_string(), networking_resources.services.into());
            }
            if networking_resources.endpoints > 0 {
                networking.insert(
                    "endpoints".to_string(),
                    networking_resources.endpoints.into(),
                );
            }
            if networking_resources.ingresses > 0 {
                networking.insert(
                    "ingresses".to_string(),
                    networking_resources.ingresses.into(),
                );
            }
            if networking_resources.networkpolicies > 0 {
                networking.insert(
                    "network_policies".to_string(),
                    networking_resources.networkpolicies.into(),
                );
            }
            resource_highlights.insert(
                "networking".to_string(),
                serde_json::Value::Object(networking),
            );
        }

        // Count directory structure (only non-empty)
        let cluster_dir_types = cluster_resource_map.len();
        let mut namespace_dir_info = Vec::new();
        for stats in namespace_stats {
            if stats.total_resources() > 0 {
                let non_empty_types = self.count_non_empty_resource_types(stats);
                namespace_dir_info.push(format!(
                    "{}/ ({} resource types)",
                    stats.namespace, non_empty_types
                ));
            }
        }

        // Build sanitization section (existing logic)
        let mut sanitization_section = serde_json::Map::new();
        if raw_mode {
            sanitization_section.insert("mode".to_string(), "raw".into());
            sanitization_section.insert("kubectl_ready".to_string(), false.into());
            sanitization_section.insert("note".to_string(), "Resources collected as-is from cluster. May require manual sanitization for kubectl apply.".into());
        } else {
            sanitization_section.insert("mode".to_string(), "sanitized".into());
            sanitization_section.insert("kubectl_ready".to_string(), true.into());
            sanitization_section.insert(
                "total_processed".to_string(),
                sanitization_stats.total_processed.into(),
            );
            sanitization_section.insert(
                "successfully_sanitized".to_string(),
                sanitization_stats.total_sanitized.into(),
            );

            if sanitization_stats.total_skipped > 0 {
                sanitization_section.insert(
                    "skipped_count".to_string(),
                    sanitization_stats.total_skipped.into(),
                );
                sanitization_section.insert("note".to_string(), "Some resources were skipped due to sanitization issues. Use --raw to collect all resources.".into());
            } else {
                sanitization_section.insert(
                    "note".to_string(),
                    "All resources successfully sanitized for kubectl apply.".into(),
                );
            }
        }

        // Build the summary with optional SUSE Edge section
        let mut summary = serde_json::json!({
            "📋 collection_info": {
                "timestamp": self.timestamp.to_rfc3339(),
                "tool": "ketchup",
                "version": env!("CARGO_PKG_VERSION")
            },
            "📊 cluster_overview": {
                "total_resources": grand_total,
                "namespaces": active_namespaces.len(),
                "cluster_resources": total_cluster_resources,
                "namespaced_resources": total_namespaced_resources
            },
            "✨ sanitization": sanitization_section,
            "☸️ cluster_resources": cluster_resource_map,
            "🏢 namespaces": namespace_details,
            "🎯 resource_highlights": resource_highlights,
            "📁 output_structure": {
                "total_files": grand_total,
                "formats": ["yaml"],
                "compression": "gzip",
                "directory_structure": {
                    "cluster_wide": format!("cluster-wide/ ({} resource types)", cluster_dir_types),
                    "namespaces": namespace_dir_info
                }
            }
        });

        // Add SUSE Edge analysis if present
        if let Some(edge_analysis) = suse_edge_analysis {
            let edge_section = self.build_suse_edge_section(edge_analysis)?;
            summary
                .as_object_mut()
                .unwrap()
                .insert("🍅 suse_edge_analysis".to_string(), edge_section);
        }

        // Create summary file
        let filename = format!("{}/collection-summary.yaml", output_dir);
        info!("📋 Creating collection summary: {}", filename);

        let mut summary_content = String::new();
        summary_content.push_str("# 🍅 KETCHUP CLUSTER COLLECTION SUMMARY\n");
        summary_content.push_str(&format!("# Generated: {}\n", self.timestamp.to_rfc3339()));

        if let Some(edge_analysis) = suse_edge_analysis {
            summary_content.push_str(&format!(
                "# SUSE Edge Detection: {} confidence\n",
                edge_analysis.confidence
            ));
            if let Some(ref k8s_dist) = edge_analysis.kubernetes_distribution {
                summary_content.push_str(&format!("# Kubernetes Distribution: {}\n", k8s_dist));
            }
        }

        if raw_mode {
            summary_content.push_str("# Mode: RAW (unsanitized resources)\n");
        } else {
            summary_content.push_str("# Mode: SANITIZED (kubectl apply ready)\n");
        }
        summary_content.push_str("# =======================================\n\n");

        let yaml_content =
            serde_yaml::to_string(&summary).context("Failed to serialize summary to YAML")?;

        // Add spacing between sections
        let spaced_yaml = yaml_content
            .replace("📋 collection_info:", "\n📋 collection_info:")
            .replace("📊 cluster_overview:", "\n📊 cluster_overview:")
            .replace("✨ sanitization:", "\n✨ sanitization:")
            .replace("☸️ cluster_resources:", "\n☸️ cluster_resources:")
            .replace("🏢 namespaces:", "\n🏢 namespaces:")
            .replace("🎯 resource_highlights:", "\n🎯 resource_highlights:")
            .replace("🍅 suse_edge_analysis:", "\n🍅 suse_edge_analysis:")
            .replace("📁 output_structure:", "\n📁 output_structure:");

        summary_content.push_str(&spaced_yaml);
        fs::write(&filename, summary_content).context("Failed to write YAML summary file")?;

        // Create separate detailed SUSE Edge report if analysis was performed
        if let Some(edge_analysis) = suse_edge_analysis {
            self.create_detailed_suse_edge_report(output_dir, edge_analysis)?;
        }

        Ok(())
    }

    /// Build SUSE Edge section for the main summary
    fn build_suse_edge_section(
        &self,
        edge_analysis: &SuseEdgeAnalysis,
    ) -> Result<serde_json::Value> {
        let mut components_by_category = std::collections::HashMap::new();

        // Group components by category
        for component in &edge_analysis.components {
            components_by_category
                .entry(component.category.clone())
                .or_insert_with(Vec::new)
                .push(serde_json::json!({
                    "name": component.name,
                    "version": component.version,
                    "chart_version": component.chart_version,
                    "locations_count": component.found_in.len()
                }));
        }

        Ok(serde_json::json!({
            "detection_summary": {
                "total_components": edge_analysis.total_components,
                "confidence_level": edge_analysis.confidence,
                "edge_version": edge_analysis.edge_version_detected,
                "deployment_type": edge_analysis.deployment_type,
                "kubernetes_distribution": edge_analysis.kubernetes_distribution
            },
            "components_by_category": components_by_category,
            "quick_assessment": self.generate_quick_assessment(edge_analysis)
        }))
    }

    /// Create detailed SUSE Edge report as separate file
    fn create_detailed_suse_edge_report(
        &self,
        output_dir: &str,
        edge_analysis: &SuseEdgeAnalysis,
    ) -> Result<()> {
        let filename = format!("{}/suse-edge-analysis.yaml", output_dir);
        info!("🍅 Creating detailed SUSE Edge analysis: {}", filename);

        let mut report_content = String::new();
        report_content.push_str("# 🍅 SUSE EDGE COMPONENT ANALYSIS\n");
        report_content.push_str(&format!("# Generated: {}\n", self.timestamp.to_rfc3339()));
        report_content.push_str(&format!(
            "# Confidence: {} ({})\n",
            edge_analysis.confidence,
            self.get_confidence_description(&edge_analysis.confidence)
        ));
        if let Some(ref version) = edge_analysis.edge_version_detected {
            report_content.push_str(&format!("# SUSE Edge Version: {}\n", version));
        }
        report_content.push_str("# ========================================\n\n");

        // Build detailed report structure
        let mut detailed_components = Vec::new();
        for component in &edge_analysis.components {
            detailed_components.push(serde_json::json!({
                "name": component.name,
                "version": component.version,
                "chart_version": component.chart_version,
                "category": component.category,
                "found_in": component.found_in,
                "detection_confidence": self.assess_component_confidence(component)
            }));
        }

        let detailed_report = serde_json::json!({
            "🎯 analysis_summary": {
                "total_components_detected": edge_analysis.total_components,
                "confidence_level": edge_analysis.confidence,
                "edge_version_detected": edge_analysis.edge_version_detected,
                "deployment_type": edge_analysis.deployment_type,
                "kubernetes_distribution": edge_analysis.kubernetes_distribution,
                "analysis_timestamp": self.timestamp.to_rfc3339()
            },
            "📊 component_breakdown": {
                "by_category": self.group_components_by_category(&edge_analysis.components),
                "version_summary": self.summarize_component_versions(&edge_analysis.components)
            },
            "🔍 detailed_findings": detailed_components,
            "💡 recommendations": self.generate_recommendations(edge_analysis),
            "⚠️ notes": [
                "This analysis is based on detected Kubernetes resources and container images",
                "Some components may be present but not detected if they use non-standard configurations",
                "Confidence level indicates the reliability of the detection based on found evidence"
            ]
        });

        let yaml_content = serde_yaml::to_string(&detailed_report)?;
        let spaced_yaml = yaml_content
            .replace("🎯 analysis_summary:", "\n🎯 analysis_summary:")
            .replace("📊 component_breakdown:", "\n📊 component_breakdown:")
            .replace("🔍 detailed_findings:", "\n🔍 detailed_findings:")
            .replace("💡 recommendations:", "\n💡 recommendations:")
            .replace("⚠️ notes:", "\n⚠️ notes:");

        report_content.push_str(&spaced_yaml);
        fs::write(&filename, report_content)?;

        Ok(())
    }

    // Helper methods for SUSE Edge analysis

    fn generate_quick_assessment(&self, edge_analysis: &SuseEdgeAnalysis) -> Vec<String> {
        let mut assessment = Vec::new();

        if edge_analysis.total_components >= 5 {
            assessment.push("Full SUSE Edge deployment detected".to_string());
        } else if edge_analysis.total_components >= 2 {
            assessment.push("Partial SUSE Edge deployment detected".to_string());
        } else {
            assessment.push("Limited SUSE Edge components detected".to_string());
        }

        if edge_analysis.kubernetes_distribution.is_some() {
            assessment.push("SUSE Kubernetes distribution in use".to_string());
        }

        assessment
    }

    fn get_confidence_description(&self, confidence: &str) -> &'static str {
        match confidence {
            "Very High" => "Strong evidence of SUSE Edge deployment",
            "High" => "Good evidence of SUSE Edge components",
            "Medium" => "Some SUSE Edge components detected",
            "Low" => "Few SUSE Edge indicators found",
            _ => "Minimal SUSE Edge presence",
        }
    }

    fn assess_component_confidence(&self, component: &SuseEdgeComponent) -> String {
        if component.version.is_some() && component.found_in.len() > 1 {
            "High".to_string()
        } else if component.version.is_some() || component.found_in.len() > 1 {
            "Medium".to_string()
        } else {
            "Low".to_string()
        }
    }

    fn group_components_by_category(&self, components: &[SuseEdgeComponent]) -> serde_json::Value {
        let mut by_category = std::collections::HashMap::new();

        for component in components {
            by_category
                .entry(&component.category)
                .or_insert_with(Vec::new)
                .push(&component.name);
        }

        serde_json::to_value(by_category).unwrap_or_default()
    }

    fn summarize_component_versions(&self, components: &[SuseEdgeComponent]) -> serde_json::Value {
        let mut version_summary = std::collections::HashMap::new();

        for component in components {
            if let Some(ref version) = component.version {
                version_summary.insert(&component.name, version);
            }
        }

        serde_json::to_value(version_summary).unwrap_or_default()
    }

    fn generate_recommendations(&self, edge_analysis: &SuseEdgeAnalysis) -> Vec<String> {
        let mut recommendations = Vec::new();

        if edge_analysis.total_components < 3 {
            recommendations.push(
                "Consider reviewing SUSE Edge documentation for complete component setup"
                    .to_string(),
            );
        }

        if edge_analysis.confidence == "Low" || edge_analysis.confidence == "Minimal" {
            recommendations.push(
                "Some components may not be detected due to custom configurations".to_string(),
            );
        }

        if edge_analysis.kubernetes_distribution.is_none() {
            recommendations.push(
                "Unable to detect Kubernetes distribution - manual verification recommended"
                    .to_string(),
            );
        }

        recommendations.push(
            "Review the detailed findings for specific component locations and versions"
                .to_string(),
        );

        recommendations
    }

    // Helper function to determine namespace purpose
    fn determine_namespace_purpose(namespace: &str) -> &'static str {
        match namespace {
            "kube-system" => "workloads + system config",
            "default" => "user workloads",
            "kube-public" => "cluster info",
            "kube-node-lease" => "node coordination",
            _ if namespace.starts_with("istio") => "service mesh",
            _ if namespace.contains("monitoring") => "observability",
            _ if namespace.contains("ingress") => "traffic routing",
            _ => "application workloads",
        }
    }

    // Helper function to calculate resource highlights
    fn calculate_resource_highlights(
        &self,
        namespace_stats: &[NamespaceStats],
    ) -> (
        WorkloadResources,
        SecurityResources,
        ConfigurationResources,
        NetworkingResources,
    ) {
        let mut workloads = WorkloadResources::default();
        let mut security = SecurityResources::default();
        let mut configuration = ConfigurationResources::default();
        let mut networking = NetworkingResources::default();

        for stats in namespace_stats {
            // Workloads
            workloads.pods += stats.pods;
            workloads.deployments += stats.deployments;
            workloads.jobs += stats.jobs;
            workloads.daemonsets += stats.daemonsets;
            workloads.statefulsets += stats.statefulsets;
            workloads.cronjobs += stats.cronjobs;
            workloads.replicasets += stats.replicasets;

            // Security/RBAC
            security.service_accounts += stats.serviceaccounts;
            security.roles += stats.roles;
            security.rolebindings += stats.rolebindings;

            // Configuration
            configuration.configmaps += stats.configmaps;
            configuration.secrets += stats.secrets;

            // Networking
            networking.services += stats.services;
            networking.endpoints += stats.endpoints;
            networking.ingresses += stats.ingresses;
            networking.networkpolicies += stats.networkpolicies;
        }

        workloads.total = workloads.pods
            + workloads.deployments
            + workloads.jobs
            + workloads.daemonsets
            + workloads.statefulsets
            + workloads.cronjobs
            + workloads.replicasets;

        security.total = security.service_accounts + security.roles + security.rolebindings;
        configuration.total = configuration.configmaps + configuration.secrets;
        networking.total = networking.services
            + networking.endpoints
            + networking.ingresses
            + networking.networkpolicies;

        (workloads, security, configuration, networking)
    }

    // Helper function to count non-empty resource types per namespace
    fn count_non_empty_resource_types(&self, stats: &NamespaceStats) -> usize {
        let mut count = 0;
        if stats.pods > 0 {
            count += 1;
        }
        if stats.services > 0 {
            count += 1;
        }
        if stats.deployments > 0 {
            count += 1;
        }
        if stats.configmaps > 0 {
            count += 1;
        }
        if stats.secrets > 0 {
            count += 1;
        }
        if stats.ingresses > 0 {
            count += 1;
        }
        if stats.pvcs > 0 {
            count += 1;
        }
        if stats.networkpolicies > 0 {
            count += 1;
        }
        if stats.replicasets > 0 {
            count += 1;
        }
        if stats.daemonsets > 0 {
            count += 1;
        }
        if stats.statefulsets > 0 {
            count += 1;
        }
        if stats.jobs > 0 {
            count += 1;
        }
        if stats.cronjobs > 0 {
            count += 1;
        }
        if stats.serviceaccounts > 0 {
            count += 1;
        }
        if stats.roles > 0 {
            count += 1;
        }
        if stats.rolebindings > 0 {
            count += 1;
        }
        if stats.resourcequotas > 0 {
            count += 1;
        }
        if stats.limitranges > 0 {
            count += 1;
        }
        if stats.horizontalpodautoscalers > 0 {
            count += 1;
        }
        if stats.poddisruptionbudgets > 0 {
            count += 1;
        }
        if stats.endpoints > 0 {
            count += 1;
        }
        if stats.endpointslices > 0 {
            count += 1;
        }
        count
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
        info!("📦 Creating archive: {}", archive_name);

        let tar_gz =
            std::fs::File::create(&archive_name).context("Failed to create archive file")?;
        let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);

        tar.append_dir_all(".", output_dir)
            .context("Failed to add directory to archive")?;
        tar.finish().context("Failed to finalize archive")?;
        info!("✅ Archive created: {}", archive_name);

        Ok(archive_name)
    }
}
