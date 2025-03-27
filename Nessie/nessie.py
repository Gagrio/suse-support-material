#!/usr/bin/env python3
# Nessie: Node Environment Support Script for Inspection and Export
# Collects logs and configurations from SUSE Kubernetes environments

import os
import yaml
import time
import logging
import shutil
import tarfile
import subprocess
from datetime import datetime, timedelta
from kubernetes import client, config
from pathlib import Path

# Configuration from environment variables with defaults
LOG_DIR = os.environ.get('NESSIE_LOG_DIR', '/tmp')
ZIP_DIR = os.environ.get('NESSIE_ZIP_DIR', f"{LOG_DIR}/archives")
MAX_LOG_SIZE = int(os.environ.get('NESSIE_MAX_LOG_SIZE', '1024')) * 1024 * 1024
RETENTION_DAYS = int(os.environ.get('NESSIE_RETENTION_DAYS', '30'))
MAX_POD_LOG_LINES = int(os.environ.get('NESSIE_MAX_POD_LOG_LINES', '1000'))

# Namespace filtering
NAMESPACES_FILTER = os.environ.get('NESSIE_NAMESPACES', '').split(',') if os.environ.get('NESSIE_NAMESPACES') else None
if NAMESPACES_FILTER and len(NAMESPACES_FILTER) == 1 and NAMESPACES_FILTER[0] == '':
    NAMESPACES_FILTER = None

# Skip flags and verbosity
VERBOSE = int(os.environ.get('NESSIE_VERBOSE', '0'))
SKIP_NODE_LOGS = os.environ.get('NESSIE_SKIP_NODE_LOGS', '').lower() in ('true', 'yes', '1', 'on')
SKIP_POD_LOGS = os.environ.get('NESSIE_SKIP_POD_LOGS', '').lower() in ('true', 'yes', '1', 'on')
SKIP_K8S_CONFIGS = os.environ.get('NESSIE_SKIP_K8S_CONFIGS', '').lower() in ('true', 'yes', '1', 'on')
SKIP_METRICS = os.environ.get('NESSIE_SKIP_METRICS', '').lower() in ('true', 'yes', '1', 'on')
SKIP_VERSIONS = os.environ.get('NESSIE_SKIP_VERSIONS', '').lower() in ('true', 'yes', '1', 'on')

# Configure logging
log_level = max(logging.WARNING - (VERBOSE * 10), logging.DEBUG)
logging.basicConfig(level=log_level, format="%(asctime)s - %(levelname)s - %(message)s")
logger = logging.getLogger(__name__)

# Service logs to collect
NODE_SERVICES = {
    "system": "journalctl -n 1000 --no-pager",
    "combustion": "journalctl -u combustion --no-pager",
    "hauler": "journalctl -u hauler --no-pager",
    "nmc": "journalctl -u nm-configurator --no-pager"
}

# Commands to retrieve version information
VERSION_COMMANDS = {
    "helm": "helm version --short",
    "kubectl": "kubectl version",
    "upgrade-controller": "kubectl get deployment system-upgrade-controller -n cattle-system -o jsonpath='{.spec.template.spec.containers[0].image}'",
    "endpoint-copier-operator": "kubectl get deployment endpoint-copier-operator -n endpoint-copier-operator -o jsonpath='{.spec.template.spec.containers[0].image}'",
    "metallb": "kubectl get deployment metallb-controller -n metallb-system -o jsonpath='{.spec.template.spec.containers[*].image}'"
}

class ProgressTracker:
    """Tracks progress of long-running operations"""
    def __init__(self, total_items, operation_name):
        self.total = total_items
        self.current = 0
        self.operation_name = operation_name
        self.start_time = time.time()
        logger.info(f"Starting {operation_name} (0/{total_items})")
    
    def update(self, increment=1):
        """Updates progress counter and logs status"""
        self.current += increment
        percent = (self.current / self.total) * 100
        elapsed = time.time() - self.start_time
        logger.info(f"{self.operation_name} progress: {self.current}/{self.total} ({percent:.1f}%) - {elapsed:.1f}s elapsed")
    
    def complete(self):
        """Marks operation as complete and returns duration"""
        total_time = time.time() - self.start_time
        logger.info(f"Completed {self.operation_name} in {total_time:.1f}s")
        return total_time

