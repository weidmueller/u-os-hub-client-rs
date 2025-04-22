# For library crates, we always use the most recent stable version of rust
ARG VARIANT="bookworm"
FROM mcr.microsoft.com/vscode/devcontainers/rust:dev-${VARIANT}
# See here for the contents of the image:
# https://github.com/devcontainers/images/blob/main/src/rust/history/dev.md#variant-bookworm

# Rust Version that is used by uOS
ENV U_OS_RUST_VERSION="1.75.0"

# Adjust the following args to the username and groupname of the base image
ARG BASE_IMAGE_USERNAME="vscode"
ARG BASE_IMAGE_GROUP="${BASE_IMAGE_USERNAME}"
ARG HOME=/home/${BASE_IMAGE_USERNAME}

# Overwriteable from outside via build-args
ARG USER_UID=1000
ARG USER_GID=1000

# Change uid and gid of base image user
RUN groupmod -g ${USER_GID} ${BASE_IMAGE_GROUP}
RUN usermod -u ${USER_UID} -g ${USER_GID} ${BASE_IMAGE_USERNAME}
RUN chown ${USER_UID}:${USER_GID} ${HOME} -R

RUN apt-get update \
    && apt-get install -y \
    cmake \
    binutils-arm-linux-gnueabihf \
    binutils-aarch64-linux-gnu \
    ca-certificates \
    curl \
    gcc \
    gcc-arm-linux-gnueabihf \
    gcc-aarch64-linux-gnu \
    libc-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install flatc
RUN git clone https://github.com/google/flatbuffers.git && cd flatbuffers \
    && git checkout v23.5.26 \
    && cmake -G "Unix Makefiles" && make -j$(nproc) \
    && sudo cp /flatbuffers/flatc /usr/bin/flatc && chmod +x /usr/bin/flatc

# Install nightly rust toolchain, needed for cargo -Z flags
RUN rustup toolchain install nightly

# Install rust version that uOS uses
# This is used during CI to ensure that the code also compiles with the same version as in production
RUN rustup toolchain install ${U_OS_RUST_VERSION}
RUN rustup component add --toolchain ${U_OS_RUST_VERSION} clippy

# Install targets for cross compiling
RUN rustup target add armv7-unknown-linux-gnueabihf aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu
RUN rustup target add armv7-unknown-linux-gnueabihf aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu --toolchain ${U_OS_RUST_VERSION}

# Download and install cargo-audit 
RUN <<-EOT
    VER="0.21.2"
    DIR="/usr/local/cargo/bin"
    mkdir -p $DIR
    curl -L -o $DIR/archive.tar.gz https://github.com/rustsec/rustsec/releases/download/cargo-audit%2Fv${VER}/cargo-audit-x86_64-unknown-linux-musl-v${VER}.tgz
    tar -xzvf $DIR/archive.tar.gz --strip-components=1 -C $DIR
    rm $DIR/archive.tar.gz
EOT

WORKDIR ${HOME}

# Install and run nats server (needed for unit tests and jenkins pipeline)
# We are currently running nats directly in the dev container 
# because that makes the Jenkins buildpipeline much easier
# as Jenkins currently does not support docker compose.
RUN curl -sf https://binaries.nats.dev/nats-io/nats-server/v2@v2.10.20 | sh

# Install nats CLI client for easy testing
RUN curl -sf https://binaries.nats.dev/nats-io/natscli/nats@latest | sh

# Adjust path so installed executables are available
ENV PATH=$PATH:/home/vscode

# Add localhost nats server to contexts
RUN nats context add localhost --description "Localhost"

ENTRYPOINT ["/usr/bin/bash", "-c", "nats-server -a 127.0.0.1 -p 4222 > $HOME/nats-log.txt 2>&1"]

