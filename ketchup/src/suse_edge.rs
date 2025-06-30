use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::output::{SuseEdgeAnalysis, SuseEdgeComponent};

/// Detect SUSE Edge components from collected Kubernetes resources
pub fn detect_suse_edge_components(
    namespaced_resources: &HashMap<String, Vec<Value>>,
    cluster_resources: &HashMap<String, Vec<Value>>,
) -> Option<SuseEdgeAnalysis> {
    let mut components = Vec::new();

    info!("🔍 Scanning for SUSE Edge components...");

    // Detect Kubernetes distribution (RKE2/K3s)
    if let Some(k8s_components) = detect_kubernetes_distribution(namespaced_resources) {
        components.extend(k8s_components);
    }

    // Detect Rancher Prime
    if let Some(rancher_components) = detect_rancher_prime(namespaced_resources) {
        components.extend(rancher_components);
    }

    // Detect SUSE Storage (Longhorn)
    if let Some(storage_components) = detect_suse_storage(namespaced_resources) {
        components.extend(storage_components);
    }

    // Detect SUSE Security (NeuVector)
    if let Some(security_components) = detect_suse_security(namespaced_resources) {
        components.extend(security_components);
    }

    // Detect MetalLB
    if let Some(metallb_components) = detect_metallb(namespaced_resources, cluster_resources) {
        components.extend(metallb_components);
    }

    // Detect other SUSE Edge components
    if let Some(other_components) = detect_other_suse_components(namespaced_resources, cluster_resources) {
        components.extend(other_components);
    }

    if components.is_empty() {
        debug!("No SUSE Edge components detected");
        return None;
    }

    let total_components = components.len();
    let confidence = determine_confidence_level(total_components);

    info!("🎯 Detected {} SUSE Edge components", total_components);

    Some(SuseEdgeAnalysis {
        components,
        total_components,
        confidence,
    })
}

/// Detect RKE2 or K3s from system pods
fn detect_kubernetes_distribution(namespaced_resources: &HashMap<String, Vec<Value>>) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();

    // Look for RKE2/K3s in kube-system pods
    if let Some(pods) = namespaced_resources.get("pods") {
        for pod in pods {
            if let Some(namespace) = get_resource_namespace(pod) {
                if namespace != "kube-system" {
                    continue;
                }

                let pod_name = get_resource_name(pod).unwrap_or("unknown".to_string());
                let location = format!("{}/pods/{}.yaml", namespace, pod_name);

                // Detect RKE2
                if pod_name.contains("rke2") {
                    if let Some(version) = extract_rke2_version(pod) {
                        components.push(SuseEdgeComponent {
                            name: "RKE2".to_string(),
                            version: Some(version),
                            chart_version: None,
                            found_in: vec![location],
                        });
                    }
                }

                // Detect K3s
                if pod_name.contains("k3s") || detect_k3s_indicators(pod) {
                    if let Some(version) = extract_k3s_version(pod) {
                        components.push(SuseEdgeComponent {
                            name: "K3s".to_string(),
                            version: Some(version),
                            chart_version: None,
                            found_in: vec![location],
                        });
                    }
                }
            }
        }
    }

    if components.is_empty() { None } else { Some(components) }
}

/// Detect Rancher Prime from cattle-system namespace
fn detect_rancher_prime(namespaced_resources: &HashMap<String, Vec<Value>>) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();
    let mut found_in = Vec::new();

    // Check deployments for Rancher
    if let Some(deployments) = namespaced_resources.get("deployments") {
        for deployment in deployments {
            if let Some(namespace) = get_resource_namespace(deployment) {
                if !namespace.contains("cattle") && !namespace.contains("rancher") {
                    continue;
                }

                let deployment_name = get_resource_name(deployment).unwrap_or("unknown".to_string());
                
                if deployment_name.contains("rancher") {
                    let location = format!("{}/deployments/{}.yaml", namespace, deployment_name);
                    found_in.push(location);

                    if let Some(version) = extract_rancher_version(deployment) {
                        components.push(SuseEdgeComponent {
                            name: "Rancher Prime".to_string(),
                            version: Some(version),
                            chart_version: None,
                            found_in: found_in.clone(),
                        });
                        break; // Only need one instance
                    }
                }
            }
        }
    }

    if components.is_empty() { None } else { Some(components) }
}