def ensure_directories():
    """Creates required directories and verifies write access"""
    try:
        Path(LOG_DIR).mkdir(exist_ok=True, parents=True)
        Path(ZIP_DIR).mkdir(exist_ok=True, parents=True)
        # Test write permissions
        test_file = Path(LOG_DIR) / "write_test"
        test_file.touch()
        test_file.unlink()
        return True
    except (PermissionError, OSError) as e:
        logger.error(f"Directory access error: {e}")
        return False

def setup_kubernetes_client():
    """Initializes Kubernetes API clients with support for SUSE K8s variants"""
    # Possible Kubernetes config locations
    kubeconfig_locations = [
        os.environ.get('KUBECONFIG'),  # KUBECONFIG env var
        '~/.kube/config',              # Standard location
        '/etc/rancher/rke2/rke2.yaml', # RKE2
        '/etc/rancher/k3s/k3s.yaml',   # K3s
    ]
    
    try:
        # Try in-cluster config first
        config.load_incluster_config()
        logger.info("Using in-cluster Kubernetes configuration")
        # Can't set KUBECONFIG for in-cluster config as it uses service account token
    except config.ConfigException:
        # Fall back to local configs
        loaded = False
        
        for kubeconfig in kubeconfig_locations:
            if not kubeconfig:
                continue
            
            expanded_path = os.path.expanduser(kubeconfig)
            if os.path.isfile(expanded_path):
                try:
                    config.load_kube_config(config_file=expanded_path)
                    logger.info(f"Using Kubernetes configuration from {expanded_path}")
                    
                    # Set the KUBECONFIG environment variable for command-line tools
                    os.environ['KUBECONFIG'] = expanded_path
                    logger.info(f"Set KUBECONFIG environment variable to {expanded_path}")
                    
                    loaded = True
                    break
                except config.ConfigException as e:
                    logger.warning(f"Found but failed to load config at {expanded_path}: {e}")
        
        if not loaded:
            logger.error("Failed to find or load any Kubernetes configuration")
            return None, None
    
    return client.CoreV1Api(), client.CustomObjectsApi()

def run_command(command, shell=False):
    """Runs a command safely and returns its output"""
    try:
        args = command if isinstance(command, list) else command
        result = subprocess.run(
            args, 
            shell=shell, 
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            universal_newlines=True,
            timeout=60
        )
        if result.returncode == 0:
            return True, result.stdout
        else:
            return False, f"Command failed with code {result.returncode}: {result.stderr}"
    except subprocess.TimeoutExpired:
        return False, "Command timed out after 60 seconds"
    except Exception as e:
        return False, f"Error executing command: {e}"

def collect_node_logs():
    """Collects logs from system services on the host node"""
    logs = {}
    progress = ProgressTracker(len(NODE_SERVICES), "Node log collection")
    
    for name, cmd in NODE_SERVICES.items():
        success, output = run_command(cmd, shell=True)
        logs[name] = output if success else f"Failed to collect logs: {output}"
        progress.update()
    
    progress.complete()
    return logs

def collect_k8s_configs(v1_api):
    """Collects Kubernetes configuration and state information"""
    data = {}
    logger.info("Collecting Kubernetes configuration information")
    
    try:
        # Get namespaces
        namespaces = v1_api.list_namespace()
        data["namespaces"] = [ns.metadata.name for ns in namespaces.items]
        logger.info(f"Collected information for {len(data['namespaces'])} namespaces")
        
        # Get Helm releases
        success, helm_output = run_command(["helm", "list", "-A", "-o", "yaml"])
        if success:
            data["helm_releases"] = yaml.safe_load(helm_output)
            logger.info(f"Collected information for {len(data['helm_releases']) if isinstance(data['helm_releases'], list) else 0} Helm releases")
        else:
            logger.warning(f"Failed to fetch Helm releases: {helm_output}")
            data["helm_releases"] = []
            
        # Collect Metal3 logs
        success, metal3_logs = run_command("journalctl -u ironic -u metal3 -n 1000 --no-pager", shell=True)
        if success:
            data["metal3_logs"] = metal3_logs
            logger.info("Collected Metal3 logs")
        else:
            logger.warning(f"Failed to collect Metal3 logs: {metal3_logs}")
            data["metal3_logs"] = "No Metal3 logs available"
    except Exception as e:
        logger.error(f"Error collecting Kubernetes configs: {e}")
        data["error"] = str(e)
    
    return data

