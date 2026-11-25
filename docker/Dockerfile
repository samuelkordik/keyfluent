# keyfluent dev toolchain: Rust (rustup, Ubuntu 24.04) + egui/eframe GUI build deps + Windows cross target.
FROM ubuntu:24.04

ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=1000

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/opt/rust/rustup \
    CARGO_HOME=/opt/rust/cargo \
    PATH=/opt/rust/cargo/bin:$PATH

# System deps: build tools, egui/eframe Linux build deps, mingw + nsis for Windows cross/installer, shellcheck for scripts.
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential pkg-config cmake clang git curl ca-certificates sudo less nano \
      libssl-dev libfontconfig1-dev libfreetype6-dev \
      libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev \
      libgl1-mesa-dev libegl1-mesa-dev libgtk-3-dev \
      mesa-vulkan-drivers \
      mingw-w64 nsis \
      shellcheck \
    && rm -rf /var/lib/apt/lists/*

# Rust: stable toolchain, components for IDEs, Windows GNU cross target.
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable \
    && rustup component add rustfmt clippy rust-src rust-analyzer \
    && rustup target add x86_64-pc-windows-gnu \
    && cargo install --locked cargo-deny \
    && rm -rf /opt/rust/cargo/registry/* \
    && chmod -R a+rwX /opt/rust

# Non-root user matching the host uid so bind-mounted files stay owned by seldon.
# (ubuntu:24.04 ships an 'ubuntu' user at uid 1000; rename it.)
RUN (getent group ${USER_GID} || groupadd -g ${USER_GID} ${USERNAME}) \
    && (id -u ${USER_UID} >/dev/null 2>&1 && usermod -l ${USERNAME} -d /home/${USERNAME} -m $(id -nu ${USER_UID}) || useradd -m -u ${USER_UID} -g ${USER_GID} -s /bin/bash ${USERNAME}) \
    && echo "${USERNAME} ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/${USERNAME}

# Pre-create volume mount points owned by the dev user (named volumes inherit this ownership).
RUN mkdir -p /opt/rust/cargo/registry \
    && chown -R ${USER_UID}:${USER_GID} /opt/rust/cargo/registry

USER ${USERNAME}
WORKDIR /workspace
CMD ["bash"]
