use anyhow::{Context, Result};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{
    ConfigMap, Namespace, PersistentVolumeClaim, Pod, Secret, Service,
};
use k8s_openapi::api::networking::v1::{Ingress, NetworkPolicy};
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

    /// Generic method to collect any namespaced Kubernetes resources
    pub async fn collect_resources<T>(
        &self,
        namespaces: &[String],
        resource_name: &str,
    ) -> Result<Vec<Value>>
    where
        T: k8s_openapi::Resource<Scope = k8s_openapi::NamespaceResourceScope>
            + k8s_openapi::Metadata<Ty = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta>,
        T: serde::Serialize + serde::de::DeserializeOwned,
        T: Clone + std::fmt::Debug,
    {
        let mut all_resources = Vec::new();

        for namespace in namespaces {
            info!("Collecting {} from namespace: {}", resource_name, namespace);
            let api: Api<T> = Api::namespaced(self.client.clone(), namespace);

            match api.list(&Default::default()).await {
                Ok(resource_list) => {
                    let resource_count = resource_list.items.len();
                    for resource in resource_list.items {
                        if let Ok(json) = serde_json::to_value(&resource) {
                            all_resources.push(json);
                        }
                    }
                    info!(
                        "Found {} {} in namespace {}",
                        resource_count, resource_name, namespace
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to collect {} from namespace {}: {}",
                        resource_name, namespace, e
                    );
                }
            }
        }

        Ok(all_resources)
    }

    /// Collect pods from specified namespaces
    pub async fn collect_pods(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<Pod>(namespaces, "pods").await
    }

    /// Collect services from specified namespaces
    pub async fn collect_services(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<Service>(namespaces, "services")
            .await
    }

    /// Collect deployments from specified namespaces
    pub async fn collect_deployments(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<Deployment>(namespaces, "deployments")
            .await
    }

    /// Collect configmaps from specified namespaces
    pub async fn collect_configmaps(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<ConfigMap>(namespaces, "configmaps")
            .await
    }

    /// Collect secrets from specified namespaces
    pub async fn collect_secrets(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<Secret>(namespaces, "secrets")
            .await
    }

    /// Collect ingresses from specified namespaces
    pub async fn collect_ingresses(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<Ingress>(namespaces, "ingresses")
            .await
    }

    /// Collect persistentvolumeclaims from specified namespaces
    pub async fn collect_persistentvolumeclaims(
        &self,
        namespaces: &[String],
    ) -> Result<Vec<Value>> {
        self.collect_resources::<PersistentVolumeClaim>(namespaces, "persistentvolumeclaims")
            .await
    }

    /// Collect networkpolicies from specified namespaces
    pub async fn collect_networkpolicies(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<NetworkPolicy>(namespaces, "networkpolicies")
            .await
    }
}