def collect_pod_logs(v1_api):
    """Collects logs from pods, optionally filtered by namespace"""
    pod_logs = {}
    
    try:
        # Get pods with optional namespace filtering
        if NAMESPACES_FILTER:
            pods = []
            for ns in NAMESPACES_FILTER:
                try:
                    ns_pods = v1_api.list_namespaced_pod(ns).items
                    pods.extend(ns_pods)
                    logger.info(f"Collected {len(ns_pods)} pods from namespace {ns}")
                except Exception as e:
                    logger.warning(f"Failed to get pods in namespace {ns}: {e}")
        else:
            pods = v1_api.list_pod_for_all_namespaces(watch=False).items
            logger.info(f"Collected {len(pods)} pods from all namespaces")
        
        progress = ProgressTracker(len(pods), "Pod log collection")
        
        # Collect logs from each container in each pod
        for pod in pods:
            pod_name = pod.metadata.name
            namespace = pod.metadata.namespace
            containers = [c.name for c in pod.spec.containers]
            
            pod_logs[f"{namespace}/{pod_name}"] = {}
            
            for container in containers:
                try:
                    log_data = v1_api.read_namespaced_pod_log(
                        name=pod_name,
                        namespace=namespace,
                        container=container,
                        tail_lines=MAX_POD_LOG_LINES
                    )
                    pod_logs[f"{namespace}/{pod_name}"][container] = log_data
                except Exception as e:
                    pod_logs[f"{namespace}/{pod_name}"][container] = f"Error: {str(e)}"
            
            progress.update()
        
        progress.complete()
        
    except Exception as e:
        logger.error(f"Error collecting pod logs: {e}")
        pod_logs["error"] = str(e)
    
    return pod_logs

def collect_node_metrics(custom_api):
    """Collects node metrics using the Kubernetes metrics API"""
    logger.info("Collecting node metrics")
    try:
        response = custom_api.list_cluster_custom_object("metrics.k8s.io", "v1beta1", "nodes")
        logger.info(f"Collected metrics for {len(response.get('items', []))} nodes")
        return response
    except Exception as e:
        logger.warning(f"Metrics server not available: {e}")
        return {"error": str(e)}

def collect_versions():
    """Collects version information for cluster components"""
    versions = {}
    progress = ProgressTracker(len(VERSION_COMMANDS), "Version collection")
    
    for component, cmd in VERSION_COMMANDS.items():
        success, output = run_command(cmd, shell=True)
        versions[component] = output.strip() if success else f"Not available: {output}"
        progress.update()
    
    progress.complete()
    return versions

