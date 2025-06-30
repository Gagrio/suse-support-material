use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::output::{SuseEdgeAnalysis, SuseEdgeComponent};

/// Official SUSE Edge 3.3 Component Registry
/// Based on: https://documentation.suse.com/suse-edge/3.3/html/edge/id-release-notes.html#id-component-versions
const SUSE_EDGE_COMPONENTS: &[SuseEdgeComponentInfo] = &[
    // Core Platform
    SuseEdgeComponentInfo { name: "SUSE Linux Micro", version: "6.1", patterns: &["sle-micro", "linux-micro"], namespaces: &["kube-system"], category: "Core" },
    SuseEdgeComponentInfo { name: "SUSE Multi-Linux Manager", version: "5.0.3", patterns: &["suse-manager", "suma"], namespaces: &["suse-manager"], category: "Core" },
    
    // Kubernetes Distributions
    SuseEdgeComponentInfo { name: "K3s", version: "1.32.4", patterns: &["k3s"], namespaces: &["kube-system"], category: "Core" },
    SuseEdgeComponentInfo { name: "RKE2", version: "1.32.4", patterns: &["rke2"], namespaces: &["kube-system"], category: "Core" },
    
    // Management & Orchestration
    SuseEdgeComponentInfo { name: "SUSE Rancher Prime", version: "2.11.2", patterns: &["rancher"], namespaces: &["cattle-system", "rancher-operator-system"], category: "Management" },
    SuseEdgeComponentInfo { name: "Rancher Turtles (CAPI)", version: "0.20.0", patterns: &["rancher-turtles", "cluster-api"], namespaces: &["rancher-turtles-system", "capi-system"], category: "Management" },
    
    // Storage & Security
    SuseEdgeComponentInfo { name: "SUSE Storage (Longhorn)", version: "1.8.1", patterns: &["longhorn"], namespaces: &["longhorn-system"], category: "Storage" },
    SuseEdgeComponentInfo { name: "SUSE Security (NeuVector)", version: "5.4.4", patterns: &["neuvector"], namespaces: &["neuvector"], category: "Security" },
    
    // Infrastructure Management
    SuseEdgeComponentInfo { name: "Metal3", version: "0.11.5", patterns: &["metal3", "baremetal-operator", "ironic"], namespaces: &["metal3-system"], category: "Infrastructure" },
    SuseEdgeComponentInfo { name: "MetalLB", version: "0.14.9", patterns: &["metallb"], namespaces: &["metallb-system"], category: "Networking" },
    SuseEdgeComponentInfo { name: "Elemental", version: "1.6.8", patterns: &["elemental"], namespaces: &["cattle-elemental-system"], category: "Infrastructure" },
    
    // Virtualization Stack
    SuseEdgeComponentInfo { name: "KubeVirt", version: "1.4.0", patterns: &["kubevirt"], namespaces: &["kubevirt"], category: "Virtualization" },
    SuseEdgeComponentInfo { name: "KubeVirt Dashboard Extension", version: "1.3.2", patterns: &["kubevirt-dashboard"], namespaces: &["cattle-ui-plugin-system"], category: "Virtualization" },
    SuseEdgeComponentInfo { name: "Containerized Data Importer", version: "1.61.0", patterns: &["cdi"], namespaces: &["cdi"], category: "Virtualization" },
    
    // Networking
    SuseEdgeComponentInfo { name: "SR-IOV Network Operator", version: "1.5.0", patterns: &["sriov"], namespaces: &["sriov-network-operator"], category: "Networking" },
    SuseEdgeComponentInfo { name: "Endpoint Copier Operator", version: "0.2.0", patterns: &["endpoint-copier"], namespaces: &["endpoint-copier-operator"], category: "Networking" },
    
    // Lifecycle Management
    SuseEdgeComponentInfo { name: "System Upgrade Controller", version: "0.15.2", patterns: &["system-upgrade-controller"], namespaces: &["system-upgrade"], category: "Management" },
    SuseEdgeComponentInfo { name: "Upgrade Controller", version: "0.1.1", patterns: &["upgrade-controller"], namespaces: &["upgrade-controller-system"], category: "Management" },
    
    // Edge Tools & Extensions
    SuseEdgeComponentInfo { name: "Edge Image Builder", version: "1.2.1", patterns: &["edge-image-builder", "eib"], namespaces: &["eib-system"], category: "Tools" },
    SuseEdgeComponentInfo { name: "NM Configurator", version: "0.3.3", patterns: &["nm-configurator"], namespaces: &["kube-system"], category: "Tools" },
    SuseEdgeComponentInfo { name: "Elemental Dashboard Extension", version: "3.0.1", patterns: &["elemental-ui"], namespaces: &["cattle-ui-plugin-system"], category: "Tools" },
    SuseEdgeComponentInfo { name: "Kiwi Builder", version: "10.2.12.0", patterns: &["kiwi-builder"], namespaces: &["kiwi-system"], category: "Tools" },
    
    // Technology Previews
    SuseEdgeComponentInfo { name: "Akri (Tech Preview)", version: "0.12.20", patterns: &["akri"], namespaces: &["akri"], category: "IoT" },
];

