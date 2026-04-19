################################################################################
# Chef stage (pre-built cargo-chef image)
################################################################################

FROM lukemathwalker/cargo-chef:latest-rust-1.95.0-alpine AS chef
WORKDIR /app

# Install build dependencies needed to compile Rust crates on Alpine
RUN apk add --no-cache clang lld git nasm dav1d-dev pkgconfig musl-dev libjxl-dev

# Disable static linking to avoid issues with libdav1d
ENV RUSTFLAGS="-C target-feature=-crt-static"

################################################################################
# Planner stage
# Analyze the project and produce a recipe file.
################################################################################

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

################################################################################
# Builder stage
# Cache dependencies and compile the application.
################################################################################

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Copy the local path dependency needed by [patch.crates-io]
COPY image/ image/

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --locked --release && \
    cp ./target/release/image-proxy /bin/image-proxy

################################################################################
# Runtime stage (DOI Alpine image)
# This stage runs the already-compiled binary with minimal dependencies.
################################################################################

FROM docker.io/library/alpine:3.23 AS runtime

# Create a non-privileged user (recommended best practice)
ARG UID=1000

RUN apk add --no-cache libdav1d libgcc libjxl

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
COPY --from=builder /bin/image-proxy /bin/image-proxy

# Document the port your app listens on.
EXPOSE 8000

# Start the application.
CMD ["/bin/image-proxy"]