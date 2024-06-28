ARG VARIANT="bookworm"
FROM mcr.microsoft.com/vscode/devcontainers/rust:1.0.8-${VARIANT}
# See here for the contents of the image:
# https://github.com/devcontainers/images/blob/main/src/rust/history/1.0.8.md#variant-bookworm

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
    && cmake -G "Unix Makefiles" && make \
    && sudo cp /flatbuffers/flatc /usr/bin/flatc && chmod +x /usr/bin/flatc

# Install Rust version
RUN rustup install 1.70 && rustup default 1.70

# Install Rust dependecies
RUN rustup component add \
    clippy \
    rustfmt \
    rust-src

# Install targets for cross compiling
RUN rustup target add arm-unknown-linux-gnueabihf \
    && rustup target add aarch64-unknown-linux-gnu \
    && rustup target add x86_64-unknown-linux-gnu

RUN su -c "cargo install cargo-audit@^0.18 --locked" vscode

WORKDIR ${HOME}
