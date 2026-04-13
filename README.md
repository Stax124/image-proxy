# image-proxy

A lightweight HTTP image-serving and transformation proxy written in Rust.

## Features

- On-the-fly resize with preserved aspect ratio
- Format conversion: **AVIF**, **JPEG**, **PNG**, **WebP**
- Fast pass-through when no transformation is requested (no decode/re-encode)
- Hybrid in-memory and disk response cache (via [foyer](https://github.com/foyer-rs/foyer))
- Prometheus metrics endpoint (`/metrics`)
- Fallback image URL support (fetch from upstream when a file is not found locally)
- Configurable via environment variables

## API

### `GET /{filename}`

Serves the image at `{IMAGE_PROXY_ROOT_PATH}/{filename}`.

| Query Parameter    | Type     | Description                                                                                                                                                           |
| ------------------ | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `format`           | `string` | Output format: `avif`, `jpeg`, `jpg`, `png`, `webp`                                                                                                                   |
| `size`             | `u32`    | Max bounding-box dimension in pixels (aspect ratio preserved)                                                                                                         |
| `resize_algorithm` | `string` | Per-request resize algorithm override: `lanczos3`, `thumbnail`, or `auto`                                                                                             |
| `dpr`              | `f64`    | Device pixel ratio (1.0–10.0). Multiplies `size` to produce the actual output dimension (useful for high-DPI displays where 1px in CSS can be multiple device pixels) |

The `dpr` value can also be supplied via the [`Sec-CH-DPR`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Sec-CH-DPR) request header. Priority order: **query parameter > header**.

If no transformation parameter is provided, the raw file bytes are returned unchanged (no decoding).

**Examples:**

```
GET /photos/sample.jpg               # serve original
GET /photos/sample.jpg?size=400      # resize to fit 400×400 box (keeps the aspect ratio)
GET /photos/sample.jpg?format=avif   # convert to AVIF
GET /photos/sample.jpg?size=400&format=webp  # resize + convert to WebP
GET /photos/sample.jpg?size=400&resize_algorithm=lanczos3  # resize with Lanczos3
GET /photos/sample.jpg?size=400&dpr=2          # resize to 800px (400 × 2.0)
```

Alternatively, pass the DPR via the client-hint header:

```
GET /photos/sample.jpg?size=400
Sec-CH-DPR: 2.0
```

### `GET /metrics`

Exposes Prometheus metrics in the standard text format.

## Configuration

All settings are provided via environment variables.

| Variable                              | Default              | Description                                                                                                                                               |
| ------------------------------------- | -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `IMAGE_PROXY_BIND_ADDRESS`            | `0.0.0.0:8000`       | TCP address and port to listen on                                                                                                                         |
| `IMAGE_PROXY_ROOT_PATH`               | `/app/data`          | Root directory for image files                                                                                                                            |
| `IMAGE_PROXY_STRIP_PATH`              | *(unset)*            | Path prefix to strip from incoming requests (e.g. `static/image/` when behind a reverse proxy like Traefik that routes `/static/image/…` to this service) |
| `IMAGE_PROXY_FALLBACK_IMAGE_URL`      | *(unset)*            | Base URL of the fallback image to use when the requested image is not found (e.g. `https://example.com/images/`)                                          |
| `IMAGE_PROXY_FALLBACK_IMAGE_MAX_SIZE` | `5242880` (5 MB)     | Maximum allowed size for the fallback image in bytes (to prevent excessive memory usage when fetching large images from the fallback URL)                 |
| `IMAGE_PROXY_AVIF_SPEED`              | `6`                  | AVIF encoder speed (1–10, higher = faster/lower quality)                                                                                                  |
| `IMAGE_PROXY_AVIF_QUALITY`            | `85`                 | AVIF quality (0–100)                                                                                                                                      |
| `IMAGE_PROXY_JPEG_QUALITY`            | `75`                 | JPEG quality (0–100)                                                                                                                                      |
| `IMAGE_PROXY_WEBP_QUALITY`            | `80`                 | WebP quality (0–100)                                                                                                                                      |
| `IMAGE_PROXY_WEBP_EFFORT`             | `4`                  | WebP encoding effort (0–6, higher = slower/better compression)                                                                                            |
| `IMAGE_PROXY_PNG_COMPRESSION_LEVEL`   | `6`                  | PNG compression level (0–9, higher = smaller file/slower encoding)                                                                                        |
| `IMAGE_PROXY_RESIZE_ALGORITHM`        | `auto`               | Resize algorithm to use: `lanczos3`, `thumbnail`, or `auto` (can be overridden by per-request query parameter)                                            |
| `IMAGE_PROXY_ENABLE_CACHE`            | `false`              | Enable the response cache (only for transformed images)                                                                                                   |
| `IMAGE_PROXY_CACHE_MEMORY_SIZE`       | `104857600` (100 MB) | In-memory cache size in bytes                                                                                                                             |
| `IMAGE_PROXY_CACHE_MAX_ITEM_SIZE`     | `1048576` (1 MB)     | Maximum size of a single item stored in the memory cache (bytes); larger items are skipped                                                                |
| `IMAGE_PROXY_ENABLE_DISK_CACHE`       | `false`              | Enable disk-backed cache (requires `IMAGE_PROXY_ENABLE_CACHE=true`)                                                                                       |
| `IMAGE_PROXY_CACHE_DISK_SIZE`         | `536870912` (512 MB) | Pre-allocated disk cache capacity in bytes                                                                                                                |
| `IMAGE_PROXY_CACHE_DISK_PATH`         | `./cache`            | Directory for the disk cache                                                                                                                              |
| `RUST_LOG`                            | `INFO`               | Log level (`TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`)                                                                                                     |

## Running

### Docker Compose (recommended)

Create a `.env` file from the provided `.env.template` and adjust settings as needed, then run:

```sh
docker compose up --build
```

Images are served from `./data/images` and the service listens on port `8000`.

### Local build

**Requirements:** Rust toolchain, `nasm`, `libdav1d`, `pkg-config`

```sh
cargo build --release
IMAGE_PROXY_ROOT_PATH=./data/images ./target/release/image-proxy
```

## Development

Run the app with `IMAGE_PROXY_ROOT_PATH=./data/images RUST_LOG=DEBUG cargo run --release`

- `IMAGE_PROXY_ROOT_PATH` - required to be overridden for development, it is set up to `/data` by default for production use, but you can point it to any directory containing images for testing
- `RUST_LOG=DEBUG` - set this environment variable to see debug logs with detailed information about request handling and transformations, highly recommended when testing new features or troubleshooting issues
- `cargo run --release` - run the app in release mode for better performance, especially important when testing image transformations, as debug mode can be significantly slower due to lack of optimizations (looking at you, AVIF encoding)