# 🚀 Rancher Elemental Demo Guide

> **Complete hands-on demonstration of Elemental's OS management capabilities**

---

## 📋 Demo Plan

This demo covers four key areas of Elemental functionality:

1. **🔧 Machine Registration & Image Creation** - Basic setup and automated provisioning
2. **🏷️ Inventory Management** - Labels, annotations, and machine discovery  
3. **☁️ Cloud-init Configuration** - Advanced user management and system customization
4. **🔨 Custom Image Creation** - Building specialized OS images with additional software

---

## 1. 🔧 Machine Registration & Image Creation

### 📝 Create Basic Machine Registration

First, let's create a basic machine registration that defines how systems should be configured:

```yaml
# demo-registration.yaml
apiVersion: elemental.cattle.io/v1beta1
kind: MachineRegistration
metadata:
  name: demo-fleet
  namespace: fleet-default
spec:
  config:
    cloud-config:
      users:
        - name: demo
          passwd: demo123
      write_files:
        - path: /etc/hosts
          permissions: '0644'
          append: true
          content: |
            127.0.0.1 localhost
            10.114.128.118 demomgmt.local
    elemental:
      install:
        reboot: false
        device: /dev/vda
        debug: true
      registration:
        emulate-tpm: true
        emulated-tpm-seed: -1
  machineInventoryLabels:
    environment: "demo"
    location: "lab"
    manufacturer: "${System Information/Manufacturer}"
    productName: "${System Information/Product Name}"
    serialNumber: "${System Information/Serial Number}"
    machineUUID: "${System Information/UUID}"
  machineInventoryAnnotations:
    demo-session: "live-demo"
```

### 💿 Create Bootable ISO Image

Next, create a SeedImage that builds a bootable ISO:

```yaml
# demo-seedimage-with-dns.yaml
apiVersion: elemental.cattle.io/v1beta1
kind: SeedImage
metadata:
  name: demo-iso
  namespace: fleet-default
spec:
  type: iso
  baseImage: "registry.suse.com/suse/sl-micro/6.1/baremetal-iso-image:2.2.0-4.3"
  cleanupAfterMinutes: 7200
  cloud-config:
    users:
      - name: demo
        passwd: demo123
    write_files:
      - path: /etc/hosts
        permissions: '0644'
        append: true
        content: |
          127.0.0.1 localhost
          10.114.128.118 demomgmt.local
  registrationRef:
    apiVersion: elemental.cattle.io/v1beta1
    kind: MachineRegistration
    name: demo-fleet
    namespace: fleet-default
```

### 🚀 Deploy the Configuration

```bash
# Apply the machine registration
kubectl apply -f demo-registration.yaml

# Apply the seed image
kubectl apply -f demo-seedimage-with-dns.yaml
```

### 🖥️ VM Creation & Auto-Registration

1. **Monitor the ISO build** 📊
   ```bash
   kubectl get seedimage demo-iso -n fleet-default -w
   ```

2. **Download and use the ISO** 💾
   - Use the generated ISO to create VMs in your virtualization platform
   - Boot the VMs from the ISO
   - Watch as they automatically register with Rancher! ✨

3. **Check the results** 🎯
   - Navigate to Rancher UI → OS Management → Inventory
   - See your newly registered machines appear automatically

---

## 2. 🏷️ Inventory Management in Action

### 🔍 View Machine Labels

See how machines are automatically labeled during registration:

```bash
# Show all machine inventories with their labels
kubectl get machineinventory -n fleet-default --show-labels
```

**Expected Output:** 📋
```
NAME                                     AGE   LABELS
m-975628cc-04cb-40c7-8306-0d32fa45b210   64m   environment=demo,location=lab,machineUUID=92f4077b-7eb3-5779-992c-986567b6bd0f,manufacturer=KubeVirt,productName=None,serialNumber=Not-Specified
m-e1952f5c-8734-4593-87cd-0abaff2513ac   18h   environment=demo,location=lab,machineUUID=421a90be-76b5-5e13-8a52-5b0fed33801e,manufacturer=KubeVirt,productName=None,serialNumber=Not-Specified
```

### 🎯 Filter by Labels

Use labels to find specific machines:

```bash
# Find all machines in the lab location
kubectl get machineinventory -n fleet-default -l location=lab
```

**Result:** 🎉
```
NAME                                     AGE
m-975628cc-04cb-40c7-8306-0d32fa45b210   62m
m-e1952f5c-8734-4593-87cd-0abaff2513ac   18h
```

### 📝 View Annotations