/// Detect SUSE Storage (Longhorn) components
fn detect_suse_storage(namespaced_resources: &HashMap<String, Vec<Value>>) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();
    let mut found_in = Vec::new();

    // Check for Longhorn in multiple resource types
    for (resource_type, resources) in namespaced_resources {
        for resource in resources {
            if let Some(namespace) = get_resource_namespace(resource) {
                if !namespace.contains("longhorn") {
                    continue;
                }

                let resource_name = get_resource_name(resource).unwrap_or("unknown".to_string());
                let location = format!("{}/{}/{}.yaml", namespace, resource_type, resource_name);
                found_in.push(location);

                if let Some(version) = extract_longhorn_version(resource) {
                    components.push(SuseEdgeComponent {
                        name: "SUSE Storage (Longhorn)".to_string(),
                        version: Some(version),
                        chart_version: extract_helm_chart_version(resource),
                        found_in: found_in.clone(),
                    });
                    return Some(components); // Found it, return early
                }
            }
        }
    }

    if !found_in.is_empty() {
        // Found Longhorn resources but no version - still report it
        components.push(SuseEdgeComponent {
            name: "SUSE Storage (Longhorn)".to_string(),
            version: None,
            chart_version: None,
            found_in,
        });
    }

    if components.is_empty() { None } else { Some(components) }
}

/// Detect SUSE Security (NeuVector) components
fn detect_suse_security(namespaced_resources: &HashMap<String, Vec<Value>>) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();
    let mut found_in = Vec::new();

    for (resource_type, resources) in namespaced_resources {
        for resource in resources {
            if let Some(namespace) = get_resource_namespace(resource) {
                let resource_name = get_resource_name(resource).unwrap_or("unknown".to_string());

                if namespace.contains("neuvector") || resource_name.contains("neuvector") {
                    let location = format!("{}/{}/{}.yaml", namespace, resource_type, resource_name);
                    found_in.push(location);

                    if let Some(version) = extract_neuvector_version(resource) {
                        components.push(SuseEdgeComponent {
                            name: "SUSE Security (NeuVector)".to_string(),
                            version: Some(version),
                            chart_version: extract_helm_chart_version(resource),
                            found_in: found_in.clone(),
                        });
                        return Some(components);
                    }
                }
            }
        }
    }

    if !found_in.is_empty() {
        components.push(SuseEdgeComponent {
            name: "SUSE Security (NeuVector)".to_string(),
            version: None,
            chart_version: None,
            found_in,
        });
    }

    if components.is_empty() { None } else { Some(components) }
}

/// Detect MetalLB from both namespaced resources and CRDs
fn detect_metallb(
    namespaced_resources: &HashMap<String, Vec<Value>>,
    cluster_resources: &HashMap<String, Vec<Value>>,
) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();
    let mut found_in = Vec::new();

    // Check namespaced resources
    for (resource_type, resources) in namespaced_resources {
        for resource in resources {
            if let Some(namespace) = get_resource_namespace(resource) {
                let resource_name = get_resource_name(resource).unwrap_or("unknown".to_string());

                if namespace.contains("metallb") || resource_name.contains("metallb") {
                    let location = format!("{}/{}/{}.yaml", namespace, resource_type, resource_name);
                    found_in.push(location);
                }
            }
        }
    }

    // Check CRDs for MetalLB
    if let Some(crds) = cluster_resources.get("customresourcedefinitions") {
        for crd in crds {
            let crd_name = get_resource_name(crd).unwrap_or("unknown".to_string());
            if crd_name.contains("metallb.io") {
                let location = format!("cluster-wide/customresourcedefinitions/{}.yaml", crd_name);
                found_in.push(location);
            }
        }
    }

    if !found_in.is_empty() {
        // Try to extract version from any of the resources
        let version = extract_metallb_version_from_resources(namespaced_resources);
        
        components.push(SuseEdgeComponent {
            name: "MetalLB".to_string(),
            version,
            chart_version: None,
            found_in,
        });
    }

    if components.is_empty() { None } else { Some(components) }
}

