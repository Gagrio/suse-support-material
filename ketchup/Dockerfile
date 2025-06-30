# =============================================================================
# 🍅 Ketchup - Kubernetes Config Collector
# Multi-stage build using SUSE Base Container Images (BCI)
# =============================================================================

# Build stage
FROM registry.suse.com/bci/rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM registry.suse.com/bci/bci-base:latest
RUN zypper --non-interactive install ca-certificates && zypper clean --all
COPY --from=builder /app/target/release/ketchup /usr/local/bin/ketchup
ENTRYPOINT ["ketchup"]
CMD ["--help"]