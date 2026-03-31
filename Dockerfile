# Pin the Rust toolchain version used in the build stage.
ARG RUST_VERSION=1.92

################################################################################
# Build stage (DOI Rust image)
# This stage compiles the application.
################################################################################

FROM docker.io/library/rust:${RUST_VERSION}-alpine AS build

# All build steps happen inside /app.
WORKDIR /app

# Install build dependencies needed to compile Rust crates on Alpine
RUN apk add --no-cache clang lld git nasm dav1d-dev pkgconfig musl-dev

# Disable static linking to avoid issues with libdav1d
ENV RUSTFLAGS="-C target-feature=-crt-static"

# Build the application 
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=image,target=image \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --profile release-optimized && \
    cp ./target/release-optimized/image-proxy /bin/image-proxy

################################################################################
# Runtime stage (DOI Alpine image)
# This stage runs the already-compiled binary with minimal dependencies.
################################################################################

FROM docker.io/library/alpine:3.23 AS runtime

# Create a non-privileged user (recommended best practice)
ARG UID=1000

RUN apk add --no-cache libdav1d libgcc

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

# Drop privileges for runtime.
USER appuser

# Set working directory
WORKDIR /app

# Copy only the compiled binary from the build stage.
COPY --from=build /bin/image-proxy /bin/image-proxy

# Document the port your app listens on.
EXPOSE 8000

# Start the application.
CMD ["/bin/image-proxy"]