use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::output::{SuseEdgeAnalysis, SuseEdgeComponent};

/// Create an empty analysis result to indicate no SUSE Edge components were found
pub fn create_empty_analysis() -> SuseEdgeAnalysis {
    SuseEdgeAnalysis {
        components: Vec::new(),
        total_components: 0,
        confidence: "None - Standard Kubernetes".to_string(),
        deployment_type: "Standard Kubernetes Cluster".to_string(),
        kubernetes_distribution: None,
    }
}

/// Comprehensive SUSE Edge component detection with clean, precise logic
pub fn detect_suse_edge_components(
    namespaced_resources: &HashMap<String, Vec<Value>>,
    cluster_resources: &HashMap<String, Vec<Value>>,
) -> Option<SuseEdgeAnalysis> {
    let mut detected_components = Vec::new();
    let mut detection_confidence = 0;
    let mut kubernetes_distribution = None;

    info!("🔍 Performing SUSE Edge component scan...");

    // 1. Detect Kubernetes Distribution (conservative approach)
    if let Some(k8s_dist) =
        detect_kubernetes_distribution_precise(namespaced_resources, cluster_resources)
    {
        detection_confidence += 20;
        kubernetes_distribution = Some(k8s_dist.name.clone());
        detected_components.push(k8s_dist);
    }

    // 2. Detect SUSE Edge specific components via CRDs (most reliable)
    if let Some(edge_crds) = detect_suse_edge_crds_precise(cluster_resources) {
        detection_confidence += 15 * edge_crds.len() as u32;
        detected_components.extend(edge_crds);
    }

    // 3. Detect core SUSE Edge deployments (strict matching)
    if let Some(core_components) = detect_core_suse_components(namespaced_resources) {
        detection_confidence += 10 * core_components.len() as u32;
        detected_components.extend(core_components);
    }

    // 4. Detect SUSE registry usage (light indicator)
    if let Some(registry_component) = detect_suse_registry_usage_precise(namespaced_resources) {
        detection_confidence += 5;
        detected_components.push(registry_component);
    }

    if detected_components.is_empty() {
        debug!("No SUSE Edge components detected");
        return None;
    }

    let total_components = detected_components.len();
    let confidence_level =
        determine_confidence_level_conservative(detection_confidence, total_components);
    let deployment_type = determine_deployment_type_precise(&detected_components);

    info!("🎯 SUSE Edge Detection Summary:");
    info!("   📊 Components found: {}", total_components);
    info!("   🎯 Confidence level: {}", confidence_level);

    Some(SuseEdgeAnalysis {
        components: detected_components,
        total_components,
        confidence: confidence_level,
        deployment_type,
        kubernetes_distribution,
    })
}

/// Precise Kubernetes distribution detection - only detect what we're certain about
fn detect_kubernetes_distribution_precise(
    _namespaced_resources: &HashMap<String, Vec<Value>>,
    cluster_resources: &HashMap<String, Vec<Value>>,
) -> Option<SuseEdgeComponent> {
    // Check for K3s via specific cluster roles AND get version from nodes
    if let Some(cluster_roles) = cluster_resources.get("clusterroles") {
        for role in cluster_roles {
            if let Some(name) = get_resource_name(role) {
                if name == "system:k3s-controller" {
                    // Found K3s, now get the actual version from nodes
                    let version = if let Some(nodes) = cluster_resources.get("nodes") {
                        extract_k3s_version_from_nodes(nodes).unwrap_or("detected".to_string())
                    } else {
                        "detected".to_string()
                    };

                    return Some(SuseEdgeComponent {
                        name: "K3s".to_string(),
                        version: Some(version),
                        found_in: vec!["Detected via cluster roles and node version".to_string()],
                        category: "Core".to_string(),
                    });
                }
            }
        }
    }

    // Check for RKE2 via specific node labels AND get version
    if let Some(nodes) = cluster_resources.get("nodes") {
        for node in nodes {
            if let Some(labels) = node
                .get("metadata")
                .and_then(|m| m.get("labels"))
                .and_then(|l| l.as_object())
            {
                if labels.contains_key("rke2.io/hostname")
                    || labels
                        .values()
                        .any(|v| v.as_str().map_or(false, |s| s.contains("rke2")))
                {
                    let version =
                        extract_rke2_version_precise(nodes).unwrap_or("detected".to_string());

                    return Some(SuseEdgeComponent {
                        name: "RKE2".to_string(),
                        version: Some(version),
                        found_in: vec!["Detected via node labels and version".to_string()],
                        category: "Core".to_string(),
                    });
                }
            }
        }

        // Fallback: Check if any node has RKE2 in the kubelet version
        for node in nodes {
            if let Some(version) = node
                .get("status")
                .and_then(|s| s.get("nodeInfo"))
                .and_then(|ni| ni.get("kubeletVersion"))
                .and_then(|kv| kv.as_str())
            {
                if version.contains("rke2") {
                    return Some(SuseEdgeComponent {
                        name: "RKE2".to_string(),
                        version: Some(version.to_string()),
                        found_in: vec!["Detected via kubelet version".to_string()],
                        category: "Core".to_string(),
                    });
                }
            }
        }
    }

    None
}