#[derive(Debug)]
struct SuseEdgeComponentInfo {
    name: &'static str,
    version: &'static str,
    patterns: &'static [&'static str],
    namespaces: &'static [&'static str],
    category: &'static str,
}

/// Comprehensive SUSE Edge component detection
pub fn detect_suse_edge_components(
    namespaced_resources: &HashMap<String, Vec<Value>>,
    cluster_resources: &HashMap<String, Vec<Value>>,
) -> Option<SuseEdgeAnalysis> {
    let mut detected_components = Vec::new();
    let mut detection_confidence = 0;
    let mut kubernetes_distribution = None;

    info!("🔍 Performing comprehensive SUSE Edge component scan...");

    // 1. Detect Kubernetes Distribution (High Priority)
    if let Some(k8s_dist) = detect_kubernetes_distribution(namespaced_resources) {
        detection_confidence += 30; // High weight for K8s distribution
        if let Some(first_dist) = k8s_dist.first() {
            kubernetes_distribution = Some(first_dist.name.clone());
        }
        detected_components.extend(k8s_dist);
    }

    // 2. Detect Core SUSE Components
    for component_info in SUSE_EDGE_COMPONENTS {
        if let Some(component) = detect_component_by_patterns(
            namespaced_resources,
            cluster_resources,
            component_info,
        ) {
            detection_confidence += calculate_component_weight(component_info.name);
            detected_components.push(component);
        }
    }

    // 3. Detect Custom Resource Definitions (CRDs)
    if let Some(edge_crds) = detect_suse_edge_crds(cluster_resources) {
        detection_confidence += 10; // Moderate weight for CRDs
        detected_components.extend(edge_crds);
    }

    // 4. Detect Edge-specific patterns
    if let Some(edge_patterns) = detect_edge_specific_patterns(namespaced_resources) {
        detection_confidence += 5; // Lower weight for generic patterns
        detected_components.extend(edge_patterns);
    }

    if detected_components.is_empty() {
        debug!("No SUSE Edge components detected");
        return None;
    }

    let total_components = detected_components.len();
    let confidence_level = determine_confidence_level(detection_confidence, total_components);
    let deployment_type = determine_deployment_type(&detected_components);
    let edge_version = detect_edge_version(&detected_components);

    info!("🎯 SUSE Edge Detection Summary:");
    info!("   📊 Components found: {}", total_components);
    info!("   🎯 Confidence level: {}", confidence_level);
    
    // Log component breakdown
    for component in &detected_components {
        debug!("   ✅ {}: {}", component.name, 
               component.version.as_deref().unwrap_or("detected"));
    }

    Some(SuseEdgeAnalysis {
        components: detected_components,
        total_components,
        confidence: confidence_level,
        edge_version_detected: edge_version,
        deployment_type,
        kubernetes_distribution,
    })
}