def save_text_logs(data, base_dir):
    """Saves collected logs as individual text files in an organized directory structure"""
    created_files = []
    timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    collection_dir = Path(base_dir) / f"nessie_logs_{timestamp}"
    
    # Create directory structure
    collection_dir.mkdir(exist_ok=True)
    (collection_dir / "node").mkdir(exist_ok=True)
    (collection_dir / "pods").mkdir(exist_ok=True)
    (collection_dir / "configs").mkdir(exist_ok=True)
    (collection_dir / "metrics").mkdir(exist_ok=True)
    (collection_dir / "versions").mkdir(exist_ok=True)
    
    # Save node logs
    if "node_logs" in data and isinstance(data["node_logs"], dict):
        for service, log_content in data["node_logs"].items():
            if service == "error":
                continue
            log_file = collection_dir / "node" / f"{service}.log"
            with open(log_file, "w") as f:
                f.write(str(log_content))
            created_files.append(log_file)
    
    # Save pod logs
    if "pod_logs" in data and isinstance(data["pod_logs"], dict):
        for pod_key, containers in data["pod_logs"].items():
            if pod_key == "error":
                continue
                
            # Create namespace directories
            if "/" in pod_key:
                namespace, pod_name = pod_key.split("/", 1)
                ns_dir = collection_dir / "pods" / namespace
                ns_dir.mkdir(exist_ok=True)
                
                # Save each container's logs
                for container, log_content in containers.items():
                    log_file = ns_dir / f"{pod_name}_{container}.log"
                    with open(log_file, "w") as f:
                        f.write(str(log_content))
                    created_files.append(log_file)
    
    # Save K8s configuration information
    if "k8s_configs" in data and isinstance(data["k8s_configs"], dict):
        # Save namespaces list
        if "namespaces" in data["k8s_configs"]:
            namespaces_file = collection_dir / "configs" / "namespaces.txt"
            with open(namespaces_file, "w") as f:
                for ns in data["k8s_configs"]["namespaces"]:
                    f.write(f"{ns}\n")
            created_files.append(namespaces_file)
        
    # Save Helm releases
        if "helm_releases" in data["k8s_configs"]:
            helm_file = collection_dir / "configs" / "helm_releases.yaml"
            with open(helm_file, "w") as f:
                yaml.dump(data["k8s_configs"]["helm_releases"], f)
            created_files.append(helm_file)
            
        # Save Metal3 logs
        if "metal3_logs" in data["k8s_configs"]:
            metal3_file = collection_dir / "configs" / "metal3.log"
            with open(metal3_file, "w") as f:
                f.write(str(data["k8s_configs"]["metal3_logs"]))
            created_files.append(metal3_file)
    
    # Save metrics as YAML (more structured)
    if "node_metrics" in data:
        metrics_file = collection_dir / "metrics" / "node_metrics.yaml"
        with open(metrics_file, "w") as f:
            yaml.dump(data["node_metrics"], f)
        created_files.append(metrics_file)
    
    # Save versions as text file
    if "versions" in data and isinstance(data["versions"], dict):
        versions_file = collection_dir / "versions" / "component_versions.txt"
        with open(versions_file, "w") as f:
            for component, version in data["versions"].items():
                f.write(f"{component}: {version}\n")
        created_files.append(versions_file)
    
    return created_files, collection_dir

