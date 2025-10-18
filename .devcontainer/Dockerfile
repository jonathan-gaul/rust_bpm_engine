FROM rust:1.76-bullseye

# Install common dependencies for building native crates and DB clients
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    postgresql-client \
    libreadline8 \
    locales \
    ca-certificates \
    curl \
 && rm -rf /var/lib/apt/lists/*

# Generate en_US.UTF-8 locale so interactive programs (perl/psql) don't warn
RUN sed -i 's/^# *\(en_US.UTF-8 UTF-8\)/\1/' /etc/locale.gen \
 && locale-gen en_US.UTF-8 || true

# Create a non-root user for VS Code (use bash as login shell so terminals get line-editing)
RUN useradd -m -s /bin/bash vscode && mkdir -p /workspace && chown -R vscode:vscode /workspace

USER vscode
WORKDIR /workspace

ENV CARGO_HOME=/home/vscode/.cargo
ENV RUSTUP_HOME=/home/vscode/.rustup
ENV SHELL=/bin/bash
ENV LANG=en_US.UTF-8

# Keep the container running so the VS Code remote extension can attach
CMD ["sleep", "infinity"]