/// Precise CRD detection - only well-known SUSE Edge CRDs
fn detect_suse_edge_crds_precise(
    cluster_resources: &HashMap<String, Vec<Value>>,
) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();

    if let Some(crds) = cluster_resources.get("customresourcedefinitions") {
        let suse_edge_crds = [
            ("longhorn.io", "SUSE Storage (Longhorn)", "Storage"),
            ("neuvector.com", "SUSE Security (NeuVector)", "Security"),
            ("kubevirt.io", "KubeVirt", "Virtualization"),
            (
                "cdi.kubevirt.io",
                "Containerized Data Importer",
                "Virtualization",
            ),
            ("metal3.io", "Metal3", "Infrastructure"),
            ("elemental.cattle.io", "Elemental", "Infrastructure"),
            ("akri.sh", "Akri", "IoT"),
        ];

        for (crd_group, component_name, category) in &suse_edge_crds {
            let count = crds
                .iter()
                .filter(|crd| get_resource_name(crd).map_or(false, |name| name.contains(crd_group)))
                .count();

            if count > 0 {
                components.push(SuseEdgeComponent {
                    name: component_name.to_string(),
                    version: None,
                    found_in: vec![format!("{} CRDs detected", count)],
                    category: category.to_string(),
                });
            }
        }
    }

    if components.is_empty() {
        None
    } else {
        Some(components)
    }
}

