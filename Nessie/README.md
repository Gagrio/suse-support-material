# 🦕 Nessie: Node Environment Support Script for Inspection and Export

Nessie is a powerful diagnostic tool designed to collect logs and configuration data from SUSE Kubernetes environments. It gathers comprehensive information from both the host system and Kubernetes clusters, making it invaluable for troubleshooting and support.

## 📋 Features

Nessie collects a wide range of diagnostic information:

* 🖥️ System logs from SUSE Linux Micro services
* 🛳️ Kubernetes pod logs from all or selected namespaces
* ⚙️ Kubernetes configuration data and Helm releases
* 📊 Node metrics and performance data
* 🏷️ Version information of key components
* 📝 Metal3 logs for bare metal provisioning

All collected data is organized in a structured directory layout and compressed into a single archive for easy sharing with support engineers.

## 🚀 Running Nessie

Nessie can be run directly as a Python script or as a container. Both methods are designed to work with minimal configuration.

### 🐍 Direct Execution

To run Nessie directly on a system:

```bash
# Basic execution with defaults
python nessie.py

# With custom configuration
NESSIE_VERBOSE=1 NESSIE_NAMESPACES=kube-system,default python nessie.py
```

### 🐋 Container Execution

Nessie is available as a container image that can be run using Podman or Docker:

```bash
# Basic execution with defaults
podman run --privileged ghcr.io/gagrio/nessie

# With mounted Kubernetes config and persistent storage
podman run --privileged \
  -v /etc/rancher/k3s/k3s.yaml:/etc/rancher/k3s/k3s.yaml:ro \
  -v /var/log/journal:/var/log/journal:ro \
  -v /run/systemd:/run/systemd:ro \
  -v /etc/machine-id:/etc/machine-id:ro \
  -v /tmp/nessie-logs:/tmp/cluster-logs \
  ghcr.io/gagrio/nessie
```

The `--privileged` flag is needed to access system journals and logs.

## ⚙️ Configuration Options

Nessie can be configured through environment variables, making it highly customizable while maintaining reasonable defaults.

| Environment Variable | Default | Description |
|----------------------|---------|-------------|
| `NESSIE_LOG_DIR` | `/tmp/cluster-logs` | Base directory for storing collected logs |
| `NESSIE_ZIP_DIR` | `${LOG_DIR}/archives` | Directory for compressed archives |
| `NESSIE_MAX_LOG_SIZE` | `1024` | Maximum log storage size in megabytes |
| `NESSIE_RETENTION_DAYS` | `30` | Number of days to keep archived logs |
| `NESSIE_MAX_POD_LOG_LINES` | `1000` | Maximum number of log lines to collect per container |
| `NESSIE_NAMESPACES` | All | Comma-separated list of namespaces to collect logs from |
| `NESSIE_VERBOSE` | `0` | Verbosity level (0=minimal, 1=info, 2=debug) |
| `NESSIE_SKIP_NODE_LOGS` | `false` | Skip collecting node system logs if set to true |
| `NESSIE_SKIP_POD_LOGS` | `false` | Skip collecting Kubernetes pod logs if set to true |
| `NESSIE_SKIP_K8S_CONFIGS` | `false` | Skip collecting Kubernetes configurations if set to true |
| `NESSIE_SKIP_METRICS` | `false` | Skip collecting node metrics if set to true |
| `NESSIE_SKIP_VERSIONS` | `false` | Skip collecting version information if set to true |
| `KUBECONFIG` | Auto-detected | Path to Kubernetes configuration file |

## 📂 Output Format

Nessie organizes collected data into a structured directory layout:

```
nessie_logs_YYYY-MM-DD_HH-MM-SS/
├── node/                # Host system logs
│   ├── system.log
│   ├── combustion.log
│   └── ...
├── pods/                # Kubernetes pod logs
│   ├── namespace1/
│   │   ├── pod1_container1.log
│   │   └── ...
│   └── ...
├── configs/             # Kubernetes configuration
│   ├── namespaces.txt
│   ├── helm_releases.yaml
│   └── ...
├── metrics/             # Performance metrics
│   └── node_metrics.yaml
├── versions/            # Component versions
│   └── component_versions.txt
└── summary.yaml         # Collection summary report
```

All of this is compressed into a single archive file: `nessie_logs_YYYY-MM-DD_HH-MM-SS.tar.gz`.

## 🔄 Kubernetes Configuration Support

Nessie automatically detects Kubernetes configuration files in various locations, including:

* Custom location specified in `KUBECONFIG` environment variable
* Standard location (`~/.kube/config`)
* RKE2 configuration (`/etc/rancher/rke2/rke2.yaml`)
* K3s configuration (`/etc/rancher/k3s/k3s.yaml`)

This makes Nessie compatible with all SUSE Kubernetes implementations without requiring manual configuration.

## 💡 Common Use Cases

### Basic Collection for Support

```bash
podman run --privileged \
  -v /var/log/journal:/var/log/journal:ro \
  -v /etc/rancher/k3s/k3s.yaml:/etc/rancher/k3s/k3s.yaml:ro \
  -v /tmp/nessie-output:/tmp/cluster-logs \
  ghcr.io/gagrio/nessie
```

### Focused Collection for App Troubleshooting

```bash
podman run --privileged \
  -v /var/log/journal:/var/log/journal:ro \
  -v /etc/rancher/k3s/k3s.yaml:/etc/rancher/k3s/k3s.yaml:ro \
  -v /tmp/nessie-output:/tmp/cluster-logs \
  -e NESSIE_NAMESPACES=app-namespace,app-database \
  -e NESSIE_SKIP_METRICS=true \
  -e NESSIE_MAX_POD_LOG_LINES=5000 \
  ghcr.io/gagrio/nessie
```

### High Verbosity for Debugging Issues

```bash
podman run --privileged \
  -v /var/log/journal:/var/log/journal:ro \
  -v /etc/rancher/k3s/k3s.yaml:/etc/rancher/k3s/k3s.yaml:ro \
  -v /tmp/nessie-output:/tmp/cluster-logs \
  -e NESSIE_VERBOSE=2 \
  ghcr.io/gagrio/nessie
```

## 🛠️ Requirements

Nessie requires:

* Python 3.6 or newer with the `kubernetes` package
* Access to the Kubernetes API (via kubeconfig)
* Access to system logs (when running in a container, requires `--privileged`)

## 🔒 Security Notes

* The container requires privileged access to read system logs
* When sharing logs with support, ensure no sensitive information is included
* For secure environments, review the collected data before sharing

## 🤝 Contributing

Contributions to Nessie are welcome! Please feel free to submit issues or pull requests to the project repository.