def create_summary_report(data, start_time, collection_dir):
    """Creates a summary report of the collected data"""
    logger.info("Creating summary report")
    
    # Build environment variables dictionary
    env_vars = {
        "NESSIE_LOG_DIR": LOG_DIR,
        "NESSIE_ZIP_DIR": ZIP_DIR,
        "NESSIE_MAX_LOG_SIZE": str(MAX_LOG_SIZE // (1024 * 1024)) + " MB",
        "NESSIE_RETENTION_DAYS": RETENTION_DAYS,
        "NESSIE_MAX_POD_LOG_LINES": MAX_POD_LOG_LINES,
        "NESSIE_NAMESPACES": ','.join(NAMESPACES_FILTER) if NAMESPACES_FILTER else "All",
        "NESSIE_VERBOSE": VERBOSE,
        "NESSIE_SKIP_NODE_LOGS": SKIP_NODE_LOGS,
        "NESSIE_SKIP_POD_LOGS": SKIP_POD_LOGS,
        "NESSIE_SKIP_K8S_CONFIGS": SKIP_K8S_CONFIGS,
        "NESSIE_SKIP_METRICS": SKIP_METRICS,
        "NESSIE_SKIP_VERSIONS": SKIP_VERSIONS
    }
    
    # Count files in each category
    pod_files = len(list(Path(collection_dir).glob("pods/**/*.log")))
    node_files = len(list(Path(collection_dir).glob("node/*.log")))
    config_files = len(list(Path(collection_dir).glob("configs/*")))
    
    summary = {
        "collection_info": {
            "timestamp": datetime.now().isoformat(),
            "duration_seconds": time.time() - start_time,
            "output_directory": str(collection_dir),
            "environment_variables": env_vars
        },
        "collection_status": {
            "node_logs": "skipped" if SKIP_NODE_LOGS else "collected" if "node_logs" in data else "failed",
            "k8s_configs": "skipped" if SKIP_K8S_CONFIGS else "collected" if "k8s_configs" in data else "failed",
            "pod_logs": "skipped" if SKIP_POD_LOGS else "collected" if "pod_logs" in data else "failed",
            "node_metrics": "skipped" if SKIP_METRICS else "collected" if "node_metrics" in data else "failed",
            "versions": "skipped" if SKIP_VERSIONS else "collected" if "versions" in data else "failed"
        },
        "stats": {
            "namespaces": len(data.get("k8s_configs", {}).get("namespaces", [])),
            "helm_releases": len(data.get("k8s_configs", {}).get("helm_releases", [])) if isinstance(data.get("k8s_configs", {}).get("helm_releases", []), list) else 0,
            "pod_log_files": pod_files,
            "node_log_files": node_files,
            "config_files": config_files,
            "components_versioned": len(data.get("versions", {}))
        }
    }
    
    # Collect error information
    errors = []
    
    # Check for node logs errors
    if isinstance(data.get("node_logs", {}), dict):
        for service, log in data.get("node_logs", {}).items():
            if service == "error":
                errors.append(f"Node logs: {log}")
            elif isinstance(log, str) and log.startswith("Failed"):
                errors.append(f"Node service '{service}': {log}")
    
    # Check for pod logs errors
    if isinstance(data.get("pod_logs", {}), dict):
        if "error" in data["pod_logs"]:
            errors.append(f"Pod logs: {data['pod_logs']['error']}")
    
    # Check for other component errors
    for component in ["k8s_configs", "node_metrics", "versions"]:
        if component in data and "error" in data[component]:
            errors.append(f"{component}: {data[component]['error']}")
    
    summary["errors"] = errors
    
    # Write summary to file
    summary_file = Path(collection_dir) / "summary.yaml"
    with open(summary_file, "w") as f:
        yaml.dump(summary, f, default_flow_style=False)
    
    logger.info(f"Summary report created at {summary_file}")
    return str(summary_file)

def zip_logs(collection_dir, zip_dir):
    """Creates a compressed archive of collected logs"""
    logger.info("Creating compressed archive")
    timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    zip_file = Path(zip_dir) / f"nessie_logs_{timestamp}.tar.gz"
    
    try:
        with tarfile.open(zip_file, "w:gz") as tar:
            tar.add(collection_dir, arcname=os.path.basename(collection_dir))
        
        logger.info(f"Archive created at {zip_file}")
        return str(zip_file)
    except Exception as e:
        logger.error(f"Failed to create archive: {e}")
        return None

def enforce_retention():
    """Deletes log archives older than the retention period"""
    logger.info(f"Enforcing {RETENTION_DAYS} day retention policy")
    deleted_count = 0
    
    try:
        for path in Path(ZIP_DIR).glob("*.tar.gz"):
            file_time = datetime.fromtimestamp(path.stat().st_ctime)
            if datetime.now() - file_time > timedelta(days=RETENTION_DAYS):
                path.unlink()
                deleted_count += 1
        
        logger.info(f"Deleted {deleted_count} old log archives")
    except Exception as e:
        logger.error(f"Error enforcing retention policy: {e}")

def check_disk_space():
    """Checks available disk space and warns if running low"""
    try:
        usage = shutil.disk_usage(LOG_DIR)
        free_percent = (usage.free / usage.total) * 100
        
        if usage.free < 1024 * 1024 * 100:  # Less than 100MB free
            logger.error(f"Critical: Only {usage.free / (1024*1024):.1f}MB free space remaining!")
            return False
        elif free_percent < 10:  # Less than 10% free
            logger.warning(f"Low disk space: {free_percent:.1f}% ({usage.free / (1024*1024*1024):.1f}GB) free")
            return True
        else:
            logger.info(f"Disk space check: {free_percent:.1f}% ({usage.free / (1024*1024*1024):.1f}GB) free")
            return True
    except Exception as e:
        logger.error(f"Error checking disk space: {e}")
        return False

def check_required_tools():
    """Check if required tools are available in the container"""
    tools = {
        "journalctl": "collecting system logs",
        "helm": "collecting Helm releases"
    }
    
    missing_tools = []
    for tool, purpose in tools.items():
        success, _ = run_command(f"command -v {tool}", shell=True)
        if not success:
            missing_tools.append((tool, purpose))
    
    if missing_tools:
        logger.warning("Some tools required by Nessie are not available in this container:")
        for tool, purpose in missing_tools:
            logger.warning(f"  - Missing '{tool}' for {purpose}")
        logger.warning("Some data collection might be limited.")
    
    return len(missing_tools) == 0

def main():
    """Orchestrates log collection with fault tolerance"""
    start_time = time.time()
    logger.info("Starting log collection process")
    
    # Log configuration
    logger.info(f"Configuration: LOG_DIR={LOG_DIR}, ZIP_DIR={ZIP_DIR}, RETENTION_DAYS={RETENTION_DAYS}")
    logger.info(f"Configuration: MAX_POD_LOG_LINES={MAX_POD_LOG_LINES}, NAMESPACES_FILTER={NAMESPACES_FILTER}")
    logger.info(f"Skip settings: NODE_LOGS={SKIP_NODE_LOGS}, POD_LOGS={SKIP_POD_LOGS}, K8S_CONFIGS={SKIP_K8S_CONFIGS}, METRICS={SKIP_METRICS}, VERSIONS={SKIP_VERSIONS}")
    
    # Initialize data dictionary
    data = {}
    
    # Check prerequisites (continue even if they fail)
    prerequisites_met = True
    if not ensure_directories():
        logger.error("Failed to set up required directories, continuing with best effort")
        prerequisites_met = False
    
    if not check_disk_space():
        logger.error("Insufficient disk space, continuing with best effort")
        prerequisites_met = False
    
    # Check for required tools
    check_required_tools()
    
    # Setup Kubernetes clients
    v1_api, custom_api = setup_kubernetes_client()
    
    # Collect node logs if not skipped
    if not SKIP_NODE_LOGS:
        try:
            logger.info("Collecting node logs")
            data["node_logs"] = collect_node_logs()
        except Exception as e:
            logger.error(f"Node log collection failed: {e}")
            data["node_logs"] = {"error": str(e)}
    else:
        logger.info("Skipping node logs collection")
    
    # Collect K8s configs if not skipped and API client is available
    if not SKIP_K8S_CONFIGS and v1_api:
        try:
            logger.info("Collecting Kubernetes configurations")
            data["k8s_configs"] = collect_k8s_configs(v1_api)
        except Exception as e:
            logger.error(f"Kubernetes configuration collection failed: {e}")
            data["k8s_configs"] = {"error": str(e)}
    elif SKIP_K8S_CONFIGS:
        logger.info("Skipping Kubernetes configuration collection")
    else:
        logger.error("Kubernetes API client not available, skipping K8s configuration collection")
    
    # Collect pod logs if not skipped and API client is available
    if not SKIP_POD_LOGS and v1_api:
        try:
            logger.info("Collecting pod logs")
            data["pod_logs"] = collect_pod_logs(v1_api)
        except Exception as e:
            logger.error(f"Pod log collection failed: {e}")
            data["pod_logs"] = {"error": str(e)}
    elif SKIP_POD_LOGS:
        logger.info("Skipping pod logs collection")
    else:
        logger.error("Kubernetes API client not available, skipping pod logs collection")
    
    # Collect node metrics if not skipped and API client is available
    if not SKIP_METRICS and custom_api:
        try:
            logger.info("Collecting node metrics")
            data["node_metrics"] = collect_node_metrics(custom_api)
        except Exception as e:
            logger.error(f"Node metrics collection failed: {e}")
            data["node_metrics"] = {"error": str(e)}
    elif SKIP_METRICS:
        logger.info("Skipping node metrics collection")
    else:
        logger.error("Kubernetes Custom API client not available, skipping node metrics collection")
    
    # Collect version information if not skipped
    if not SKIP_VERSIONS:
        try:
            logger.info("Collecting version information")
            data["versions"] = collect_versions()
        except Exception as e:
            logger.error(f"Version collection failed: {e}")
            data["versions"] = {"error": str(e)}
    else:
        logger.info("Skipping version information collection")
    
    # Save collected data as individual text files
    try:
        created_files, collection_dir = save_text_logs(data, LOG_DIR)
        logger.info(f"Data saved to {collection_dir} ({len(created_files)} files)")
    except Exception as e:
        logger.error(f"Failed to save log files: {e}")
        return 1
    
    # Create summary report
    try:
        summary_file = create_summary_report(data, start_time, collection_dir)
        logger.info(f"Summary report created at {summary_file}")
    except Exception as e:
        logger.error(f"Failed to create summary report: {e}")
        summary_file = None
    
    # Create compressed archive
    try:
        archive_file = zip_logs(collection_dir, ZIP_DIR)
        if archive_file:
            logger.info(f"Archive created at {archive_file}")
    except Exception as e:
        logger.error(f"Failed to create archive: {e}")
        archive_file = None
    
    # Clean up old archives
    try:
        enforce_retention()
    except Exception as e:
        logger.error(f"Failed to enforce retention policy: {e}")
    
    # Calculate total execution time
    total_time = time.time() - start_time
    minutes, seconds = divmod(total_time, 60)
    
    # Print a comprehensive summary of the collection
    logger.info("\n" + "="*80)
    logger.info("📋 NESSIE COLLECTION SUMMARY")
    logger.info("="*80)
    
    # Collection statistics
    logger.info(f"⏱️  Execution time: {int(minutes)} minutes, {int(seconds)} seconds")
    
    # Content summary
    logger.info("\n📦 COLLECTED CONTENT:")
    if "node_logs" in data:
        success_count = len([k for k, v in data["node_logs"].items() if not str(v).startswith("Failed")])
        total_count = len(data["node_logs"])
        logger.info(f"  • System logs: {success_count}/{total_count} services")
    else:
        logger.info("  • System logs: Not collected")
    
    if "pod_logs" in data and "error" not in data["pod_logs"]:
        pod_count = len(data["pod_logs"])
        namespaces = set()
        for pod_key in data["pod_logs"].keys():
            if "/" in pod_key:
                namespace = pod_key.split("/")[0]
                namespaces.add(namespace)
        logger.info(f"  • Pod logs: {pod_count} pods from {len(namespaces)} namespaces")
    else:
        logger.info("  • Pod logs: Not collected")
    
    if "k8s_configs" in data and "namespaces" in data["k8s_configs"]:
        logger.info(f"  • Kubernetes configs: {len(data['k8s_configs']['namespaces'])} namespaces")
        if data.get("k8s_configs", {}).get("helm_releases"):
            helm_count = len(data["k8s_configs"]["helm_releases"]) if isinstance(data["k8s_configs"]["helm_releases"], list) else 0
            logger.info(f"  • Helm releases: {helm_count}")
    else:
        logger.info("  • Kubernetes configs: Not collected")
    
    if "versions" in data:
        version_count = len([v for v in data["versions"].values() if not v.startswith("Not available")])
        logger.info(f"  • Component versions: {version_count}/{len(data['versions'])}")
    else:
        logger.info("  • Component versions: Not collected")
    
    # Output location
    logger.info("\n📁 OUTPUT LOCATION:")
    if 'archive_file' in locals() and archive_file:
        logger.info(f"  • Archive: {archive_file}")
    if 'collection_dir' in locals() and collection_dir:
        logger.info(f"  • Raw data directory: {collection_dir}")
    if 'summary_file' in locals() and summary_file:
        logger.info(f"  • Summary YAML: {summary_file}")
    
    # Any issues or notes
    issues = []
    if "node_logs" in data:
        failed_services = [k for k, v in data["node_logs"].items() if str(v).startswith("Failed")]
        if failed_services:
            issues.append(f"Could not collect logs from: {', '.join(failed_services)}")
    
    if "versions" in data:
        missing_versions = [k for k, v in data["versions"].items() if v.startswith("Not available")]
        if missing_versions:
            issues.append(f"Missing version information for: {', '.join(missing_versions)}")
    
    if issues:
        logger.info("\n⚠️ NOTES:")
        for issue in issues:
            logger.info(f"  • {issue}")
    
    logger.info("\n" + "="*80)
    logger.info("Collection complete! Use the archive file for sharing with support.")
    logger.info("="*80 + "\n")
    
    return 0

if __name__ == "__main__":
    try:
        exit_code = main()
        exit(exit_code)
    except Exception as e:
        logger.critical(f"Unhandled exception: {e}")
        exit(1)