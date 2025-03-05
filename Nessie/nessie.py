###################################################################################################
#   🦕  Node Environment Support Script for Inspection and Export  🦕                             #
#   ☸️🐍  Enhanced script for using native Kubernetes python client  ☸️🐍                         #
###################################################################################################
#   This program:                                                                                 #
#   ✅ Loads K3s/RKE2 configs                                                                     #
#   ✅ Collects logs from SLE Micro and SUSE EDGE apps installed on it                            #
#   ✅ Captures logs for all SUSE EDGE Pods in all namespaces                                     #
#   ✅ Gathers the versions of all SUSE EDGE apps and relevant software                           #
#   ✅ Creates a summary report of collected information                                          #
#   ✅ Stores data in YAML format and creates compressed archives                                 #
###################################################################################################

import os
import yaml
import time
import logging
import shutil
import tarfile
import subprocess
import concurrent.futures

REQUIRED_PACKAGES = ["kubernetes"]

from datetime import datetime, timedelta
from kubernetes import client, config
from pathlib import Path

# Logging setup
logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s")
logger = logging.getLogger(__name__)

# Configuration parameters
LOG_DIR = "/var/log/cluster-logs"  # Base directory for storing collected logs
ZIP_DIR = f"{LOG_DIR}/archives"    # Directory for compressed archives
MAX_LOG_SIZE = 1 * 1024 * 1024 * 1024  # 1GB maximum log storage size
RETENTION_DAYS = 30                # Number of days to keep archived logs
MAX_POD_LOG_LINES = 1000          # Maximum number of log lines to collect per container
NAMESPACES_FILTER = None          # Set to a list of namespaces to limit collection scope, None for all

class ProgressTracker:
    """Simple progress tracking for long-running operations"""
    def __init__(self, total_items, operation_name):
        self.total = total_items
        self.current = 0
        self.operation_name = operation_name
        self.start_time = time.time()
        logger.info(f"Starting {operation_name} (0/{total_items})")
    
    def update(self, increment=1):
        """Update progress counter and log status"""
        self.current += increment
        percent = (self.current / self.total) * 100
        elapsed = time.time() - self.start_time
        logger.info(f"{self.operation_name} progress: {self.current}/{self.total} ({percent:.1f}%) - {elapsed:.1f}s elapsed")
    
    def complete(self):
        """Mark operation as complete and log final stats"""
        total_time = time.time() - self.start_time
        logger.info(f"Completed {self.operation_name} in {total_time:.1f}s")
        return total_time

def ensure_directories():
    """Ensure all required directories exist and are writable"""
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
    """Initialize and return Kubernetes API clients"""
    try:
        # Try to load in-cluster config first (for running as a pod)
        config.load_incluster_config()
        logger.info("Using in-cluster Kubernetes configuration")
    except config.ConfigException:
        # Fall back to local config (for running outside the cluster)
        try:
            config.load_kube_config()
            logger.info("Using local Kubernetes configuration")
        except config.ConfigException as e:
            logger.error(f"Failed to load Kubernetes configuration: {e}")
            return None, None
    
    return client.CoreV1Api(), client.CustomObjectsApi()

# Dictionary of system services to collect logs from on SUSE Linux Micro
NODE_SERVICES = {
    "system": "journalctl -n 1000 --no-pager",          # Last 1000 system logs
    "combustion": "journalctl -u combustion --no-pager", # Combustion service logs
    "hauler": "journalctl -u hauler --no-pager",        # Hauler service logs
    "nmc": "journalctl -u nm-configurator --no-pager"   # Network Manager Configurator logs
}

# Commands to retrieve version information for various components
VERSION_COMMANDS = {
    "k3s": "k3s --version",
    "rke2": "rke2 --version",
    "helm": "helm version --short",
    "kubectl": "kubectl version --short",
    "upgrade-controller": "kubectl get deployment upgrade-controller -n kube-system -o jsonpath='{.spec.template.spec.containers[0].image}'",
    "endpoint-copier-operator": "kubectl get deployment endpoint-copier-operator -n kube-system -o jsonpath='{.spec.template.spec.containers[0].image}'",
    "metallb": "kubectl get deployment -n metallb-system -o jsonpath='{.items[*].spec.template.spec.containers[*].image}'"
}

