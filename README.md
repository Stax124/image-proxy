# image-proxy

A lightweight HTTP image-serving and transformation proxy written in Rust.

## Features

- On-the-fly resize with preserved aspect ratio
- Format conversion: **AVIF**, **JPEG**, **PNG**, **WebP**
- Fast pass-through when no transformation is requested (no decode/re-encode)
- Directory traversal protection
- Non-privileged Docker runtime
- Configurable via environment variables

## API

### `GET /{filename}`

Serves the image at `{IMAGE_PROXY_ROOT_PATH}/{filename}`.

| Query Parameter | Type     | Description                                                   |
| --------------- | -------- | ------------------------------------------------------------- |
| `format`        | `string` | Output format: `avif`, `jpeg`, `jpg`, `png`, `webp`           |
| `size`          | `u32`    | Max bounding-box dimension in pixels (aspect ratio preserved) |

If neither parameter is provided, the raw file bytes are returned unchanged (no decoding).

**Examples:**

```
GET /photos/sample.jpg               # serve original
GET /photos/sample.jpg?size=400      # resize to fit 400×400 box (keeps the aspect ratio)
GET /photos/sample.jpg?format=avif   # convert to AVIF
GET /photos/sample.jpg?size=400&format=webp  # resize + convert to WebP
```

## Configuration

All settings are provided via environment variables.

| Variable                            | Default        | Description                                                                                                                                               |
| ----------------------------------- | -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `IMAGE_PROXY_BIND_ADDRESS`          | `0.0.0.0:8000` | TCP address and port to listen on                                                                                                                         |
| `IMAGE_PROXY_ROOT_PATH`             | `/app/data`    | Root directory for image files                                                                                                                            |
| `IMAGE_PROXY_STRIP_PATH`            | *(unset)*      | Path prefix to strip from incoming requests (e.g. `static/image/` when behind a reverse proxy like Traefik that routes `/static/image/…` to this service) |
| `IMAGE_PROXY_FALLBACK_IMAGE_URL`    | *(unset)*      | URL of the fallback image to use when the requested image is not found (e.g. `https://example.com/images/`)                                               |
| `IMAGE_PROXY_AVIF_SPEED`            | `7`            | AVIF encoder speed (1–10, higher = faster/lower quality)                                                                                                  |
| `IMAGE_PROXY_AVIF_QUALITY`          | `75`           | AVIF quality (0–100)                                                                                                                                      |
| `IMAGE_PROXY_JPEG_QUALITY`          | `75`           | JPEG quality (0–100)                                                                                                                                      |
| `IMAGE_PROXY_WEBP_QUALITY`          | `75.0`         | WebP quality (0.0–100.0)                                                                                                                                  |
| `IMAGE_PROXY_PNG_COMPRESSION_LEVEL` | `6`            | PNG compression level (0–9, higher = smaller file/slower encoding)                                                                                        |
| `IMAGE_PROXY_USE_FASTER_RESIZE`     | `true`         | Use faster scaling algorithm instead of Lanczos3, but may introduce aliasing-like artifacts                                                               |
| `RUST_LOG`                          | `INFO`         | Log level (`TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`)                                                                                                     |

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