/// Enhanced Kubernetes distribution detection
fn detect_kubernetes_distribution(namespaced_resources: &HashMap<String, Vec<Value>>) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();
    let mut found_distributions = HashMap::new();

    // Check pods, deployments, and daemonsets for K8s distribution indicators
    let resource_types = ["pods", "deployments", "daemonsets"];
    
    for resource_type in &resource_types {
        if let Some(resources) = namespaced_resources.get(*resource_type) {
            for resource in resources {
                if let Some(namespace) = get_resource_namespace(resource) {
                    if namespace != "kube-system" {
                        continue;
                    }

                    let resource_name = get_resource_name(resource).unwrap_or_default();
                    let location = format!("{}/{}/{}.yaml", namespace, resource_type, resource_name);

                    // Detect RKE2
                    if resource_name.contains("rke2") || contains_rke2_indicators(resource) {
                        let version = extract_rke2_version(resource).unwrap_or("detected".to_string());
                        found_distributions.entry("RKE2".to_string())
                            .or_insert_with(|| (version, Vec::new()))
                            .1.push(location);
                    }

                    // Detect K3s
                    if resource_name.contains("k3s") || contains_k3s_indicators(resource) {
                        let version = extract_k3s_version(resource).unwrap_or("detected".to_string());
                        found_distributions.entry("K3s".to_string())
                            .or_insert_with(|| (version, Vec::new()))
                            .1.push(location);
                    }
                }
            }
        }
    }

    // Convert to components
    for (name, (version, locations)) in found_distributions {
        components.push(SuseEdgeComponent {
            name,
            version: Some(version),
            chart_version: None,
            found_in: locations,
            category: "Core".to_string(),
        });
    }

    if components.is_empty() { None } else { Some(components) }
}

/// Generic component detection by patterns
fn detect_component_by_patterns(
    namespaced_resources: &HashMap<String, Vec<Value>>,
    cluster_resources: &HashMap<String, Vec<Value>>,
    component_info: &SuseEdgeComponentInfo,
) -> Option<SuseEdgeComponent> {
    let mut found_in = Vec::new();
    let mut detected_version = None;

    // Search in namespaced resources
    for (resource_type, resources) in namespaced_resources {
        for resource in resources {
            if matches_component_patterns(resource, component_info) {
                if let Some(namespace) = get_resource_namespace(resource) {
                    let resource_name = get_resource_name(resource).unwrap_or_default();
                    let location = format!("{}/{}/{}.yaml", namespace, resource_type, resource_name);
                    found_in.push(location);

                    // Try to extract version
                    if detected_version.is_none() {
                        detected_version = extract_version_from_resource(resource, component_info);
                    }
                }
            }
        }
    }

    // Search in cluster resources
    for (resource_type, resources) in cluster_resources {
        for resource in resources {
            if matches_component_patterns(resource, component_info) {
                let resource_name = get_resource_name(resource).unwrap_or_default();
                let location = format!("cluster-wide/{}/{}.yaml", resource_type, resource_name);
                found_in.push(location);

                if detected_version.is_none() {
                    detected_version = extract_version_from_resource(resource, component_info);
                }
            }
        }
    }

    if !found_in.is_empty() {
        Some(SuseEdgeComponent {
            name: component_info.name.to_string(),
            version: detected_version.or_else(|| Some(component_info.version.to_string())),
            chart_version: extract_helm_chart_version_from_locations(&found_in),
            found_in,
            category: component_info.category.to_string(),
        })
    } else {
        None
    }
}

/// Enhanced CRD detection for SUSE Edge
fn detect_suse_edge_crds(cluster_resources: &HashMap<String, Vec<Value>>) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();

    if let Some(crds) = cluster_resources.get("customresourcedefinitions") {
        let edge_crd_patterns = [
            ("k3s.cattle.io", "K3s Add-ons", "Core"),
            ("helm.cattle.io", "Helm Controller", "Management"),
            ("rke.cattle.io", "RKE2 Add-ons", "Core"),
            ("traefik.containo.us", "Traefik Ingress", "Networking"),
            ("traefik.io", "Traefik Ingress", "Networking"),
            ("longhorn.io", "SUSE Storage (Longhorn)", "Storage"),
            ("neuvector.com", "SUSE Security (NeuVector)", "Security"),
            ("metallb.io", "MetalLB", "Networking"),
            ("kubevirt.io", "KubeVirt", "Virtualization"),
            ("cdi.kubevirt.io", "Containerized Data Importer", "Virtualization"),
            ("elemental.cattle.io", "Elemental", "Infrastructure"),
            ("metal3.io", "Metal3", "Infrastructure"),
            ("akri.sh", "Akri (Tech Preview)", "IoT"),
            ("sriovnetwork.openshift.io", "SR-IOV Network Operator", "Networking"),
            ("upgrade.cattle.io", "System Upgrade Controller", "Management"),
            ("lifecycle.suse.com", "Upgrade Controller", "Management"),
        ];

        for (pattern, component_name, category) in &edge_crd_patterns {
            let mut found_in = Vec::new();
            
            for crd in crds {
                let crd_name = get_resource_name(crd).unwrap_or_default();
                if crd_name.contains(pattern) {
                    let location = format!("cluster-wide/customresourcedefinitions/{}.yaml", crd_name);
                    found_in.push(location);
                }
            }

            if !found_in.is_empty() {
                components.push(SuseEdgeComponent {
                    name: component_name.to_string(),
                    version: None,
                    chart_version: None,
                    found_in,
                    category: category.to_string(),
                });
            }
        }
    }

    if components.is_empty() { None } else { Some(components) }
}

