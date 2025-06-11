# 🍅 Ketchup - Kubernetes Config Collector

> **Catch up** on your cluster configurations! 🏃‍♂️💨

A blazingly fast 🦀 Rust-powered tool that collects and archives Kubernetes cluster configurations for backup, analysis, and troubleshooting.

## ✨ Features

🔐 **Secure & Explicit** - Requires explicit kubeconfig path (no magic auto-discovery)  
📦 **Multi-Format Output** - Saves configurations in both JSON and YAML formats  
🗂️ **Organized Structure** - Creates timestamped directories for each collection  
📊 **Collection Summaries** - Generates detailed metadata about what was collected  
🗜️ **Compressed Archives** - Creates `.tar.gz` archives for easy storage and sharing  
🐳 **Container Ready** - Uses `/tmp` for output, perfect for containerized environments  
🚀 **Production Tested** - Works with real Kubernetes clusters (tested with K3s)  
⚡ **Fast & Reliable** - Built with Rust for maximum performance and safety  

## 🚀 Quick Start

### Prerequisites

- 🦀 **Rust** (install via [rustup](https://rustup.rs/))
- ☸️ **Kubernetes cluster** with accessible kubeconfig
- 📁 **Write access** to `/tmp` directory

### Installation

```bash
# Clone the repository
git clone https://github.com/Gagrio/suse-support-material.git
cd suse-support-material/ketchup

# Build the tool
cargo build --release

# Run it!
cargo run -- --kubeconfig ~/.kube/config --verbose
```

## 📖 Usage

### Basic Usage

```bash
# Collect from default namespace
cargo run -- --kubeconfig ~/.kube/config

# Collect from specific namespaces
cargo run -- --kubeconfig ~/.kube/config --namespaces "kube-system,default"

# Verbose output with detailed logging
cargo run -- --kubeconfig ~/.kube/config --verbose

# Custom output directory
cargo run -- --kubeconfig ~/.kube/config --output /my/backup/dir
```

### Command Line Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--kubeconfig` | `-k` | **Required** Path to kubeconfig file | - |
| `--namespaces` | `-n` | Comma-separated list of namespaces | `default` |
| `--output` | `-o` | Output directory for archives | `/tmp` |
| `--verbose` | `-v` | Enable verbose logging | `false` |
| `--help` | `-h` | Show help message | - |

## 📁 Output Structure

Ketchup creates organized, timestamped output:

```
/tmp/ketchup-2025-06-11-19-46-40/
├── 📄 collection-summary.json       # Collection metadata (JSON)
├── 📄 collection-summary.yaml       # Collection metadata (YAML) 
├── 📄 default-pods.json             # Pods from 'default' namespace (JSON)
├── 📄 default-pods.yaml             # Pods from 'default' namespace (YAML)
├── 📄 kube-system-pods.json         # Pods from 'kube-system' namespace (JSON)
└── 📄 kube-system-pods.yaml         # Pods from 'kube-system' namespace (YAML)

# Plus a compressed archive:
/tmp/ketchup-2025-06-11-19-46-40.tar.gz  🗜️
```

### Summary File Example

```json
{
  "collection_info": {
    "timestamp": "2025-06-11T19:46:40.569981Z",
    "tool": "ketchup",
    "version": "0.1.0"
  },
  "cluster_info": {
    "namespaces_requested": ["kube-system", "default"],
    "namespaces_collected": 2,
    "total_pods_collected": 7
  },
  "files_created": {
    "json_files": ["kube-system-pods.json", "default-pods.json"],
    "yaml_files": ["kube-system-pods.yaml", "default-pods.yaml"]
  }
}
```

## 🐳 Containerization

Perfect for running in containers! Example Dockerfile:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ketchup /usr/local/bin/
CMD ["ketchup", "--help"]
```

Run in container:
```bash
docker run -v ~/.kube/config:/kubeconfig:ro \
           -v /tmp:/tmp \
           ketchup --kubeconfig /kubeconfig --output /tmp --verbose
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
├── main.rs          # 🚪 CLI interface and main application logic
├── k8s.rs           # ☸️ Kubernetes client and resource collection
└── output.rs        # 📁 File output and archive management
```

## 🛣️ Roadmap

### ✅ Completed
- [x] 🔐 Explicit kubeconfig requirement
- [x] 📦 Pod collection with JSON/YAML output
- [x] 🗂️ Organized file structure
- [x] 📊 Collection summaries
- [x] 🗜️ Compressed archives

### 🚧 Coming Soon
- [ ] ⚙️ **Configuration files** - YAML configs for customizable behavior
- [ ] 🎯 **More resource types** - Services, Deployments, ConfigMaps, Secrets
- [ ] 🏷️ **Label selectors** - Filter resources by labels
- [ ] 📅 **Scheduling** - Automated periodic collections
- [ ] 🔍 **Diff mode** - Compare configurations between collections

## 🤝 Contributing

We love contributions! 💖

1. 🍴 Fork the repository
2. 🌟 Create a feature branch
3. 🛠️ Make your changes
4. ✅ Add tests if needed
5. 📤 Submit a pull request

## 📋 Requirements

- 🦀 **Rust 1.70+** (2021 edition)
- ☸️ **Kubernetes cluster** (any version)
- 📁 **File system access** for output directory

## 🐛 Troubleshooting

### Common Issues

**🚫 "Failed to load kubeconfig"**
- Check that the kubeconfig file exists and is readable
- Verify the file format is valid YAML
- Ensure you have network access to the cluster

**📁 "Permission denied" on output**
- Make sure the output directory is writable
- Try using `/tmp` as output directory
- Check file permissions with `ls -la`

**☸️ "Failed to connect to cluster"**
- Verify cluster is accessible: `kubectl get nodes`
- Check if kubeconfig context is correct
- Ensure cluster certificates are valid

## 📄 License

This project is part of the SUSE Support Material collection.

## 🙏 Acknowledgments

- 🦀 Built with **Rust** for performance and safety
- ☸️ Uses the **kube-rs** crate for Kubernetes API access
- 🎨 Inspired by the need for better cluster configuration management
- ☕ Powered by lots of coffee and determination

---

**Made with ❤️ and 🦀 by the SUSE Support Team**

*Catch up on your cluster configs with Ketchup!* 🍅✨