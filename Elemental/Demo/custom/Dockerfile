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

# Update os-release with proper version information (SAFER VERSION)
RUN echo "IMAGE_REPO=\"${IMAGE_REPO}\"" >> /etc/os-release && \
    echo "IMAGE_TAG=\"${IMAGE_TAG}\"" >> /etc/os-release && \
    echo "IMAGE=\"${IMAGE_REPO}:${IMAGE_TAG}\"" >> /etc/os-release

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