/// Detect Edge-specific patterns and configurations
fn detect_edge_specific_patterns(namespaced_resources: &HashMap<String, Vec<Value>>) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();

    // Check for SUSE container registry usage
    if let Some(suse_registry_usage) = detect_suse_registry_usage(namespaced_resources) {
        components.push(suse_registry_usage);
    }

    if components.is_empty() { None } else { Some(components) }
}

// ===== Helper Functions =====

fn matches_component_patterns(resource: &Value, component_info: &SuseEdgeComponentInfo) -> bool {
    let resource_name = get_resource_name(resource).unwrap_or_default().to_lowercase();
    let namespace = get_resource_namespace(resource).unwrap_or_default().to_lowercase();

    // Check namespace match
    let namespace_match = component_info.namespaces.iter()
        .any(|ns| namespace.contains(&ns.to_lowercase()));

    // Check pattern match in resource name or image
    let pattern_match = component_info.patterns.iter()
        .any(|pattern| {
            resource_name.contains(&pattern.to_lowercase()) ||
            contains_pattern_in_images(resource, pattern)
        });

    namespace_match || pattern_match
}

fn contains_pattern_in_images(resource: &Value, pattern: &str) -> bool {
    if let Some(containers) = extract_containers_from_resource(resource) {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                if image.to_lowercase().contains(&pattern.to_lowercase()) {
                    return true;
                }
            }
        }
    }
    false
}

