# image-proxy — Agent Guide

A lightweight HTTP image-serving and transformation proxy in Rust (actix-web). Serves images from disk, optionally resizing and converting formats on the fly, with a hybrid memory/disk cache and Prometheus metrics. See [README.md](README.md) and the [docs site source](docs/content/docs/).

## Build, Run, Test

Rust **edition 2024** (toolchain 1.95.0). System libraries are required — `cargo build` fails without them:

- nasm, dav1d, libhwy, libbrotli (for `image` crate with `nasm` + `avif-native`)
- libjxl (for `jpegxl-rs`/`jpegxl-sys`)

Install on Debian/Ubuntu (matches [.github/workflows/test.yml](.github/workflows/test.yml)):

```bash
sudo apt install -y libdav1d7 libdav1d-dev nasm libhwy-dev libbrotli-dev
cd data/libjxl && sudo dpkg -i libjxl_0.11.1_amd64.deb libjxl-dev_0.11.1_amd64.deb && sudo ldconfig
```

Tests are run with `cargo pretty-test` (install once with `cargo install cargo-pretty-test`). It wraps `cargo test`, accepts the same arguments, and prints a hierarchically structured tree of results with a summary at the end.

Common commands:

```bash
cargo pretty-test                        # all tests (unit + integration in tests/)
cargo pretty-test --test transformations # one integration test file
cargo pretty-test <name> -- --nocapture  # one test, show output
cargo clippy --all-targets
cargo fmt
# Run locally (ROOT_PATH must point at a dir with images):
IMAGE_PROXY_ROOT_PATH=./data/images RUST_LOG=debug cargo run --release
# Throughput benchmark (images/sec on this machine):
cargo run --example bench --release
cargo run --example bench --release resize-avif
BENCH_DURATION=5 BENCH_CONCURRENCY=128 cargo run --example bench --release resize-jpeg
```

Use `--release` for any performance testing — debug AVIF/JXL encoding is extremely slow. See [development docs](docs/content/docs/development.mdx).

## Architecture

Request flow: actix-web server ([src/main.rs](src/main.rs)) → handler [src/api/image.rs](src/api/image.rs) → path/format validation → cache lookup → pass-through OR decode/transform/encode pipeline ([src/operations/](src/operations/)) → cache store → response. `/metrics` is served by [src/api/metrics.rs](src/api/metrics.rs).

- [src/config.rs](src/config.rs) — `EncodingConfig::from_env()` reads all `IMAGE_PROXY_*` env vars. Add new tunables here. Reference: [configuration docs](docs/content/docs/configuration.mdx).
- [src/cache.rs](src/cache.rs) — hybrid memory/disk cache via `foyer`; disabled by default, returns `Option`.
- [src/operations/format.rs](src/operations/format.rs) — encoders (AVIF/JPEG/PNG/WebP/JXL); [resize.rs](src/operations/resize.rs) — `ResizeAlgorithm`; [pipeline.rs](src/operations/pipeline.rs) — orchestration.
- [src/utils/path.rs](src/utils/path.rs) — path sanitization (directory-traversal prevention); [mime.rs](src/utils/mime.rs); [encoding.rs](src/utils/encoding.rs).
- [src/preferred_formats.rs](src/preferred_formats.rs) — `Accept`-header format negotiation.

## Conventions & Pitfalls

- **Vendored `image` crate**: `[patch.crates-io]` in [Cargo.toml](Cargo.toml) points `image` at the local fork in [image/](image/). Make encoder/codec changes in `src/operations/` first; only touch `image/` when backporting a codec feature is unavoidable (see [image/CHANGES.md](image/CHANGES.md)).
- **CPU work goes in `web::block()`**: never decode/resize/encode directly in the async handler — it stalls the actix executor.
- **Pass-through optimization**: when no transformation is requested and the on-disk format already matches, original bytes are streamed without decode/re-encode. Preserve this fast path.
- **Non-processable formats** (SVG, ICO, GIF, BMP, TIFF, JXL) are streamed raw and never decoded. Output re-encoding is only AVIF/JPEG/PNG/WebP/JXL.
- **Cache keys must include every transform param** (format, size, resize algorithm, dpr) to avoid collisions.
- **Errors**: use `anyhow::Result`; do not introduce custom error types. Map to HTTP status at the handler boundary.
- **Observability**: keep `#[tracing::instrument]` on handlers and update Prometheus counters in [src/metrics.rs](src/metrics.rs) when adding code paths.

## Tests

Integration tests live in [tests/](tests/) (one file per concern: `basic_serving`, `transformations`, `security`, `headers`, `metrics`, `allowed_formats`, `auto_format`, `non_processable_formats`). Shared helpers are in [tests/common/mod.rs](tests/common/mod.rs) — use `test_config()`, `write_test_jpeg()` / `write_test_png_with_alpha()`, and `build_app_data()` to build an in-process app against a tempdir. Add security/path-traversal coverage in [tests/security.rs](tests/security.rs).
