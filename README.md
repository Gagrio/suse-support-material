#   🦕  Nessie: Node Environment Support Script for Inspection and Export  🦕                     #
#   ☸️🐍  Enhanced script for using native Kubernetes python client  ☸️🐍                         #

## Overview

Nessie is a comprehensive log collection and analysis Python script designed for SUSE EDGE environments. It provides an automated solution for gathering critical system and cluster information, making troubleshooting and monitoring easier.

## 🚀 Features

- ✅ Collects logs from K3s/RKE2 configurations
- ✅ Gathers system service logs from SLE Micro
- ✅ Captures logs for all SUSE EDGE Pods across namespaces
- ✅ Collects version information for cluster components
- ✅ Generates detailed summary reports
- ✅ Creates compressed log archives
- ✅ Implements log rotation and retention policies

## 📋 Prerequisites

### System Requirements
- Python 3.8+
- Kubernetes cluster (K3s/RKE2)
- `kubectl` and `helm` CLI tools installed
- Sufficient disk space in `/var/log/cluster-logs`

### Required Python Packages
- kubernetes
- pyyaml

## 🔧 Installation

1. Clone the repository:
```bash
git clone https://github.com/Gagrio/suse-support-material.git
cd suse-support-material
```

2. Install required dependencies:
```bash
pip install -r requirements.txt
```

## 🐳 Docker Deployment

### Building the Container
```bash
docker build -t nessie .
```

### Running the Container
```bash
docker run --rm \
  -v /path/to/kubeconfig:/root/.kube/config \
  -v /var/log/cluster-logs:/var/log/cluster-logs \
  nessie
```

## ⚙️ Configuration

The script offers several configurable parameters:

- `LOG_DIR`: Base directory for storing collected logs (default: `/var/log/cluster-logs`)
- `MAX_LOG_SIZE`: Maximum log storage size (default: 1GB)
- `RETENTION_DAYS`: Number of days to retain log archives (default: 30)
- `MAX_POD_LOG_LINES`: Maximum log lines per container (default: 1000)
- `NAMESPACES_FILTER`: Optional list to limit log collection scope

Modify these in the script directly or pass as environment variables.

## 🔍 Usage

### Standalone Script
```bash
python nessie.py
```

## 📊 Output

Nessie generates:
- Detailed YAML data file with collected information
- Summary report with collection statistics
- Compressed log archive

Logs are stored in `/var/log/cluster-logs/archives`

## 🛡️ Security Considerations

- Requires appropriate Kubernetes RBAC permissions
- Sensitive information may be collected, so secure the output files
- Use with caution in production environments

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Create a new Pull Request

## 📜 License

GNU General Public License v3

## 🐞 Troubleshooting

- Ensure sufficient disk space
- Check Kubernetes configuration and permissions
- Verify Python and required packages are installed
- Review log files in `/var/log/cluster-logs` for detailed information

## 📞 Support

For issues or questions, please raise an issue.