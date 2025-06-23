use anyhow::{Context, Result};
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler;
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    ConfigMap, Endpoints, LimitRange, Namespace, Node, PersistentVolume, PersistentVolumeClaim,
    Pod, ResourceQuota, Secret, Service, ServiceAccount,
};
use k8s_openapi::api::discovery::v1::EndpointSlice;
use k8s_openapi::api::networking::v1::{Ingress, NetworkPolicy};
use k8s_openapi::api::policy::v1::PodDisruptionBudget;
use k8s_openapi::api::rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding};
use k8s_openapi::api::storage::v1::StorageClass;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
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

    /// Collect replicasets from specified namespaces
    pub async fn collect_replicasets(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<ReplicaSet>(namespaces, "replicasets")
            .await
    }

    /// Collect daemonsets from specified namespaces
    pub async fn collect_daemonsets(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<DaemonSet>(namespaces, "daemonsets")
            .await
    }

    /// Collect statefulsets from specified namespaces
    pub async fn collect_statefulsets(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<StatefulSet>(namespaces, "statefulsets")
            .await
    }

    /// Collect jobs from specified namespaces
    pub async fn collect_jobs(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<Job>(namespaces, "jobs").await
    }

    /// Collect cronjobs from specified namespaces
    pub async fn collect_cronjobs(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<CronJob>(namespaces, "cronjobs")
            .await
    }

    /// Collect serviceaccounts from specified namespaces
    pub async fn collect_serviceaccounts(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<ServiceAccount>(namespaces, "serviceaccounts")
            .await
    }

    /// Collect roles from specified namespaces
    pub async fn collect_roles(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<Role>(namespaces, "roles").await
    }

    /// Collect rolebindings from specified namespaces
    pub async fn collect_rolebindings(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<RoleBinding>(namespaces, "rolebindings")
            .await
    }

    /// Collect resourcequotas from specified namespaces
    pub async fn collect_resourcequotas(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<ResourceQuota>(namespaces, "resourcequotas")
            .await
    }

    /// Collect limitranges from specified namespaces
    pub async fn collect_limitranges(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<LimitRange>(namespaces, "limitranges")
            .await
    }

    /// Collect horizontalpodautoscalers from specified namespaces
    pub async fn collect_horizontalpodautoscalers(
        &self,
        namespaces: &[String],
    ) -> Result<Vec<Value>> {
        self.collect_resources::<HorizontalPodAutoscaler>(namespaces, "horizontalpodautoscalers")
            .await
    }

    /// Collect poddisruptionbudgets from specified namespaces
    pub async fn collect_poddisruptionbudgets(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<PodDisruptionBudget>(namespaces, "poddisruptionbudgets")
            .await
    }

    /// Collect endpoints from specified namespaces
    pub async fn collect_endpoints(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<Endpoints>(namespaces, "endpoints")
            .await
    }

    /// Collect endpointslices from specified namespaces
    pub async fn collect_endpointslices(&self, namespaces: &[String]) -> Result<Vec<Value>> {
        self.collect_resources::<EndpointSlice>(namespaces, "endpointslices")
            .await
    }

    /// Generic method to collect cluster-scoped Kubernetes resources
    pub async fn collect_cluster_resources<T>(&self, resource_name: &str) -> Result<Vec<Value>>
    where
        T: k8s_openapi::Resource<Scope = k8s_openapi::ClusterResourceScope>
            + k8s_openapi::Metadata<Ty = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta>,
        T: serde::Serialize + serde::de::DeserializeOwned,
        T: Clone + std::fmt::Debug,
    {
        info!("Collecting cluster-scoped {}...", resource_name);
        let api: Api<T> = Api::all(self.client.clone());

        match api.list(&Default::default()).await {
            Ok(resource_list) => {
                let resource_count = resource_list.items.len();
                let resources: Vec<Value> = resource_list
                    .items
                    .into_iter()
                    .filter_map(|item| serde_json::to_value(&item).ok())
                    .collect();
                info!("Found {} cluster-scoped {}", resource_count, resource_name);
                Ok(resources)
            }
            Err(e) => {
                warn!("Failed to collect cluster-scoped {}: {}", resource_name, e);
                Ok(Vec::new())
            }
        }
    }

    /// Collect cluster roles (cluster-scoped)
    pub async fn collect_clusterroles(&self) -> Result<Vec<Value>> {
        self.collect_cluster_resources::<ClusterRole>("clusterroles")
            .await
    }

    /// Collect cluster role bindings (cluster-scoped)
    pub async fn collect_clusterrolebindings(&self) -> Result<Vec<Value>> {
        self.collect_cluster_resources::<ClusterRoleBinding>("clusterrolebindings")
            .await
    }

    /// Collect nodes (cluster-scoped)
    pub async fn collect_nodes(&self) -> Result<Vec<Value>> {
        self.collect_cluster_resources::<Node>("nodes").await
    }

    /// Collect persistent volumes (cluster-scoped)
    pub async fn collect_persistentvolumes(&self) -> Result<Vec<Value>> {
        self.collect_cluster_resources::<PersistentVolume>("persistentvolumes")
            .await
    }

    /// Collect storage classes (cluster-scoped)
    pub async fn collect_storageclasses(&self) -> Result<Vec<Value>> {
        self.collect_cluster_resources::<StorageClass>("storageclasses")
            .await
    }

    /// Collect custom resource definitions (cluster-scoped)
    pub async fn collect_customresourcedefinitions(&self) -> Result<Vec<Value>> {
        self.collect_cluster_resources::<CustomResourceDefinition>("customresourcedefinitions")
            .await
    }
}
