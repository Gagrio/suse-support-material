# Use the latest SUSE Linux Enterprise Server (SLES) base image
FROM registry.suse.com/bci/bci-base:15.6

# Install required OS packages
RUN zypper --non-interactive ref && \
    zypper --non-interactive install \
    python3-PyYAML \
    python3-kubernetes

# Set the working directory inside the container
WORKDIR /app

# Copy the Python script into the container
COPY ./nessie.py /app/nessie.py

# Run the script when the container starts
CMD ["python3", "/app/nessie.py"]