/// Detect other SUSE Edge components (Elemental, Edge Image Builder artifacts, etc.)
fn detect_other_suse_components(
    namespaced_resources: &HashMap<String, Vec<Value>>,
    cluster_resources: &HashMap<String, Vec<Value>>,
) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();

    // Detect Elemental
    if let Some(elemental) = detect_elemental(namespaced_resources, cluster_resources) {
        components.extend(elemental);
    }

    // Detect SUSE Edge CRDs
    if let Some(edge_crds) = detect_suse_edge_crds(cluster_resources) {
        components.extend(edge_crds);
    }

    if components.is_empty() { None } else { Some(components) }
}

/// Helper function to detect Elemental components
fn detect_elemental(
    namespaced_resources: &HashMap<String, Vec<Value>>,
    cluster_resources: &HashMap<String, Vec<Value>>,
) -> Option<Vec<SuseEdgeComponent>> {
    let mut found_in = Vec::new();

    // Check for Elemental CRDs
    if let Some(crds) = cluster_resources.get("customresourcedefinitions") {
        for crd in crds {
            let crd_name = get_resource_name(crd).unwrap_or("unknown".to_string());
            if crd_name.contains("elemental") {
                let location = format!("cluster-wide/customresourcedefinitions/{}.yaml", crd_name);
                found_in.push(location);
            }
        }
    }

    // Check for Elemental pods/deployments
    for (resource_type, resources) in namespaced_resources {
        for resource in resources {
            let resource_name = get_resource_name(resource).unwrap_or("unknown".to_string());
            if resource_name.contains("elemental") {
                if let Some(namespace) = get_resource_namespace(resource) {
                    let location = format!("{}/{}/{}.yaml", namespace, resource_type, resource_name);
                    found_in.push(location);
                }
            }
        }
    }

    if !found_in.is_empty() {
        Some(vec![SuseEdgeComponent {
            name: "Elemental".to_string(),
            version: None,
            chart_version: None,
            found_in,
        }])
    } else {
        None
    }
}

/// Detect SUSE Edge specific CRDs
fn detect_suse_edge_crds(cluster_resources: &HashMap<String, Vec<Value>>) -> Option<Vec<SuseEdgeComponent>> {
    let mut components = Vec::new();

    if let Some(crds) = cluster_resources.get("customresourcedefinitions") {
        let suse_edge_patterns = vec![
            "k3s.cattle.io",
            "helm.cattle.io", 
            "traefik.containo.us",
            "traefik.io",
        ];

        for pattern in suse_edge_patterns {
            let mut found_in = Vec::new();
            
            for crd in crds {
                let crd_name = get_resource_name(crd).unwrap_or("unknown".to_string());
                if crd_name.contains(pattern) {
                    let location = format!("cluster-wide/customresourcedefinitions/{}.yaml", crd_name);
                    found_in.push(location);
                }
            }

            if !found_in.is_empty() {
                let component_name = match pattern {
                    "k3s.cattle.io" => "K3s Add-ons",
                    "helm.cattle.io" => "Helm Controller",
                    "traefik.containo.us" | "traefik.io" => "Traefik Ingress",
                    _ => "SUSE Edge Component",
                };

                components.push(SuseEdgeComponent {
                    name: component_name.to_string(),
                    version: None,
                    chart_version: None,
                    found_in,
                });
            }
        }
    }

    if components.is_empty() { None } else { Some(components) }
}

// ===== Helper Functions =====

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

// ===== Version Extraction Functions =====

