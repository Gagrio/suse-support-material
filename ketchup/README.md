# 🍅 Ketchup - Kubernetes Config Collector

> **Catch up** on your cluster configuration! 🏃‍♂️💨

A blazingly fast 🦀 Rust-powered tool that collects and archives Kubernetes cluster configuration for backup, analysis, and troubleshooting.

## ✨ Features

🔐 **Secure & Explicit** - Requires explicit kubeconfig path (no magic auto-discovery)  
📦 **Multi-Format Output** - Saves resources in JSON, YAML, or both formats  
🗂️ **Organized Structure** - Creates timestamped directories with logical resource grouping  
📊 **Collection Summaries** - Generates detailed metadata about what was collected  
🗜️ **Flexible Compression** - Creates `.tar.gz` archives, uncompressed, or both  
🐳 **Container Ready** - Uses `/tmp` for output, perfect for containerized environments  
🚀 **Production Tested** - Works with real Kubernetes clusters (tested with K3s and RKE2)  
⚡ **Fast & Reliable** - Built with Rust for maximum performance and safety  
✨ **kubectl Apply Ready** - Sanitizes resources by default for immediate redeployment  
🍅 **SUSE Edge Detection** - Automatically detects and analyzes SUSE Edge components  
🎯 **Custom Resources** - Comprehensive CRD and custom resource instance collection  

## 🚀 Quick Start

### Prerequisites

- 🐳 **Docker** or **Podman** for running containers
- ☸️ **Kubernetes cluster** with accessible kubeconfig
- 📁 **Write access** to output directory (default: `/tmp`)

### Running with Podman

```bash
# Pull the latest container image
podman pull ghcr.io/gagrio/ketchup:latest

# Basic collection (all namespaces)
# Note: --network host allows container to reach your Kubernetes cluster
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig --verbose

# Collect from specific namespaces
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig \
           --namespaces "kube-system,default" --verbose

# Custom output directory
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /my/backup:/output \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig \
           --output /output --verbose
```

> **💡 Network Note**: The `--network host` flag is required for the container to access your Kubernetes cluster. This gives the container the same network access as your host machine, allowing it to connect to local clusters (K3s, kind, etc.) and remote clusters exactly like running Ketchup natively.

## 📖 Usage

### Basic Usage

> **⚠️ Important**: All commands require `--network host` to allow the container to connect to your Kubernetes cluster.

```bash
# Collect from all namespaces (default behavior)
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig

# Collect from specific namespaces
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig \
           --namespaces "kube-system,default"

# Verbose output with detailed logging
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig --verbose

# Include custom resources and CRDs
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig -C

# Disable SUSE Edge analysis
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig -D

# Raw mode (unsanitized resources)
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig --raw
```

### Command Line Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--kubeconfig` | `-k` | **Required** Path to kubeconfig file | - |
| `--namespaces` | `-n` | Comma-separated list of namespaces | All namespaces |
| `--output` | `-o` | Output directory for archives | `/tmp` |
| `--format` | `-f` | Output format: `json`, `yaml`, or `both` | `yaml` |
| `--compression` | `-c` | Compression: `compressed`, `uncompressed`, or `both` | `compressed` |
| `--include-custom-resources` | `-C` | Include CRDs and custom resource instances | `false` |
| `--raw` | `-r` | Collect raw unsanitized resources | `false` |
| `--disable-suse-edge-analysis` | `-D` | Disable SUSE Edge component detection | `false` |
| `--verbose` | `-v` | Enable verbose logging | `false` |
| `--debug` | `-d` | Enable debug logging | `false` |
| `--help` | `-h` | Show help message | - |

## 📁 Output Structure

Ketchup creates organized, timestamped output with a new logical structure:

```
/tmp/ketchup-2025-06-30-14-30-45/
├── 📋 collection-summary.yaml           # Main collection metadata and overview
├── 🍅 suse-edge-analysis.yaml          # SUSE Edge component analysis (if enabled)
├── 📂 cluster-wide-resources/           # Cluster-scoped resources
│   ├── 📂 nodes/                        # Individual node YAML files
│   ├── 📂 clusterroles/                 # Cluster roles
│   ├── 📂 clusterrolebindings/          # Cluster role bindings
│   ├── 📂 persistentvolumes/            # Persistent volumes
│   ├── 📂 storageclasses/               # Storage classes
│   └── 📂 custom-resources/             # CRDs and cluster-scoped custom resources
│       └── 📂 customresourcedefinitions/
└── 📂 namespaced-resources/             # Namespace-scoped resources
    ├── 📂 kube-system/                  # Resources in kube-system namespace
    │   ├── 📂 pods/                     # Individual pod YAML files
    │   ├── 📂 services/                 # Services
    │   ├── 📂 deployments/              # Deployments
    │   ├── 📂 configmaps/               # ConfigMaps
    │   ├── 📂 secrets/                  # Secrets
    │   └── 📂 custom-resources/         # Namespaced custom resources
    └── 📂 default/                      # Resources in default namespace
        ├── 📂 pods/
        ├── 📂 services/
        └── ...

# Plus compressed archive:
/tmp/ketchup-2025-06-30-14-30-45.tar.gz  🗜️
```

### Collection Summary Example

```yaml
# 🍅 KETCHUP CLUSTER COLLECTION SUMMARY
# Generated: 2025-06-30T14:30:45Z
# SUSE Edge Analysis: High confidence
# Kubernetes Distribution: K3s
# Mode: SANITIZED (kubectl apply ready)
# =======================================

📋 collection_info:
  timestamp: "2025-06-30T14:30:45Z"
  tool: ketchup
  version: "0.1.0"

📊 cluster_overview:
  total_resources: 156
  namespaces: 4
  cluster_resources: 28
  namespaced_resources: 128

✨ sanitization:
  mode: sanitized
  kubectl_ready: true
  total_processed: 156
  successfully_sanitized: 156
  note: "All resources successfully sanitized for kubectl apply."

🎯 resource_highlights:
  workloads:
    pods: 12
    deployments: 8
    daemon_sets: 4
  security:
    service_accounts: 15
    total_rbac_resources: 45
  configuration:
    config_maps: 23
    secrets: 18

📁 output_structure:
  kubectl_usage:
    apply_cluster_resources: "kubectl apply -f cluster-wide-resources/ --recursive"
    apply_namespaced_resources: "kubectl apply -f namespaced-resources/ --recursive"
    apply_specific_namespace: "kubectl apply -f namespaced-resources/{namespace}/ --recursive"
```

## 🔧 Resource Collection

### Core Resource Types Collected

**🏢 Namespaced Resources:**
- 🚀 **Workloads**: Pods, Deployments, ReplicaSets, DaemonSets, StatefulSets, Jobs, CronJobs
- 🌐 **Networking**: Services, Endpoints, EndpointSlices, Ingresses, NetworkPolicies  
- ⚙️ **Configuration**: ConfigMaps, Secrets
- 💾 **Storage**: PersistentVolumeClaims
- 👤 **RBAC**: ServiceAccounts, Roles, RoleBindings
- 📏 **Resource Management**: ResourceQuotas, LimitRanges, HorizontalPodAutoscalers, PodDisruptionBudgets

**☸️ Cluster-Scoped Resources:**
- 🖥️ **Infrastructure**: Nodes, PersistentVolumes, StorageClasses
- 🔐 **Security**: ClusterRoles, ClusterRoleBindings
- 🎯 **Custom Resources**: CustomResourceDefinitions (with `-C` flag)

### 🍅 SUSE Edge Detection

Ketchup automatically analyzes your cluster for SUSE Edge components:

- **🎯 Kubernetes Distribution**: Detects K3s, RKE2, or standard Kubernetes
- **🔍 Component Detection**: Identifies Rancher, Longhorn, NeuVector, KubeVirt, and more
- **📊 Confidence Scoring**: Provides confidence levels from "Minimal" to "Very High"
- **🏗️ Deployment Classification**: Categorizes as Management, Downstream, or Standalone cluster
- **📁 Detailed Analysis**: Creates separate `suse-edge-analysis.yaml` report

