# image-proxy

A lightweight HTTP image-serving and transformation proxy written in Rust.

## Features

- On-the-fly resize with preserved aspect ratio
- Format conversion: **AVIF**, **JPEG**, **PNG**, **WebP**, **JPEG XL (output only)**
- Fast pass-through when no transformation is requested (no decode/re-encode)
- Hybrid in-memory and disk response cache (via [foyer](https://github.com/foyer-rs/foyer))
- Prometheus metrics endpoint (`/metrics`)
- Fallback image URL support (fetch from upstream when a file is not found locally)
- Configurable via environment variables
- Automatic `Vary: Sec-CH-DPR` header for proper CDN caching with device pixel ratio
- Configurable `Cache-Control` header for optimal CDN and browser caching

## Usage

```sh
docker run -p 8000:8000 -v /path/to/images:/app/data ghcr.io/stax124/image-proxy:latest
```

```
GET /photos/sample.jpg               # serve original
GET /photos/sample.jpg?size=400      # resize to fit 400×400 box (keeps the aspect ratio)
GET /photos/sample.jpg?format=avif   # convert to AVIF
GET /photos/sample.jpg?size=400&format=webp  # resize + convert to WebP
GET /photos/sample.jpg?format=jxl            # convert to JPEG XL
GET /photos/sample.jpg?size=400&resize_algorithm=lanczos3  # resize with Lanczos3
GET /photos/sample.jpg?size=400&dpr=2          # resize to 800px (400 × 2.0)
```

For more configuration options and installation methods, please refer to the [documentation](https://stax124.github.io/image-proxy/).

## Documentation

Detailed documentation is available on this address [https://stax124.github.io/image-proxy/](https://stax124.github.io/image-proxy/).

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.