fn extract_rke2_version(pod: &Value) -> Option<String> {
    // Look in container images for RKE2 version
    if let Some(containers) = pod
        .get("spec")
        .and_then(|s| s.get("containers"))
        .and_then(|c| c.as_array())
    {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                if image.contains("rke2") {
                    // Extract version from image tag
                    if let Some(tag) = image.split(':').nth(1) {
                        return Some(tag.to_string());
                    }
                }
            }
        }
    }

    // Look in pod name or labels
    if let Some(pod_name) = get_resource_name(pod) {
        if pod_name.contains("rke2") {
            return Some("detected".to_string());
        }
    }

    None
}

fn extract_k3s_version(pod: &Value) -> Option<String> {
    // Similar logic for K3s
    if let Some(containers) = pod
        .get("spec")
        .and_then(|s| s.get("containers"))
        .and_then(|c| c.as_array())
    {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                if image.contains("k3s") {
                    if let Some(tag) = image.split(':').nth(1) {
                        return Some(tag.to_string());
                    }
                }
            }
        }
    }

    Some("detected".to_string())
}

fn detect_k3s_indicators(pod: &Value) -> bool {
    // Look for K3s-specific labels or annotations
    if let Some(labels) = pod
        .get("metadata")
        .and_then(|m| m.get("labels"))
        .and_then(|l| l.as_object())
    {
        for (key, _) in labels {
            if key.contains("k3s") {
                return true;
            }
        }
    }
    false
}

fn extract_rancher_version(deployment: &Value) -> Option<String> {
    if let Some(containers) = deployment
        .get("spec")
        .and_then(|s| s.get("template"))
        .and_then(|t| t.get("spec"))
        .and_then(|s| s.get("containers"))
        .and_then(|c| c.as_array())
    {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                if image.contains("rancher/rancher") {
                    if let Some(tag) = image.split(':').last() {
                        return Some(tag.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_longhorn_version(resource: &Value) -> Option<String> {
    // Look in container images or labels
    if let Some(containers) = resource
        .get("spec")
        .and_then(|s| s.get("template"))
        .and_then(|t| t.get("spec"))
        .and_then(|s| s.get("containers"))
        .and_then(|c| c.as_array())
    {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                if image.contains("longhorn") {
                    if let Some(tag) = image.split(':').last() {
                        return Some(tag.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_neuvector_version(resource: &Value) -> Option<String> {
    // Similar logic for NeuVector
    if let Some(containers) = resource
        .get("spec")
        .and_then(|s| s.get("template"))
        .and_then(|t| t.get("spec"))
        .and_then(|s| s.get("containers"))
        .and_then(|c| c.as_array())
    {
        for container in containers {
            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                if image.contains("neuvector") {
                    if let Some(tag) = image.split(':').last() {
                        return Some(tag.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_metallb_version_from_resources(namespaced_resources: &HashMap<String, Vec<Value>>) -> Option<String> {
    // Look through deployments for MetalLB version
    if let Some(deployments) = namespaced_resources.get("deployments") {
        for deployment in deployments {
            if let Some(namespace) = get_resource_namespace(deployment) {
                if namespace.contains("metallb") {
                    if let Some(containers) = deployment
                        .get("spec")
                        .and_then(|s| s.get("template"))
                        .and_then(|t| t.get("spec"))
                        .and_then(|s| s.get("containers"))
                        .and_then(|c| c.as_array())
                    {
                        for container in containers {
                            if let Some(image) = container.get("image").and_then(|i| i.as_str()) {
                                if image.contains("metallb") {
                                    if let Some(tag) = image.split(':').last() {
                                        return Some(tag.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_helm_chart_version(resource: &Value) -> Option<String> {
    // Look for Helm chart annotations
    if let Some(annotations) = resource
        .get("metadata")
        .and_then(|m| m.get("annotations"))
        .and_then(|a| a.as_object())
    {
        if let Some(chart_version) = annotations.get("meta.helm.sh/release-namespace") {
            return chart_version.as_str().map(|s| s.to_string());
        }
    }
    None
}

fn determine_confidence_level(component_count: usize) -> String {
    match component_count {
        0 => "None".to_string(),
        1..=2 => "Low".to_string(),
        3..=4 => "Medium".to_string(),
        _ => "High".to_string(),
    }
}