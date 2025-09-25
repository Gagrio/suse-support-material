# 🚀 Rancher Elemental Installation Guide

> **Complete guide to set up Rancher with Elemental OS management capabilities**

---

## 🎯 Prerequisites

✅ Fresh SL Micro 6.1 VM or bare metal system  
✅ Root or sudo access  
✅ Internet connectivity  
✅ At least 8GB RAM and 50GB storage  

---

## 📋 Step 1: Prepare the Base System

### 🔧 Install SL Micro 6.1 Base System
Start with a clean SL Micro 6.1 installation (Base or Default VM variant)

### 📦 Install Essential Packages
```bash
# Install QEMU guest agent and Helm
transactional-update pkg install qemu-guest-agent helm

# Reboot to apply transactional update
reboot
```

⏳ **Wait for the system to reboot completely before continuing**

---

## 🎪 Step 2: Install RKE2 Kubernetes

### 📥 Download and Install RKE2
```bash
# Download and install RKE2
curl -sfL https://get.rke2.io | sudo sh -
```

### 🚀 Start RKE2 Services
```bash
# Enable RKE2 server to start at boot
sudo systemctl enable rke2-server.service

# Start RKE2 server
sudo systemctl start rke2-server.service

# Check service status (this can take a few minutes ⏱️)
sudo systemctl status rke2-server.service
```

### ⚙️ Configure kubectl Access
```bash
# Create kubectl config directory
mkdir -p ~/.kube

# Copy RKE2 kubeconfig
sudo cp /etc/rancher/rke2/rke2.yaml ~/.kube/config
sudo chown $(id -u):$(id -g) ~/.kube/config

# Add RKE2 binaries to PATH
echo 'export PATH=$PATH:/var/lib/rancher/rke2/bin' >> ~/.bashrc
source ~/.bashrc
```

### ✅ Verify Kubernetes Cluster
```bash
# Check if cluster is ready
kubectl get nodes
```

You should see your node in `Ready` status! 🎉

---

## 🔐 Step 3: Install cert-manager

### 📚 Add cert-manager Repository
```bash
# Add Jetstack Helm repository
helm repo add jetstack https://charts.jetstack.io
helm repo update
```

### 🛠️ Install cert-manager
```bash
# Create cert-manager namespace
kubectl create namespace cert-manager

# Install cert-manager with CRDs
helm install cert-manager jetstack/cert-manager \
  --namespace cert-manager \
  --create-namespace \
  --version v1.13.0 \
  --set installCRDs=true
```

### ✅ Verify cert-manager Installation
```bash
# Check if cert-manager pods are running
kubectl get pods --namespace cert-manager
```

All pods should show `Running` status! 🟢

---

## 🐄 Step 4: Install Rancher

### 📚 Add Rancher Repository
```bash
# Add Rancher stable Helm repository
helm repo add rancher-stable https://releases.rancher.com/server-charts/stable
helm repo update
```

### 🏗️ Install Rancher Server
```bash
# Create cattle-system namespace
kubectl create namespace cattle-system

# Install Rancher with self-signed certificates
helm install rancher rancher-stable/rancher \
  --namespace cattle-system \
  --set hostname=<YOUR-VM-IP-OR-HOSTNAME> \
  --set replicas=1 \
  --set bootstrapPassword=admin
```

> **⚠️ Important:** Replace `<YOUR-VM-IP-OR-HOSTNAME>` with your actual VM IP address or hostname!

### ⏳ Wait for Rancher Deployment
```bash
# Monitor Rancher deployment status
kubectl -n cattle-system rollout status deploy/rancher
```

### 🔑 Access Information
```bash
# Display access details
echo "🌐 Rancher URL: https://<YOUR-VM-IP-OR-HOSTNAME>"
echo "👤 Username: admin"
echo "🔒 Password: admin"
```

---

## 🌟 Step 5: Install Elemental Operator

### 📦 Install Elemental CRDs
```bash
# Install Elemental Operator Custom Resource Definitions
helm install elemental-operator-crds \
  oci://registry.suse.com/rancher/elemental-operator-crds-chart \
  --namespace cattle-elemental-system \
  --create-namespace
```

### 🔧 Install Elemental Operator
```bash
# Install the main Elemental Operator
helm install elemental-operator \
  oci://registry.suse.com/rancher/elemental-operator-chart \
  --namespace cattle-elemental-system \
  --create-namespace
```

### ✅ Verify Elemental Installation
```bash
# Check Elemental Operator pods
kubectl get pods -n cattle-elemental-system
```

All pods should be `Running`! 🎯

---

## 🎨 Step 6: Enable Elemental UI Extension

### 🖥️ Access Rancher Web Interface
1. **Open your browser** and navigate to: `https://<YOUR-VM-IP>`
2. **Accept the security warning** (self-signed certificate)
3. **Login** with:
   - 👤 **Username:** `admin`
   - 🔒 **Password:** `admin`

### 🔌 Install Extensions
1. **Navigate** to ☰ menu → **Extensions** (under Configuration section)
2. **Enable extension operator** if not already enabled ⚡
3. **Find "Elemental"** in the Available tab
4. **Click "Install"** 📥
5. **Reload the page** after installation completes 🔄

---

## ✅ Step 7: Final Verification

### 🧪 Run System Checks
```bash
# Check all pods across namespaces
kubectl get pods --all-namespaces

# Verify Elemental CRDs are installed
kubectl get crd | grep elemental

# Test Rancher accessibility
curl -k https://<YOUR-VM-IP>
```

### 🎊 Success Indicators
- ✅ All pods show `Running` status
- ✅ Elemental CRDs are present
- ✅ Rancher UI loads without errors
- ✅ Elemental extension appears in Rancher sidebar

---

## 🎉 Congratulations!

You have successfully installed:
- 🎪 **RKE2 Kubernetes cluster**
- 🔐 **cert-manager for certificate management**
- 🐄 **Rancher management platform**
- 🌟 **Elemental OS management capabilities**

### 🚀 What's Next?
You're now ready to:
- Create MachineRegistrations 📝
- Build custom ISOs 💿
- Deploy and manage edge nodes 🌐
- Experience immutable OS management! 🛡️

---

## 📞 Need Help?

If you encounter any issues:
- 📖 Check the [Elemental Documentation](https://elemental.docs.rancher.com/)
- 💬 Join the [SUSE Community](https://www.suse.com/community/)
- 🐛 Report issues on [GitHub](https://github.com/rancher/elemental)

**Happy deploying!** 🚀✨
