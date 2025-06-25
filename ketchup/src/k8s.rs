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
use kube::{api::DynamicObject, discovery::Discovery};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct CustomResourceInfo {
    pub group: String,
    pub version: String,
    pub plural: String,
    pub namespaced: bool,
}

pub struct KubeClient {
    client: Client,
}

impl KubeClient {
    /// Create a new Kubernetes client using the specified kubeconfig file
    pub async fn new_client(kubeconfig_path: &str) -> Result<Self> {
        debug!("Loading kubeconfig from: {}", kubeconfig_path);

        // Set the KUBECONFIG environment variable (safe in our single-threaded context)
        unsafe {
            std::env::set_var("KUBECONFIG", kubeconfig_path);
        }

        let config = Config::infer().await.context("Failed to load kubeconfig")?;

        let client = Client::try_from(config).context("Failed to create Kubernetes client")?;

        debug!("Successfully connected to Kubernetes cluster");
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

        debug!("Found {} namespaces: {:?}", names.len(), names);
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
            debug!("Collecting {} from namespace: {}", resource_name, namespace);
            let api: Api<T> = Api::namespaced(self.client.clone(), namespace);

            match api.list(&Default::default()).await {
                Ok(resource_list) => {
                    let resource_count = resource_list.items.len();
                    for resource in resource_list.items {
                        if let Ok(json) = serde_json::to_value(&resource) {
                            all_resources.push(json);
                        }
                    }
                    debug!(
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
        debug!("Collecting cluster-scoped {}...", resource_name);
        let api: Api<T> = Api::all(self.client.clone());

        match api.list(&Default::default()).await {
            Ok(resource_list) => {
                let resource_count = resource_list.items.len();
                let resources: Vec<Value> = resource_list
                    .items
                    .into_iter()
                    .filter_map(|item| serde_json::to_value(&item).ok())
                    .collect();
                debug!("Found {} cluster-scoped {}", resource_count, resource_name);
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

    /// Parse collected CRDs to extract custom resource information
    pub fn parse_crd_info(&self, crd: &Value) -> Result<Option<CustomResourceInfo>> {
        let metadata = crd.get("metadata").context("Missing metadata")?;
        let spec = crd.get("spec").context("Missing spec")?;

        let _name = metadata
            .get("name")
            .and_then(|n| n.as_str())
            .context("Missing CRD name")?;

        let group = spec.get("group").and_then(|g| g.as_str()).unwrap_or("");

        let names = spec.get("names").context("Missing names section")?;
        let plural = names
            .get("plural")
            .and_then(|p| p.as_str())
            .context("Missing plural name")?;

        let scope = spec
            .get("scope")
            .and_then(|s| s.as_str())
            .unwrap_or("Namespaced");
        let namespaced = scope == "Namespaced";

        // Get the first served version
        let versions = spec
            .get("versions")
            .and_then(|v| v.as_array())
            .context("Missing versions")?;

        for version in versions {
            if let (Some(version_name), Some(served)) = (
                version.get("name").and_then(|n| n.as_str()),
                version.get("served").and_then(|s| s.as_bool()),
            ) {
                if served {
                    return Ok(Some(CustomResourceInfo {
                        group: group.to_string(),
                        version: version_name.to_string(),
                        plural: plural.to_string(),
                        namespaced,
                    }));
                }
            }
        }

        warn!("No served version found for CRD: {}", _name);
        Ok(None)
    }

    /// Collect all custom resource instances using hybrid approach
    pub async fn collect_all_custom_resources(
        &self,
        namespaces: &[String],
    ) -> Result<HashMap<String, Vec<Value>>> {
        debug!("Starting hybrid custom resource discovery and collection...");

        // Get all CRDs we've already collected
        let crds = self.collect_customresourcedefinitions().await?;
        debug!("Found {} CRDs to process", crds.len());

        let mut all_custom_resources = HashMap::new();

        for crd in crds {
            if let Ok(Some(cr_info)) = self.parse_crd_info(&crd) {
                debug!(
                    "Processing custom resource: {}.{}",
                    cr_info.plural, cr_info.group
                );

                // Try discovery-based collection first, then CRD-based fallback
                match self
                    .collect_custom_resource_instances_hybrid(&cr_info, namespaces)
                    .await
                {
                    Ok(instances) => {
                        if !instances.is_empty() {
                            let resource_key = if cr_info.group.is_empty() {
                                cr_info.plural.clone()
                            } else {
                                format!("{}.{}", cr_info.plural, cr_info.group)
                            };

                            debug!(
                                "Collected {} instances of {}",
                                instances.len(),
                                resource_key
                            );
                            all_custom_resources.insert(resource_key, instances);
                        }
                    }
                    Err(e) => {
                        debug!(
                            "Failed to collect instances of {}.{}: {}",
                            cr_info.plural, cr_info.group, e
                        );
                    }
                }
            }
        }

        debug!(
            "Collected {} different custom resource types",
            all_custom_resources.len()
        );
        Ok(all_custom_resources)
    }

    /// Hybrid approach: Try discovery first, fallback to direct CRD-based collection
    async fn collect_custom_resource_instances_hybrid(
        &self,
        cr_info: &CustomResourceInfo,
        namespaces: &[String],
    ) -> Result<Vec<Value>> {
        // Phase 1: Try discovery-based collection (fast when it works)
        match self
            .collect_custom_resource_instances_discovery(cr_info, namespaces)
            .await
        {
            Ok(instances) => {
                debug!(
                    "Discovery-based collection succeeded for {}",
                    cr_info.plural
                );
                return Ok(instances);
            }
            Err(e) => {
                debug!(
                    "Discovery-based collection failed for {}: {}",
                    cr_info.plural, e
                );
                debug!("Falling back to CRD-based collection...");
            }
        }

        // Phase 2: Fallback to CRD-based collection (more reliable)
        self.collect_custom_resource_instances_crd_based(cr_info, namespaces)
            .await
    }

    /// Discovery-based collection (original method)
    async fn collect_custom_resource_instances_discovery(
        &self,
        cr_info: &CustomResourceInfo,
        namespaces: &[String],
    ) -> Result<Vec<Value>> {
        let mut all_instances = Vec::new();

        // Use discovery to get the API resource info
        let discovery = Discovery::new(self.client.clone()).run().await?;

        if cr_info.namespaced {
            // Collect from each namespace
            for namespace in namespaces {
                match self
                    .collect_namespaced_custom_resource_discovery(
                        &discovery,
                        &cr_info.plural,
                        namespace,
                    )
                    .await
                {
                    Ok(mut instances) => {
                        all_instances.append(&mut instances);
                    }
                    Err(e) => {
                        debug!(
                            "No {} found in namespace {}: {}",
                            cr_info.plural, namespace, e
                        );
                    }
                }
            }
        } else {
            // Collect cluster-scoped
            match self
                .collect_cluster_custom_resource_discovery(&discovery, &cr_info.plural)
                .await
            {
                Ok(mut instances) => {
                    all_instances.append(&mut instances);
                }
                Err(e) => {
                    debug!("No cluster-scoped {} found: {}", cr_info.plural, e);
                }
            }
        }

        Ok(all_instances)
    }

    /// CRD-based collection (fallback method) with graceful error handling
    async fn collect_custom_resource_instances_crd_based(
        &self,
        cr_info: &CustomResourceInfo,
        namespaces: &[String],
    ) -> Result<Vec<Value>> {
        let api_version = if cr_info.group.is_empty() {
            cr_info.version.clone()
        } else {
            format!("{}/{}", cr_info.group, cr_info.version)
        };

        debug!(
            "Using CRD-based collection for {} ({})",
            cr_info.plural, api_version
        );

        let mut all_instances = Vec::new();

        if cr_info.namespaced {
            // Collect from each namespace with individual error handling
            for namespace in namespaces {
                match self
                    .collect_namespaced_custom_resource_crd_based(
                        &api_version,
                        &cr_info.plural,
                        namespace,
                    )
                    .await
                {
                    Ok(mut instances) => {
                        all_instances.append(&mut instances);
                        if !instances.is_empty() {
                            debug!(
                                "✅ Collected {} {} from namespace {}",
                                instances.len(),
                                cr_info.plural,
                                namespace
                            );
                        }
                    }
                    Err(e) => {
                        debug!(
                            "⚠️ Could not collect {} from namespace {} (API unavailable): {}",
                            cr_info.plural, namespace, e
                        );
                        // Continue with other namespaces
                    }
                }
            }
        } else {
            // Collect cluster-scoped with error handling
            match self
                .collect_cluster_custom_resource_crd_based(&api_version, &cr_info.plural)
                .await
            {
                Ok(mut instances) => {
                    all_instances.append(&mut instances);
                    if !instances.is_empty() {
                        debug!(
                            "✅ Collected {} cluster-scoped {}",
                            instances.len(),
                            cr_info.plural
                        );
                    }
                }
                Err(e) => {
                    debug!(
                        "⚠️ Could not collect cluster-scoped {} (API unavailable): {}",
                        cr_info.plural, e
                    );
                    // Continue anyway
                }
            }
        }

        Ok(all_instances)
    }

    /// Collect namespaced custom resource instances using discovery
    async fn collect_namespaced_custom_resource_discovery(
        &self,
        discovery: &Discovery,
        plural: &str,
        namespace: &str,
    ) -> Result<Vec<Value>> {
        debug!(
            "Collecting namespaced {} from {} (discovery)",
            plural, namespace
        );

        // Find the API resource
        for group in discovery.groups() {
            for (api_resource, capabilities) in group.recommended_resources() {
                if api_resource.plural == plural
                    && capabilities.scope == kube::discovery::Scope::Namespaced
                {
                    // Create dynamic API
                    let api: kube::Api<DynamicObject> =
                        kube::Api::namespaced_with(self.client.clone(), namespace, &api_resource);

                    let objects = api.list(&Default::default()).await?;

                    return Ok(objects
                        .items
                        .into_iter()
                        .filter_map(|obj| serde_json::to_value(obj).ok())
                        .collect());
                }
            }
        }

        Err(anyhow::anyhow!(
            "API resource not found in discovery: {}",
            plural
        ))
    }

    /// Collect cluster-scoped custom resource instances using discovery
    async fn collect_cluster_custom_resource_discovery(
        &self,
        discovery: &Discovery,
        plural: &str,
    ) -> Result<Vec<Value>> {
        debug!("Collecting cluster-scoped {} (discovery)", plural);

        // Find the API resource
        for group in discovery.groups() {
            for (api_resource, capabilities) in group.recommended_resources() {
                if api_resource.plural == plural
                    && capabilities.scope == kube::discovery::Scope::Cluster
                {
                    // Create dynamic API
                    let api: kube::Api<DynamicObject> =
                        kube::Api::all_with(self.client.clone(), &api_resource);

                    let objects = api.list(&Default::default()).await?;

                    return Ok(objects
                        .items
                        .into_iter()
                        .filter_map(|obj| serde_json::to_value(obj).ok())
                        .collect());
                }
            }
        }

        Err(anyhow::anyhow!(
            "API resource not found in discovery: {}",
            plural
        ))
    }

    /// Collect namespaced custom resource instances using CRD info
    async fn collect_namespaced_custom_resource_crd_based(
        &self,
        api_version: &str,
        plural: &str,
        namespace: &str,
    ) -> Result<Vec<Value>> {
        debug!(
            "Collecting namespaced {} from {} (CRD-based: {})",
            plural, namespace, api_version
        );

        // Parse the API version
        let (group, version) = if api_version.contains('/') {
            let parts: Vec<&str> = api_version.split('/').collect();
            (parts[0], parts[1])
        } else {
            ("", api_version)
        };

        // Create API resource manually from CRD info
        let api_resource = kube::discovery::ApiResource {
            group: group.to_string(),
            version: version.to_string(),
            api_version: api_version.to_string(),
            kind: "".to_string(), // We don't need kind for DynamicObject
            plural: plural.to_string(),
        };

        // Create dynamic API
        let api: kube::Api<DynamicObject> =
            kube::Api::namespaced_with(self.client.clone(), namespace, &api_resource);

        let objects = api.list(&Default::default()).await?;

        Ok(objects
            .items
            .into_iter()
            .filter_map(|obj| serde_json::to_value(obj).ok())
            .collect())
    }

    /// Collect cluster-scoped custom resource instances using CRD info
    async fn collect_cluster_custom_resource_crd_based(
        &self,
        api_version: &str,
        plural: &str,
    ) -> Result<Vec<Value>> {
        debug!(
            "Collecting cluster-scoped {} (CRD-based: {})",
            plural, api_version
        );

        // Parse the API version
        let (group, version) = if api_version.contains('/') {
            let parts: Vec<&str> = api_version.split('/').collect();
            (parts[0], parts[1])
        } else {
            ("", api_version)
        };

        // Create API resource manually from CRD info
        let api_resource = kube::discovery::ApiResource {
            group: group.to_string(),
            version: version.to_string(),
            api_version: api_version.to_string(),
            kind: "".to_string(), // We don't need kind for DynamicObject
            plural: plural.to_string(),
        };

        // Create dynamic API
        let api: kube::Api<DynamicObject> = kube::Api::all_with(self.client.clone(), &api_resource);

        let objects = api.list(&Default::default()).await?;

        Ok(objects
            .items
            .into_iter()
            .filter_map(|obj| serde_json::to_value(obj).ok())
            .collect())
    }
}