def run_command(command, shell=False):
    """
    Run a command safely and return its output
    
    Args:
        command (str or list): Command to execute
        shell (bool): Whether to use shell execution (use carefully)
    
    Returns:
        tuple: (success, output) where success is a boolean and output is the command output or error message
    """
    try:
        args = command if isinstance(command, list) else command
        result = subprocess.run(
            args, 
            shell=shell, 
            capture_output=True, 
            text=True,
            timeout=60  # Add timeout to prevent hanging
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
    """
    Collect logs from various system services on the host node.
    
    Returns:
        dict: A dictionary with service names as keys and their logs as values
    """
    logs = {}
    progress = ProgressTracker(len(NODE_SERVICES), "Node log collection")
    
    for name, cmd in NODE_SERVICES.items():
        success, output = run_command(cmd, shell=True)
        logs[name] = output if success else f"Failed to collect logs: {output}"
        progress.update()
    
    progress.complete()
    return logs

def collect_k8s_configs(v1_api):
    """
    Collect Kubernetes cluster configuration and state information.
    
    Args:
        v1_api: Initialized Kubernetes CoreV1Api client
    
    Returns:
        dict: A dictionary containing namespace information, Helm releases, and Metal3 logs
    """
    data = {}
    logger.info("Collecting Kubernetes configuration information")
    
    try:
        # Get all namespaces in the cluster
        namespaces = v1_api.list_namespace()
        data["namespaces"] = [ns.metadata.name for ns in namespaces.items]
        logger.info(f"Collected information for {len(data['namespaces'])} namespaces")
        
        # Get all Helm releases across all namespaces
        success, helm_output = run_command(["helm", "list", "-A", "-o", "yaml"])
        if success:
            data["helm_releases"] = yaml.safe_load(helm_output)
            logger.info(f"Collected information for {len(data['helm_releases']) if isinstance(data['helm_releases'], list) else 0} Helm releases")
        else:
            logger.warning(f"Failed to fetch Helm releases: {helm_output}")
            data["helm_releases"] = []
            
        # Collect Metal3 related logs (used for bare metal provisioning)
        success, metal3_logs = run_command("journalctl -u ironic -u metal3 -n 1000 --no-pager", shell=True)
        if success:
            data["metal3_logs"] = metal3_logs
            logger.info("Collected Metal3 logs")
        else:
            logger.warning(f"Failed to collect Metal3 logs: {metal3_logs}")
            data["metal3_logs"] = "No Metal3 logs available"
    except Exception as e:
        logger.error(f"Error collecting Kubernetes configs: {e}")
    
    return data

def collect_pod_logs(v1_api):
    """
    Collect logs from pods, optionally filtering by namespace.
    
    Args:
        v1_api: Initialized Kubernetes CoreV1Api client
    
    Returns:
        dict: A nested dictionary with pod names as keys and container logs as values
    """
    pod_logs = {}
    
    try:
        # Get pods (filtered by namespace if specified)
        if NAMESPACES_FILTER:
            pods = []
            for ns in NAMESPACES_FILTER:
                ns_pods = v1_api.list_namespaced_pod(ns).items
                pods.extend(ns_pods)
        else:
            pods = v1_api.list_pod_for_all_namespaces(watch=False).items
        
        progress = ProgressTracker(len(pods), "Pod log collection")
        
        # For each pod, collect logs from all containers
        for pod in pods:
            pod_name = pod.metadata.name
            namespace = pod.metadata.namespace
            containers = [c.name for c in pod.spec.containers]
            
            pod_logs[f"{namespace}/{pod_name}"] = {}
            
            for container in containers:
                try:
                    # Use tail lines parameter to limit log size
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
    
    return pod_logs

def collect_node_metrics(custom_api):
    """
    Collect node metrics using the Kubernetes metrics API.
    
    Args:
        custom_api: Initialized Kubernetes CustomObjectsApi client
    
    Returns:
        dict: Node metrics information or empty dict if metrics server is not available
    """
    logger.info("Collecting node metrics")
    try:
        response = custom_api.list_cluster_custom_object("metrics.k8s.io", "v1beta1", "nodes")
        logger.info(f"Collected metrics for {len(response.get('items', []))} nodes")
        return response
    except Exception as e:
        logger.warning(f"Metrics server not available: {e}")
        return {}

def collect_versions():
    """
    Collect version information for various components.
    
    Returns:
        dict: A dictionary with component names as keys and their version information as values
    """
    versions = {}
    progress = ProgressTracker(len(VERSION_COMMANDS), "Version collection")
    
    for component, cmd in VERSION_COMMANDS.items():
        success, output = run_command(cmd, shell=True)
        versions[component] = output.strip() if success else f"Not available: {output}"
        progress.update()
    
    progress.complete()
    return versions

def create_summary_report(data, start_time, output_file):
    """
    Create a summary report of the collected data.
    
    Args:
        data: The collected data dictionary
        start_time: The time when collection started
        output_file: Path to the main output file
    
    Returns:
        str: Path to the summary report file
    """
    logger.info("Creating summary report")
    
    summary = {
        "collection_info": {
            "timestamp": datetime.now().isoformat(),
            "duration_seconds": time.time() - start_time,
            "output_file": str(output_file)
        },
        "stats": {
            "namespaces": len(data.get("k8s_configs", {}).get("namespaces", [])),
            "helm_releases": len(data.get("k8s_configs", {}).get("helm_releases", [])) if isinstance(data.get("k8s_configs", {}).get("helm_releases", []), list) else 0,
            "pods_collected": len(data.get("pod_logs", {})),
            "node_services": len(data.get("node_logs", {})),
            "components_versioned": len(data.get("versions", {}))
        },
        "versions": data.get("versions", {})
    }
    
    # Add information about any collection errors
    errors = []
    for service, log in data.get("node_logs", {}).items():
        if isinstance(log, str) and log.startswith("Failed"):
            errors.append(f"Node service '{service}': {log}")
    
    for pod_container, logs in data.get("pod_logs", {}).items():
        for container, log in logs.items():
            if isinstance(log, str) and log.startswith("Error:"):
                errors.append(f"Pod container '{pod_container}/{container}': {log}")
    
    summary["errors"] = errors
    
    # Write summary to file
    summary_file = Path(LOG_DIR) / f"summary_{datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}.yaml"
    with open(summary_file, "w") as f:
        yaml.dump(summary, f, default_flow_style=False)
    
    logger.info(f"Summary report created at {summary_file}")
    return str(summary_file)

def zip_logs(output_file, summary_file):
    """
    Create a compressed archive of collected logs.
    
    Args:
        output_file: Path to the main output file
        summary_file: Path to the summary file
    
    Returns:
        str: Path to the created archive
    """
    logger.info("Creating compressed archive")
    timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    zip_file = Path(ZIP_DIR) / f"logs_{timestamp}.tar.gz"
    
    try:
        with tarfile.open(zip_file, "w:gz") as tar:
            tar.add(output_file, arcname=os.path.basename(output_file))
            tar.add(summary_file, arcname=os.path.basename(summary_file))
        
        logger.info(f"Archive created at {zip_file}")
        return str(zip_file)
    except Exception as e:
        logger.error(f"Failed to create archive: {e}")
        return None

def enforce_retention():
    """
    Delete log archives that are older than the retention period (RETENTION_DAYS).
    This helps manage disk space by removing outdated logs.
    """
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
    """
    Check available disk space and warn if running low.
    
    Returns:
        bool: True if sufficient space is available, False otherwise
    """
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

def parallel_collect_data():
    """
    Collect all data using parallel execution for efficiency.
    
    Returns:
        dict: All collected data
    """
    logger.info("Starting parallel data collection")
    v1_api, custom_api = setup_kubernetes_client()
    
    if not v1_api or not custom_api:
        logger.error("Failed to initialize Kubernetes clients, aborting collection")
        return None
    
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as executor:
        # Submit collection tasks to be executed in parallel
        node_logs_future = executor.submit(collect_node_logs)
        k8s_configs_future = executor.submit(collect_k8s_configs, v1_api)
        pod_logs_future = executor.submit(collect_pod_logs, v1_api)
        node_metrics_future = executor.submit(collect_node_metrics, custom_api)
        versions_future = executor.submit(collect_versions)
        
        # Gather results as they complete
        data = {
            "node_logs": node_logs_future.result(),
            "k8s_configs": k8s_configs_future.result(),
            "pod_logs": pod_logs_future.result(),
            "node_metrics": node_metrics_future.result(),
            "versions": versions_future.result()
        }
    
    logger.info("Parallel data collection completed")
    return data

def main():
    """
    Main function that orchestrates one-time log collection.
    """
    start_time = time.time()
    logger.info("Starting log collection process")
    
    # Check prerequisites
    if not ensure_directories():
        logger.error("Failed to set up required directories, aborting")
        return 1
    
    if not check_disk_space():
        logger.error("Insufficient disk space, aborting")
        return 1
    
    # Collect all data
    data = parallel_collect_data()
    if not data:
        logger.error("Data collection failed")
        return 1
    
    # Save collected data to YAML file
    timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    output_file = Path(LOG_DIR) / f"collected_data_{timestamp}.yaml"
    
    try:
        with open(output_file, "w") as f:
            yaml.dump(data, f, default_flow_style=False)
        logger.info(f"Data saved to {output_file}")
    except Exception as e:
        logger.error(f"Failed to save data: {e}")
        return 1
    
    # Create summary report
    summary_file = create_summary_report(data, start_time, output_file)
    
    # Create compressed archive
    archive_file = zip_logs(output_file, summary_file)
    
    # Clean up old archives
    enforce_retention()
    
    # Final report
    total_time = time.time() - start_time
    logger.info(f"Log collection completed in {total_time:.1f} seconds")
    logger.info(f"Results saved to {output_file}")
    if archive_file:
        logger.info(f"Archive created at {archive_file}")
    
    return 0

if __name__ == "__main__":
    exit_code = main()
    exit(exit_code)