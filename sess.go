###################################################################################################
#   Simple example for using client-go                                                            #
###################################################################################################
#   This program:                                                                                 #
#   ✅ Reads K3s/RKE2 configs from /etc/rancher/k3s/config.yaml or /etc/rancher/rke2/config.yaml. #
#   ✅ Lists installed Helm charts and saves their values.                                        #
#   ✅ Captures logs for all Pods in all namespaces.                                              #
#   ✅ Zips all generated files together.                                                         #
###################################################################################################


package main

import (
	"archive/zip"
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"path/filepath"

	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

const (
	outputDir    = "/tmp/k3s_rke2_diagnostics"
	zipFilePath  = "/tmp/k3s_rke2_diagnostics.zip"
	k3sConfig    = "/etc/rancher/k3s/config.yaml"
	rke2Config   = "/etc/rancher/rke2/config.yaml"
	helmCmd      = "helm"
	helmListArgs = "list --all-namespaces -o json"
)

func main() {
	os.MkdirAll(outputDir, os.ModePerm)

	// Save K3s/RKE2 config only if it exists
	if _, err := os.Stat(k3sConfig); err == nil {
		saveConfigFile(k3sConfig, "k3s_config.yaml")
	} else if _, err := os.Stat(rke2Config); err == nil {
		saveConfigFile(rke2Config, "rke2_config.yaml")
	}

	// List Helm charts and save values
	saveHelmCharts()

	// Fetch logs for all pods
	savePodLogs()

	// Zip all files
	zipFiles(outputDir, zipFilePath)

	fmt.Println("Diagnostics collected:", zipFilePath)
}

// Save configuration file
func saveConfigFile(configPath, outputFile string) {
	saveFile(configPath, filepath.Join(outputDir, outputFile))
}

// Fetch installed Helm charts and their values
func saveHelmCharts() {
	out, err := exec.Command(helmCmd, "list", "--all-namespaces", "-o", "json").Output()
	if err != nil {
		log.Println("Error listing Helm charts:", err)
		return
	}
	helmListFile := filepath.Join(outputDir, "helm_charts.json")
	os.WriteFile(helmListFile, out, 0644)

	// Extract values for each Helm release
	var charts []map[string]string
	json.Unmarshal(out, &charts)
	for _, chart := range charts {
		name, ns := chart["name"], chart["namespace"]
		values, err := exec.Command(helmCmd, "get", "values", name, "-n", ns, "-o", "yaml").Output()
		if err != nil {
			log.Printf("Error getting values for %s: %v\n", name, err)
			continue
		}
		os.WriteFile(filepath.Join(outputDir, fmt.Sprintf("helm_%s_%s.yaml", ns, name)), values, 0644)
	}
}

// Fetch logs for all pods in all namespaces
func savePodLogs() {
	config, _ := rest.InClusterConfig()
	clientset, _ := kubernetes.NewForConfig(config)
	pods, _ := clientset.CoreV1().Pods("").List(context.TODO(), metav1.ListOptions{})

	for _, pod := range pods.Items {
		logFile := filepath.Join(outputDir, fmt.Sprintf("pod_%s_%s.log", pod.Namespace, pod.Name))
		req := clientset.CoreV1().Pods(pod.Namespace).GetLogs(pod.Name, &metav1.PodLogOptions{})
		logs, err := req.Stream(context.TODO())
		if err != nil {
			log.Printf("Error fetching logs for pod %s/%s: %v", pod.Namespace, pod.Name, err)
			continue
		}
		defer logs.Close()
		outFile, _ := os.Create(logFile)
		io.Copy(outFile, logs)
		outFile.Close()
	}
}

// Helper function to save a file
func saveFile(src, dst string) {
	in, _ := os.Open(src)
	defer in.Close()
	out, _ := os.Create(dst)
	defer out.Close()
	io.Copy(out, in)
}

// Zip collected files
func zipFiles(srcDir, destZip string) {
	outFile, _ := os.Create(destZip)
	defer outFile.Close()
	zipWriter := zip.NewWriter(outFile)
	defer zipWriter.Close()

	filepath.Walk(srcDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() {
			return nil
		}
		relPath, _ := filepath.Rel(srcDir, path)
		f, _ := zipWriter.Create(relPath)
		inFile, _ := os.Open(path)
		defer inFile.Close()
		io.Copy(f, inFile)
		return nil
	})
}