/// Detect core SUSE components via specific deployments/namespaces
fn detect_core_suse_components(
    namespaced_resources: &HashMap<String, Vec<Value>>,
) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();

    // Check for Rancher in cattle-system namespace
    if let Some(deployments) = namespaced_resources.get("deployments") {
        for deployment in deployments {
            if let Some(namespace) = get_resource_namespace(deployment) {
                if namespace == "cattle-system" {
                    if let Some(name) = get_resource_name(deployment) {
                        if name.contains("rancher") {
                            components.push(SuseEdgeComponent {
                                name: "SUSE Rancher Prime".to_string(),
                                version: extract_version_from_deployment(deployment),
                                found_in: vec![format!("cattle-system/{}", name)],
                                category: "Management".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Check for Longhorn in longhorn-system namespace
    if let Some(deployments) = namespaced_resources.get("deployments") {
        for deployment in deployments {
            if let Some(namespace) = get_resource_namespace(deployment) {
                if namespace == "longhorn-system" {
                    if let Some(name) = get_resource_name(deployment) {
                        if name.contains("longhorn") {
                            components.push(SuseEdgeComponent {
                                name: "SUSE Storage (Longhorn)".to_string(),
                                version: extract_version_from_deployment(deployment),
                                found_in: vec![format!("longhorn-system/{}", name)],
                                category: "Storage".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    if components.is_empty() {
        None
    } else {
        Some(components)
    }
}

/// Precise SUSE registry detection
fn detect_suse_registry_usage_precise(
    namespaced_resources: &HashMap<String, Vec<Value>>,
) -> Option<SuseEdgeComponent> {
    let mut suse_image_count = 0;
    let suse_registries = ["registry.suse.com", "registry.opensuse.org"];

    for (resource_type, resources) in namespaced_resources {
        if resource_type == "pods" || resource_type == "deployments" {
            for resource in resources {
                if let Some(containers) = extract_containers_from_resource(resource) {
                    for container in containers {
                        if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                            if suse_registries.iter().any(|reg| image.starts_with(reg)) {
                                suse_image_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    if suse_image_count > 0 {
        Some(SuseEdgeComponent {
            name: "SUSE Container Images".to_string(),
            version: None,
            found_in: vec![format!("{} SUSE images in use", suse_image_count)],
            category: "Infrastructure".to_string(),
        })
    } else {
        None
    }
}

// ===== Helper Functions =====

fn extract_k3s_version_from_nodes(nodes: &[Value]) -> Option<String> {
    for node in nodes {
        if let Some(version) = node
            .get("status")
            .and_then(|s| s.get("nodeInfo"))
            .and_then(|ni| ni.get("kubeletVersion"))
            .and_then(|kv| kv.as_str())
        {
            // K3s versions look like: v1.30.8+k3s1
            if version.contains("k3s") {
                return Some(version.to_string());
            }
        }
    }
    Some("detected".to_string())
}

fn extract_rke2_version_precise(nodes: &[Value]) -> Option<String> {
    for node in nodes {
        if let Some(version) = node
            .get("status")
            .and_then(|s| s.get("nodeInfo"))
            .and_then(|ni| ni.get("kubeletVersion"))
            .and_then(|kv| kv.as_str())
        {
            // Return the actual kubelet version which includes K8s distribution info
            return Some(version.to_string());
        }
    }
    Some("detected".to_string())
}

fn extract_version_from_deployment(deployment: &Value) -> Option<String> {
    if let Some(containers) = extract_containers_from_resource(deployment) {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                if let Some(version) = extract_semantic_version(image) {
                    return Some(version);
                }
            }
        }
    }
    None
}

fn extract_semantic_version(image: &str) -> Option<String> {
    if let Some(tag) = image.split(':').last() {
        // Only return if it looks like a semantic version
        if tag.starts_with('v') && tag.contains('.') && !tag.contains("sha256") {
            return Some(tag.to_string());
        }
    }
    None
}

fn extract_containers_from_resource(resource: &Value) -> Option<&Vec<Value>> {
    resource
        .get("spec")
        .and_then(|s| s.get("template"))
        .and_then(|t| t.get("spec"))
        .and_then(|s| s.get("containers"))
        .and_then(|c| c.as_array())
        .or_else(|| {
            resource
                .get("spec")
                .and_then(|s| s.get("containers"))
                .and_then(|c| c.as_array())
        })
}

fn determine_confidence_level_conservative(
    confidence_score: u32,
    component_count: usize,
) -> String {
    match (confidence_score, component_count) {
        (60.., 5..) => "Very High".to_string(),
        (40.., 3..) => "High".to_string(),
        (20.., 2..) => "Medium".to_string(),
        (10.., 1..) => "Low".to_string(),
        _ => "Minimal".to_string(),
    }
}

fn determine_deployment_type_precise(components: &[SuseEdgeComponent]) -> String {
    let has_rancher = components.iter().any(|c| c.name.contains("Rancher"));
    let has_metal3 = components.iter().any(|c| c.name.contains("Metal3"));
    let has_elemental = components.iter().any(|c| c.name.contains("Elemental"));
    let has_k8s_dist = components
        .iter()
        .any(|c| c.name == "K3s" || c.name == "RKE2");

    match (has_rancher, has_metal3, has_elemental, has_k8s_dist) {
        (true, true, _, _) => "Management Cluster".to_string(),
        (true, false, true, _) => "Elemental Management Cluster".to_string(),
        (true, false, false, _) => "Rancher Management Cluster".to_string(),
        (false, _, _, true) => "Downstream Cluster".to_string(),
        _ => "Standalone Cluster".to_string(),
    }
}

fn get_resource_name(resource: &Value) -> Option<String> {
    resource
        .get("metadata")?
        .get("name")?
        .as_str()
        .map(|s| s.to_string())
}

fn get_resource_namespace(resource: &Value) -> Option<String> {
    resource
        .get("metadata")?
        .get("namespace")?
        .as_str()
        .map(|s| s.to_string())
}
