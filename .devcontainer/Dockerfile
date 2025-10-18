FROM rust:1.76-bullseye

# Install common dependencies for building native crates and DB clients
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    postgresql-client \
    ca-certificates \
    curl \
 && rm -rf /var/lib/apt/lists/*

# Create a non-root user for VS Code
RUN useradd -m vscode && mkdir -p /workspace && chown -R vscode:vscode /workspace

USER vscode
WORKDIR /workspace

ENV CARGO_HOME=/home/vscode/.cargo
ENV RUSTUP_HOME=/home/vscode/.rustup

# Keep the container running so the VS Code remote extension can attach
CMD ["sleep", "infinity"]