## 💡 Resource Sanitization

By default, Ketchup sanitizes resources for `kubectl apply` readiness:

### ✨ Automatic Cleanup
- **Removes**: `status`, `uid`, `resourceVersion`, `creationTimestamp`, `generation`
- **Cleans**: Problematic annotations and finalizers
- **Handles**: Service cluster IPs, PV claim references, auto-assigned node ports

### 🔧 Raw Mode
Use `--raw` flag to collect resources as-is from the cluster:
```bash
podman run --network host \
           -v ~/.kube/config:/kubeconfig \
           -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest \
           --kubeconfig /kubeconfig --raw
```

## 🔧 Development

### Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Check for issues
cargo clippy
```

### Project Structure

```
src/
├── main.rs          # 🚪 CLI interface and orchestration
├── k8s.rs           # ☸️ Kubernetes client and resource collection
├── output.rs        # 📁 File output, sanitization, and archive management
└── suse_edge.rs     # 🍅 SUSE Edge component detection and analysis
```

### 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test module
cargo test k8s::tests
```

## 💡 Resource Sanitization

By default, Ketchup sanitizes resources for `kubectl apply` readiness:

### ✨ Automatic Cleanup
- **Removes**: `status`, `uid`, `resourceVersion`, `creationTimestamp`, `generation`
- **Cleans**: Problematic annotations and finalizers
- **Handles**: Service cluster IPs, PV claim references, auto-assigned node ports

### 🔧 Raw Mode
Use `--raw` flag to collect resources as-is from the cluster:
```bash
docker run -v ~/.kube/config:/kubeconfig:ro -v /tmp:/tmp \
           ghcr.io/gagrio/ketchup:latest --kubeconfig /kubeconfig --raw
```

## 📋 Requirements

- 🦀 **Rust 1.70+** (2021 edition)
- ☸️ **Kubernetes cluster** (any version, tested with 1.28+)
- 📁 **File system access** for output directory
- 🌐 **Network access** to Kubernetes API server

## 🐛 Troubleshooting

### Common Issues

**🚫 "Failed to load kubeconfig"**
- ✅ Check that the kubeconfig file exists and is readable
- ✅ Verify the file format is valid YAML
- ✅ Ensure you have network access to the cluster
- ✅ Test with `kubectl get nodes` first

**📁 "Permission denied" on output****
- ✅ Make sure the output directory is writable
- ✅ Try using `/tmp` as output directory: `-o /tmp`
- ✅ Check file permissions with `ls -la`

**☸️ "Failed to connect to cluster"**
- ✅ Verify cluster is accessible: `kubectl cluster-info`
- ✅ Check if kubeconfig context is correct: `kubectl config current-context`
- ✅ Ensure cluster certificates are valid

**🎯 "Custom resource API errors"**
- ✅ These are often safe to ignore - the tool continues successfully
- ✅ Some CRDs may not have instances or may be in different API versions
- ✅ Check the collection summary for actual resource counts

**🍅 "SUSE Edge analysis disabled"**
- ✅ SUSE Edge analysis runs by default
- ✅ Use `-D` or `--disable-suse-edge-analysis` to explicitly disable
- ✅ Check `suse-edge-analysis.yaml` for detailed component detection results

## 📊 Performance

- **⚡ Fast Collection**: Typically processes 100+ resources in under 30 seconds
- **💾 Memory Efficient**: Streams resources to disk, minimal memory footprint
- **🔄 Concurrent Processing**: Async Rust for optimal network utilization
- **📦 Efficient Compression**: gzip compression reduces archive size by ~70%

## 📄 License

See the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- 🦀 Built with **Rust** for performance and safety
- ☸️ Uses the **kube-rs** crate for Kubernetes API access
- 🎨 Inspired by the need for better cluster configuration management
- 🍅 Enhanced with SUSE Edge ecosystem awareness
- ☕ Powered by lots of coffee and determination

---

**Made with ❤️ and 🦀 by the SUSE Support Team**

*Catch up on your cluster configuration with Ketchup!* 🍅✨