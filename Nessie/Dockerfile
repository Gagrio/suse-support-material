# Use the latest SUSE Linux Enterprise Server (SLES) base image
#FROM registry.suse.com/sles/sles15:latest
FROM registry.suse.com/bci/bci-base:15.6

# Install Python and other dependencies (Python 3, pip, etc.)
RUN zypper refresh && \
    zypper install -y python3 python3-pip && \
    zypper clean

# Set the working directory inside the container
WORKDIR /app

# Copy the Python script into the container
COPY ./nessie.py /app/nessie.py

# Optionally, copy the requirements.txt if you have any additional dependencies
COPY requirements.txt /app/requirements.txt
RUN pip install --no-cache-dir -r /app/requirements.txt

# Run the script when the container starts
CMD ["python3", "/app/nessie.py"]