Annotations provide detailed metadata about each machine:

```bash
# Show all annotations
kubectl get machineinventory -n fleet-default -o yaml | grep -A 10 "annotations:"
```

Or use a more focused view:

```bash
# Get specific annotation values
kubectl get machineinventory -n fleet-default -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.metadata.annotations.demo-session}{"\n"}{end}'
```

**Output:** 📊
```
m-975628cc-04cb-40c7-8306-0d32fa45b210  live-demo
m-e1952f5c-8734-4593-87cd-0abaff2513ac  live-demo
```

### 🖥️ UI Exploration

**In Rancher UI:**
1. Navigate to **OS Management** → **Inventory** 📋
2. Click on any machine to see:
   - 🏷️ All labels applied automatically
   - 📝 System information gathered during registration
   - 🔧 Hardware details discovered

---

## 3. ☁️ Advanced Cloud-init Configuration

### 👥 Advanced User & Security Setup

Create a more sophisticated machine registration with multiple users and SSH keys:

```yaml
# demo-advanced-registration.yaml
apiVersion: elemental.cattle.io/v1beta1
kind: MachineRegistration
metadata:
  name: demo-fleet-advanced
  namespace: fleet-default
spec:
  config:
    cloud-config:
      users:
        - name: root
          passwd: root
        - name: admin
          shell: "/bin/bash"
          ssh_authorized_keys:
            - ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIO0Ndoc42s70At7WJAjqQqJpYkltMUV5tpwAUmmNdIpi
      write_files:
        - path: /etc/hosts
          permissions: '0644'
          owner: root:root
          content: |
            127.0.0.1 localhost
            10.114.128.118 demomgmt.local
        - path: /etc/motd
          permissions: '0644'
          owner: root:root
          content: |
            ==========================================
            Advanced Elemental Demo Machine
            ==========================================
            - SSH key authentication configured
            - Custom MOTD deployed
            - Configured with cloud-init automation
            ==========================================
        - path: /etc/sudoers.d/admin
          permissions: '0440'
          owner: root:root
          content: |
            admin ALL=(ALL) NOPASSWD:ALL
      runcmd:
        - echo "root" | passwd --stdin root
        - hostname -I
    elemental:
      install:
        reboot: false
        device: /dev/vda
        debug: true
      registration:
        emulate-tpm: true
        emulated-tpm-seed: -1
  machineInventoryLabels:
    environment: "demo"
    location: "lab"
    config-type: "advanced"
    manufacturer: "${System Information/Manufacturer}"
    productName: "${System Information/Product Name}"
    serialNumber: "${System Information/Serial Number}"
    machineUUID: "${System Information/UUID}"
  machineInventoryAnnotations:
    demo-session: "advanced-config"
```

### 💿 Create Advanced SeedImage

```yaml
# demo-advanced-seedimage.yaml
apiVersion: elemental.cattle.io/v1beta1
kind: SeedImage
metadata:
  name: demo-iso-advanced
  namespace: fleet-default
spec:
  type: iso
  baseImage: "registry.suse.com/suse/sl-micro/6.1/baremetal-iso-image:2.2.0-4.3"
  cleanupAfterMinutes: 7200
  cloud-config:
    write_files:
      - path: /etc/hosts
        permissions: '0644'
        append: true
        content: |
          127.0.0.1 localhost
          10.114.128.118 demomgmt.local
  registrationRef:
    apiVersion: elemental.cattle.io/v1beta1
    kind: MachineRegistration
    name: demo-fleet-advanced
    namespace: fleet-default
```

### 🔄 Deploy and Access

```bash
# Apply the advanced configuration
kubectl apply -f demo-advanced-registration.yaml
kubectl apply -f demo-advanced-seedimage.yaml

# Monitor the build process
kubectl get pods -n fleet-default | grep advanced

# When complete, set up access to the ISO
kubectl port-forward -n fleet-default pod/demo-iso-advanced 8080:80 --address=0.0.0.0 &

# Find the ISO
curl http://10.114.128.118:8080/
```

**Expected Output:** 📋
```html
<!doctype html>
<meta name="viewport" content="width=device-width">
<pre>
<a href="./demo-fleet-advanced-2025-09-12T12:47:22Z.iso">demo-fleet-advanced-2025-09-12T12:47:22Z.iso</a>
<a href="./demo-fleet-advanced-2025-09-12T12:47:22Z.iso.sha256">demo-fleet-advanced-2025-09-12T12:47:22Z.iso.sha256</a>
</pre>
```

