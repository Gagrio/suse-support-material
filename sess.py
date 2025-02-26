###################################################################################################
#   Simple example for using native kubernetes python client                                      #
###################################################################################################
#   This program:                                                                                 #
#   ✅ Loads K3s/RKE2 configs                                                                     #
#   ✅ Colelcts logs from SLE Micro and SUSE EDGE apps installed on it.                           #
#   ✅ Captures logs for all SUSE EDGE Pods in all namespaces.                                    #
#   ✅ Gathers the versions of all SUSE EDGE apps and relevant software.                          #
#   ✅ Zips all generated files together.                                                         #
###################################################################################################


import os
import json
import time
import logging
import shutil
import tarfile
import subprocess
from datetime import datetime, timedelta
from kubernetes import client, config

# Logging setup
logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s")
LOG_DIR = "/var/log/cluster-logs"
ZIP_DIR = f"{LOG_DIR}/archives"
MAX_LOG_SIZE = 1 * 1024 * 1024 * 1024  # 1GB
RETENTION_DAYS = 30

# Ensure log directories exist
os.makedirs(LOG_DIR, exist_ok=True)
os.makedirs(ZIP_DIR, exist_ok=True)

# Load Kubernetes configuration
try:
    config.load_incluster_config()  # For inside-cluster execution
except config.ConfigException:
    config.load_kube_config()  # Fallback for local testing

v1 = client.CoreV1Api()
custom_api = client.CustomObjectsApi()

# Services to collect logs from on SUSE Linux Micro
NODE_SERVICES = {
    "system": "journalctl -n 1000 --no-pager",
    "combustion": "journalctl -u combustion --no-pager",
    "hauler": "journalctl -u hauler --no-pager",
    "nmc": "journalctl -u nm-configurator --no-pager"
}

VERSION_COMMANDS = {
    "k3s": "k3s --version",
    "rke2": "rke2 --version",
    "helm": "helm version --short",
    "kubectl": "kubectl version --short",
    "upgrade-controller": "kubectl get deployment upgrade-controller -n kube-system -o jsonpath='{.spec.template.spec.containers[0].image}'",
    "endpoint-copier-operator": "kubectl get deployment endpoint-copier-operator -n kube-system -o jsonpath='{.spec.template.spec.containers[0].image}'",
    "metallb": "kubectl get deployment -n metallb-system -o jsonpath='{.items[*].spec.template.spec.containers[*].image}'"
}

def collect_node_logs():
    logs = {}
    for name, cmd in NODE_SERVICES.items():
        try:
            result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
            logs[name] = result.stdout if result.returncode == 0 else f"Failed to collect {name} logs"
        except Exception as e:
            logs[name] = f"Error: {e}"
    return logs

def collect_k8s_configs():
    data = {}
    try:
        data["namespaces"] = v1.list_namespace().to_dict()
        helm_cmd = "helm list -A -o json"
        helm_data = subprocess.run(helm_cmd.split(), capture_output=True, text=True)
        if helm_data.returncode == 0:
            data["helm_releases"] = json.loads(helm_data.stdout)
        else:
            logging.warning("Failed to fetch Helm releases")
        data["metal3_logs"] = subprocess.run("journalctl -u ironic -u metal3 -n 1000 --no-pager", shell=True, capture_output=True, text=True).stdout
    except Exception as e:
        logging.error(f"Error collecting Kubernetes configs: {e}")
    return data

def collect_pod_logs():
    pod_logs = {}
    try:
        pods = v1.list_pod_for_all_namespaces(watch=False)
        for pod in pods.items:
            pod_name = pod.metadata.name
            namespace = pod.metadata.namespace
            containers = [c.name for c in pod.spec.containers]
            pod_logs[pod_name] = {}
            for container in containers:
                try:
                    log_data = v1.read_namespaced_pod_log(name=pod_name, namespace=namespace, container=container)
                    pod_logs[pod_name][container] = log_data
                except Exception:
                    logging.warning(f"Could not fetch logs for {pod_name}/{container}")
    except Exception as e:
        logging.error(f"Error collecting pod logs: {e}")
    return pod_logs

def collect_node_metrics():
    try:
        response = custom_api.list_cluster_custom_object("metrics.k8s.io", "v1beta1", "nodes")
        return response
    except Exception:
        logging.warning("Metrics server not available")
        return {}

def collect_versions():
    versions = {}
    for component, cmd in VERSION_COMMANDS.items():
        try:
            output = subprocess.run(cmd, shell=True, capture_output=True, text=True)
            if output.returncode == 0:
                versions[component] = output.stdout.strip()
            else:
                logging.warning(f"Could not get version for {component}")
        except Exception as e:
            logging.error(f"Error fetching version for {component}: {e}")
    return versions

def zip_logs():
    now = datetime.now()
    zip_file = f"{ZIP_DIR}/logs_{now.strftime('%Y-%m-%d_%H-%M-%S')}.tar.gz"
    with tarfile.open(zip_file, "w:gz") as tar:
        tar.add(LOG_DIR, arcname=os.path.basename(LOG_DIR))

def enforce_retention():
    for filename in os.listdir(ZIP_DIR):
        filepath = os.path.join(ZIP_DIR, filename)
        file_time = datetime.fromtimestamp(os.path.getctime(filepath))
        if datetime.now() - file_time > timedelta(days=RETENTION_DAYS):
            os.remove(filepath)
            logging.info(f"Deleted old log archive: {filename}")

def main():
    while True:
        logging.info("Starting data collection")
        data = {
            "node_logs": collect_node_logs(),
            "k8s_configs": collect_k8s_configs(),
            "pod_logs": collect_pod_logs(),
            "node_metrics": collect_node_metrics(),
            "versions": collect_versions()
        }
        timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
        with open(f"{LOG_DIR}/collected_data_{timestamp}.json", "w") as f:
            json.dump(data, f, indent=4)
        if shutil.disk_usage(LOG_DIR).used > MAX_LOG_SIZE:
            logging.warning("Log storage exceeded 1GB! Consider cleaning up old logs.")
        zip_logs()
        enforce_retention()
        time.sleep(86400)  # Run once a day

if __name__ == "__main__":
    main()