fn extract_containers_from_resource(resource: &Value) -> Option<&Vec<Value>> {
    // Try different paths for containers based on resource type
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

fn extract_version_from_resource(resource: &Value, component_info: &SuseEdgeComponentInfo) -> Option<String> {
    // Try to extract version from container images
    if let Some(containers) = extract_containers_from_resource(resource) {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                for pattern in component_info.patterns {
                    if image.to_lowercase().contains(&pattern.to_lowercase()) {
                        if let Some(tag) = image.split(':').last() {
                            if tag != "latest" && !tag.is_empty() && tag != "stable" {
                                return Some(tag.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Try to extract from labels or annotations
    extract_version_from_metadata(resource)
}

fn extract_version_from_metadata(resource: &Value) -> Option<String> {
    if let Some(metadata) = resource.get("metadata") {
        // Check labels
        if let Some(labels) = metadata.get("labels").and_then(|l| l.as_object()) {
            for (key, value) in labels {
                if key.contains("version") || key.contains("release") {
                    if let Some(version_str) = value.as_str() {
                        return Some(version_str.to_string());
                    }
                }
            }
        }

        // Check annotations
        if let Some(annotations) = metadata.get("annotations").and_then(|a| a.as_object()) {
            for (key, value) in annotations {
                if key.contains("version") || key.contains("chart") {
                    if let Some(version_str) = value.as_str() {
                        return Some(version_str.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_helm_chart_version_from_locations(_locations: &[String]) -> Option<String> {
    // TODO: Implement Helm chart version extraction
    None
}

fn detect_suse_registry_usage(namespaced_resources: &HashMap<String, Vec<Value>>) -> Option<SuseEdgeComponent> {
    let mut suse_images = Vec::new();
    let suse_registries = ["registry.suse.com", "registry.opensuse.org"];

    for (resource_type, resources) in namespaced_resources {
        for resource in resources {
            if let Some(containers) = extract_containers_from_resource(resource) {
                for container in containers {
                    if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                        if suse_registries.iter().any(|reg| image.contains(reg)) {
                            if let Some(namespace) = get_resource_namespace(resource) {
                                let resource_name = get_resource_name(resource).unwrap_or_default();
                                let location = format!("{}/{}/{}.yaml", namespace, resource_type, resource_name);
                                suse_images.push(location);
                            }
                        }
                    }
                }
            }
        }
    }

    if !suse_images.is_empty() {
        Some(SuseEdgeComponent {
            name: "SUSE Container Registry Usage".to_string(),
            version: Some(format!("{} images", suse_images.len())),
            chart_version: None,
            found_in: suse_images,
            category: "Infrastructure".to_string(),
        })
    } else {
        None
    }
}

fn calculate_component_weight(component_name: &str) -> u32 {
    match component_name {
        // Core platform components (high weight)
        "SUSE Linux Micro" | "K3s" | "RKE2" => 25,
        "SUSE Rancher Prime" => 20,
        "SUSE Storage (Longhorn)" | "SUSE Security (NeuVector)" => 15,
        
        // Infrastructure components (medium weight)
        "Metal3" | "MetalLB" | "Elemental" => 10,
        "KubeVirt" | "Rancher Turtles (CAPI)" => 8,
        
        // Tools and extensions (lower weight)
        "Edge Image Builder" | "System Upgrade Controller" => 5,
        _ => 3, // Default weight
    }
}

fn determine_confidence_level(confidence_score: u32, component_count: usize) -> String {
    match (confidence_score, component_count) {
        (80.., 5..) => "Very High".to_string(),
        (60.., 3..) => "High".to_string(),
        (40.., 2..) => "Medium".to_string(),
        (20.., 1..) => "Low".to_string(),
        _ => "Minimal".to_string(),
    }
}

fn determine_deployment_type(components: &[SuseEdgeComponent]) -> String {
    let has_rancher = components.iter().any(|c| c.name.contains("Rancher"));
    let has_metal3 = components.iter().any(|c| c.name.contains("Metal3"));
    let has_elemental = components.iter().any(|c| c.name.contains("Elemental"));
    
    match (has_rancher, has_metal3, has_elemental) {
        (true, true, _) => "Management Cluster".to_string(),
        (true, false, true) => "Elemental Management Cluster".to_string(),
        (false, false, false) => "Standalone Cluster".to_string(),
        _ => "Downstream Cluster".to_string(),
    }
}

fn detect_edge_version(components: &[SuseEdgeComponent]) -> Option<String> {
    // Try to infer SUSE Edge version from component versions
    for component in components {
        if let Some(ref version) = component.version {
            if component.name.contains("Rancher") && version.starts_with("2.11") {
                return Some("3.3.x".to_string());
            }
            if component.name.contains("K3s") || component.name.contains("RKE2") {
                if version.starts_with("1.32") {
                    return Some("3.3.x".to_string());
                }
            }
        }
    }
    None
}

// ===== Specific Detection Functions =====

fn contains_rke2_indicators(resource: &Value) -> bool {
    // Check for RKE2-specific labels, annotations, or configurations
    if let Some(metadata) = resource.get("metadata") {
        if let Some(labels) = metadata.get("labels").and_then(|l| l.as_object()) {
            return labels.keys().any(|key| key.contains("rke2") || key.contains("rancher"));
        }
    }
    false
}

fn contains_k3s_indicators(resource: &Value) -> bool {
    // Check for K3s-specific labels, annotations, or configurations
    if let Some(metadata) = resource.get("metadata") {
        if let Some(labels) = metadata.get("labels").and_then(|l| l.as_object()) {
            return labels.keys().any(|key| key.contains("k3s"));
        }
    }
    false
}

fn extract_rke2_version(resource: &Value) -> Option<String> {
    if let Some(containers) = extract_containers_from_resource(resource) {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                if image.contains("rke2") || image.contains("rancher") {
                    if let Some(tag) = image.split(':').last() {
                        if tag != "latest" && tag != "stable" {
                            return Some(tag.to_string());
                        }
                    }
                }
            }
        }
    }
    Some("detected".to_string())
}

fn extract_k3s_version(resource: &Value) -> Option<String> {
    if let Some(containers) = extract_containers_from_resource(resource) {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                if image.contains("k3s") {
                    if let Some(tag) = image.split(':').last() {
                        if tag != "latest" && tag != "stable" {
                            return Some(tag.to_string());
                        }
                    }
                }
            }
        }
    }
    Some("detected".to_string())
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