### 🖥️ Test the Configuration

1. **Boot a VM** from the new ISO 💿
2. **Access the console** and verify:
   - ✅ Custom MOTD appears at login
   - ✅ SSH key authentication works for `admin` user
   - ✅ Sudoers configuration is applied
   - ✅ All files are written correctly

---

## 4. 🔨 Custom Image Creation

### 🏗️ Build Custom OS with Cockpit

Create a sophisticated multi-stage Dockerfile that adds Cockpit web management:

```dockerfile
# =============================================================================
# STAGE 1: Build Custom OS with Cockpit
# =============================================================================
FROM registry.suse.com/suse/sl-micro/6.1/baremetal-os-container:latest AS custom-os

# Build arguments
ARG SUSE_REGISTRATION_CODE
ARG IMAGE_REPO=demo/custom-cockpit-iso
ARG IMAGE_TAG=v1.0.0

# Copy SUSE Connect RPM
COPY suseconnect-ng-1.13.0-slfo.1.1_1.1.x86_64.rpm /tmp/suseconnect.rpm

# Install SUSE Connect and register
RUN rpm -ivh /tmp/suseconnect.rpm && rm -f /tmp/suseconnect.rpm && \
    SUSEConnect -p SL-Micro/6.1/x86_64 --gpg-auto-import-keys -r $SUSE_REGISTRATION_CODE && \
    zypper --non-interactive --no-gpg-checks refresh

# Install Cockpit
RUN zypper refresh && \
    zypper install -y cockpit && \
    zypper clean --all

# Enable Cockpit service
RUN systemctl enable cockpit.socket

# Update os-release with proper version information
RUN sed -i "s/IMAGE_TAG=.*/IMAGE_TAG=\"${IMAGE_TAG}\"/" /etc/os-release && \
    sed -i "s/IMAGE_REPO=.*/IMAGE_REPO=\"${IMAGE_REPO}\"/" /etc/os-release && \
    sed -i "s|IMAGE=.*|IMAGE=\"${IMAGE_REPO}:${IMAGE_TAG}\"|" /etc/os-release

# Rebuild initrd with elemental toolkit
RUN elemental init --force elemental-rootfs,grub-config,dracut-config,cloud-config-essentials,elemental-setup

# =============================================================================
# STAGE 2: Build ISO from Custom OS
# =============================================================================
FROM registry.suse.com/suse/sl-micro/6.1/baremetal-os-container:latest AS iso-builder

# Architecture support
ARG TARGETARCH=x86_64

# Set working directory
WORKDIR /iso

# Copy the entire custom OS filesystem
COPY --from=custom-os / rootfs/

# Clean up problematic files that can cause build issues
RUN rm -f rootfs/etc/resolv.conf && \
    rm -rf rootfs/tmp/* && \
    rm -rf rootfs/var/cache/* || true

# Build the ISO using elemental toolkit
RUN elemental build-iso \
    dir:rootfs \
    --bootloader-in-rootfs \
    --squash-no-compression \
    -o /output \
    -n "elemental-${TARGETARCH}" \
    --debug

# =============================================================================
# STAGE 3: Package ISO in Busybox Container
# =============================================================================
FROM busybox

# Copy the built ISO to the expected location
COPY --from=iso-builder /output /elemental-iso

# Set the entrypoint for ISO extraction
ENTRYPOINT ["busybox", "sh", "-c"]
```

### 🔨 Build the Custom Image

```bash
# Set your SUSE registration code
export SUSE_REGISTRATION_CODE="YOUR_ACTUAL_REGISTRATION_CODE_HERE"

# Build the custom image
podman build \
    --build-arg SUSE_REGISTRATION_CODE=$SUSE_REGISTRATION_CODE \
    --build-arg IMAGE_REPO=demo/custom-cockpit-iso \
    --build-arg IMAGE_TAG=v1.0.0 \
    --build-arg TARGETARCH=x86_64 \
    -t demo/custom-cockpit-iso:v1.0.0 \
    -f Dockerfile \
    .
```

### ✅ Verify the Build

```bash
# Check the image was created
podman image ls | head -2

# Verify the ISO is inside the container
podman run --rm demo/custom-cockpit-iso:v1.0.0 "ls -la /elemental-iso/"

# Check ISO size
podman run --rm demo/custom-cockpit-iso:v1.0.0 "du -h /elemental-iso/*.iso"

# Extract the ISO
podman run --privileged --rm -v $(pwd):/host demo/custom-cockpit-iso:v1.0.0 "cp /elemental-iso/*.iso /host"

# Verify ISO file
file elemental-x86_64.iso
```

