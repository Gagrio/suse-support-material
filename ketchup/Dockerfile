# =============================================================================
# 🍅 Ketchup - Kubernetes Config Collector
# Multi-stage build using SUSE Base Container Images (BCI)
# =============================================================================

# Build stage - Use SUSE BCI Rust image for compilation
FROM registry.suse.com/bci/rust:1.81 as builder

# Set working directory
WORKDIR /app

# Copy source code
COPY . ./

# Build the application in release mode
RUN cargo build --release

# Runtime stage - Use SUSE BCI minimal image for smaller final image
FROM registry.suse.com/bci/bci-minimal:latest

# Install ca-certificates for HTTPS connections to Kubernetes API
RUN microdnf install -y ca-certificates && \
    microdnf clean all

# Create a non-root user for security
RUN useradd -r -s /bin/false -U ketchup

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/ketchup /usr/local/bin/ketchup

# Ensure binary is executable
RUN chmod +x /usr/local/bin/ketchup

# Switch to non-root user
USER ketchup

# Set working directory for runtime
WORKDIR /tmp

# Add metadata labels
LABEL org.opencontainers.image.title="Ketchup - Kubernetes Config Collector"
LABEL org.opencontainers.image.description="Collect Kubernetes cluster configuration for backup and analysis"
LABEL org.opencontainers.image.vendor="SUSE"
LABEL org.opencontainers.image.source="https://github.com/Gagrio/suse-support-material/tree/master/ketchup"
LABEL org.opencontainers.image.licenses="Apache-2.0"
LABEL org.opencontainers.image.base.name="registry.suse.com/bci/bci-minimal:latest"

# Set default entrypoint
ENTRYPOINT ["ketchup"]

# Default help command
CMD ["--help"]