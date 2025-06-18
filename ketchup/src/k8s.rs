use anyhow::{Context, Result};
use k8s_openapi::api::core::v1::{Namespace, Pod, Service};
use kube::{Api, Client, Config};
use serde_json::Value;
use tracing::{debug, info, warn};

pub struct KubeClient {
    client: Client,
}

impl KubeClient {
    /// Create a new Kubernetes client using the specified kubeconfig file
    pub async fn new_client(kubeconfig_path: &str) -> Result<Self> {
        info!("Loading kubeconfig from: {}", kubeconfig_path);

        // Set the KUBECONFIG environment variable (safe in our single-threaded context)
        unsafe {
            std::env::set_var("KUBECONFIG", kubeconfig_path);
        }

        let config = Config::infer().await.context("Failed to load kubeconfig")?;

        let client = Client::try_from(config).context("Failed to create Kubernetes client")?;

        info!("Successfully connected to Kubernetes cluster");
        Ok(KubeClient { client })
    }

    /// List all available namespaces in the cluster
    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        debug!("Fetching list of namespaces...");

        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        let namespace_list = namespaces
            .list(&Default::default())
            .await
            .context("Failed to list namespaces")?;

        let names: Vec<String> = namespace_list
            .items
            .iter()
            .filter_map(|ns| ns.metadata.name.clone())
            .collect();

        info!("Found {} namespaces: {:?}", names.len(), names);
        Ok(names)
    }

    /// Verify that specified namespaces exist
    pub async fn verify_namespaces(&self, requested: &[String]) -> Result<Vec<String>> {
        let available = self.list_namespaces().await?;
        let mut verified = Vec::new();

        for ns in requested {
            if available.contains(ns) {
                verified.push(ns.clone());
            } else {
                warn!("Namespace '{}' does not exist, skipping", ns);
            }
        }

        if verified.is_empty() {
            anyhow::bail!("No valid namespaces found");
        }

        Ok(verified)
    }

    /// Collect pods from specified namespaces
    pub async fn collect_pods(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        let mut all_pods = Vec::new();

        for namespace in namespaces {
            info!("Collecting pods from namespace: {}", namespace);
            let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);

            match pods.list(&Default::default()).await {
                Ok(pod_list) => {
                    let pod_count = pod_list.items.len();
                    for pod in pod_list.items {
                        if let Ok(json) = serde_json::to_value(&pod) {
                            all_pods.push(json);
                        }
                    }
                    info!("Found {} pods in namespace {}", pod_count, namespace);
                }
                Err(e) => {
                    warn!("Failed to collect pods from namespace {}: {}", namespace, e);
                }
            }
        }

        Ok(all_pods)
    }

    /// Collect services from specified namespaces
    pub async fn collect_services(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        let mut all_services = Vec::new();

        for namespace in namespaces {
            info!("Collecting services from namespace: {}", namespace);
            let services: Api<Service> = Api::namespaced(self.client.clone(), namespace);

            match services.list(&Default::default()).await {
                Ok(service_list) => {
                    let service_count = service_list.items.len();
                    for service in service_list.items {
                        if let Ok(json) = serde_json::to_value(&service) {
                            all_services.push(json);
                        }
                    }
                    info!(
                        "Found {} services in namespace {}",
                        service_count, namespace
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to collect services from namespace {}: {}",
                        namespace, e
                    );
                }
            }
        }

        Ok(all_services)
    }
}