**Expected Output:** 📊
```
elemental-x86_64.iso: ISO 9660 CD-ROM filesystem data (DOS/MBR boot sector) 'COS_LIVE' (bootable)
```

### 📦 Import to Kubernetes

```bash
# Export from Podman
podman save demo/custom-cockpit-iso:v1.0.0 -o custom-cockpit-iso.tar

# Import to containerd (RKE2/K3s)
sudo /var/lib/rancher/rke2/bin/ctr -n k8s.io --address /run/k3s/containerd/containerd.sock images import custom-cockpit-iso.tar

# Verify import
sudo /var/lib/rancher/rke2/bin/ctr -n k8s.io --address /run/k3s/containerd/containerd.sock images list | grep custom-cockpit-iso
```

### 🖥️ Create Machine Registration for Custom Image

```yaml
# demo-custom-registration.yaml
apiVersion: elemental.cattle.io/v1beta1
kind: MachineRegistration
metadata:
  name: demo-fleet-custom
  namespace: fleet-default
spec:
  config:
    cloud-config:
      users:
        - name: root
          passwd: rancher123
      write_files:
        - path: /etc/hosts
          permissions: '0644'
          content: |
            127.0.0.1 localhost
            10.114.128.118 demomgmt.local
        - path: /etc/motd
          permissions: '0644'
          content: |
            ==========================================
            Custom Elemental Demo - Cockpit Enabled
            ==========================================
            Access Cockpit: https://this-ip:9090
            Login: root / rancher123
            ==========================================
      runcmd:
        - systemctl status cockpit.socket
    elemental:
      install:
        reboot: false
        device: /dev/vda
        debug: true
      registration:
        emulate-tpm: true
        emulated-tpm-seed: -1
  machineInventoryLabels:
    environment: "demo"
    location: "lab"
    config-type: "custom-cockpit"
    cockpit-enabled: "true"
  machineInventoryAnnotations:
    demo-session: "demo-custom-iso"
```

### 💿 Create Custom SeedImage

```yaml
# demo-custom-seedimage.yaml
apiVersion: elemental.cattle.io/v1beta1
kind: SeedImage
metadata:
  name: demo-custom-iso
  namespace: fleet-default
spec:
  type: iso
  baseImage: "localhost/demo/custom-cockpit-iso:v1.0.0"
  cleanupAfterMinutes: 7200
  cloud-config:
    write_files:
      - path: /etc/hosts
        permissions: '0644'
        content: |
          127.0.0.1 localhost
          10.114.128.118 demomgmt.local
  registrationRef:
    apiVersion: elemental.cattle.io/v1beta1
    kind: MachineRegistration
    name: demo-fleet-custom
    namespace: fleet-default
```

### 🚀 Deploy Custom Configuration

```bash
# Apply the custom configurations
kubectl apply -f demo-custom-registration.yaml
kubectl apply -f demo-custom-seedimage.yaml

# Monitor the build
kubectl get seedimage demo-custom-iso -n fleet-default -w

# When complete, access the ISO
kubectl port-forward -n fleet-default pod/demo-custom-iso 8086:80 --address=0.0.0.0 &

# Find the ISO
curl http://10.114.128.118:8086/
```

### 🎯 Test Custom Image

1. **Create VM** from the custom ISO 💿
2. **Boot the system** and verify:
   - ✅ Cockpit service is running
   - ✅ Custom MOTD displays Cockpit access info
   - ✅ Web interface accessible at `https://vm-ip:9090`
   - ✅ Machine appears in Rancher with custom labels

---

## 🎉 Demo Summary

You've now demonstrated:

- **🔧 Automated provisioning** with zero-touch machine registration
- **🏷️ Intelligent inventory** management with labels and annotations  
- **☁️ Flexible configuration** using cloud-init for user management
- **🔨 Custom image creation** with additional software packages

### 🚀 Key Takeaways

- **Zero-touch deployment** - Machines self-configure and register automatically
- **Flexible customization** - Cloud-init enables any configuration scenario
- **Container-native approach** - Everything managed through Kubernetes resources
- **Immutable infrastructure** - Consistent, reliable system deployments
- **GitOps ready** - All configurations version-controlled and declarative

### 🔗 Next Steps

- Explore OS updates and rollback capabilities
- Implement GitOps workflows with Fleet
- Scale to hundreds of edge nodes
- Integrate with monitoring and observability tools

**Your infrastructure is now fully cloud-native!** ☁️✨
