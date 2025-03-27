# Use the latest SUSE Linux Enterprise Server (SLES) base image
FROM registry.suse.com/bci/bci-base:15.6

# Install pip
RUN zypper --non-interactive ref && \
    zypper --non-interactive install \
    python312 \
    python312-pip \
    helm \
    systemd \
    util-linux \
    kubernetes1.28-client \
    podman

# Use pip to install kubernetes client
RUN python3.12 -m pip install pyyaml kubernetes

# Set the working directory inside the container
WORKDIR /app

# Copy the Python script into the container
COPY ./nessie.py /app/nessie.py

# Run the script when the container starts
CMD ["python3.12", "/app/nessie.py"]
