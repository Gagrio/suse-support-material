use anyhow::{Context, Result};
use kube::{Api, Client, Config};
use k8s_openapi::api::core::v1::Namespace;
use tracing::{debug, info, warn};

pub struct KubeClient {
    client: Option<Client>,
    demo_mode: bool,
}

impl KubeClient {
    /// Create a new Kubernetes client, falling back to demo mode if no cluster available
    pub async fn new() -> Result<Self> {
        info!("Connecting to Kubernetes cluster...");
        
        // Try to connect to real cluster first
        match Self::try_real_connection().await {
            Ok(client) => {
                info!("Successfully connected to Kubernetes cluster");
                Ok(KubeClient { 
                    client: Some(client), 
                    demo_mode: false 
                })
            }
            Err(e) => {
                warn!("Could not connect to Kubernetes cluster: {}", e);
                warn!("Falling back to demo mode for development");
                Ok(KubeClient { 
                    client: None, 
                    demo_mode: true 
                })
            }
        }
    }
    
    async fn try_real_connection() -> Result<Client> {
        let config = Config::infer().await
            .context("Failed to load kubeconfig")?;
        
        let client = Client::try_from(config)
            .context("Failed to create Kubernetes client")?;
        
        // Test the connection by listing namespaces with a timeout
        let namespaces: Api<Namespace> = Api::all(client.clone());
        
        // Use a timeout to avoid hanging
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            namespaces.list(&Default::default())
        ).await
            .context("Connection timeout")?
            .context("Failed to connect to cluster")?;
            
        Ok(client)
    }
    
    /// List all available namespaces in the cluster
    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        if self.demo_mode {
            return self.demo_list_namespaces().await;
        }
        
        debug!("Fetching list of namespaces...");
        
        let client = self.client.as_ref().unwrap();
        let namespaces: Api<Namespace> = Api::all(client.clone());
        let namespace_list = namespaces.list(&Default::default()).await
            .context("Failed to list namespaces")?;
        
        let names: Vec<String> = namespace_list
            .items
            .iter()
            .filter_map(|ns| ns.metadata.name.clone())
            .collect();
        
        info!("Found {} namespaces: {:?}", names.len(), names);
        Ok(names)
    }
    
    /// Demo mode: return fake namespaces
    async fn demo_list_namespaces(&self) -> Result<Vec<String>> {
        info!("Demo mode: simulating namespace list");
        let demo_namespaces = vec![
            "default".to_string(),
            "kube-system".to_string(),
            "kube-public".to_string(),
            "my-app".to_string(),
            "monitoring".to_string(),
        ];
        info!("Found {} namespaces: {:?}", demo_namespaces.len(), demo_namespaces);
        Ok(demo_namespaces)
    }
    
    /// Verify that specified namespaces exist
    pub async fn verify_namespaces(&self, requested: &[String]) -> Result<Vec<String>> {
        let available = self.list_namespaces().await?;
        let mut verified = Vec::new();
        
        for ns in requested {
            if available.contains(ns) {
                verified.push(ns.clone());
            } else {
                tracing::warn!("Namespace '{}' does not exist, skipping", ns);
            }
        }
        
        if verified.is_empty() {
            anyhow::bail!("No valid namespaces found");
        }
        
        Ok(verified)
    }
}
