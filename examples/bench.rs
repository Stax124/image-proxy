//! Throughput benchmark for image-proxy.
//!
//! Measures realistic end-to-end images per second on the current machine.
//!
//! Usage:
//!   cargo run --example bench --release
//!   cargo run --example bench --release resize-jpeg
//!   BENCH_DURATION=10 BENCH_CONCURRENCY=128 cargo run --example bench --release resize-avif
//!
//! Always use --release. Debug builds (especially AVIF/JXL) are orders of magnitude slower.
//!
//! The benchmark starts an in-process server on a random localhost port against
//! synthetic images in a temp directory. It drives concurrent load with awc.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use actix_web::{App, HttpServer, web};
use bytes::Bytes;
use tempfile::tempdir;
use tokio::time::sleep;

use image_proxy::{
    api::{image::process_image_request, metrics::metrics_handler},
    config::EncodingConfig,
    metrics::setup_metrics,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let scenario = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "passthrough".to_string());

    let duration_secs: u64 = std::env::var("BENCH_DURATION")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let concurrency: usize = std::env::var("BENCH_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);

    let enable_cache = scenario.contains("cache") || std::env::var("BENCH_ENABLE_CACHE").is_ok();

    println!("image-proxy throughput benchmark");
    println!(
        "scenario: {}  |  duration: {}s  |  concurrency: {}",
        scenario, duration_secs, concurrency
    );
    if enable_cache {
        println!("cache: ENABLED (will warm up)");
    } else {
        println!("cache: disabled (raw processing)");
    }
    if cfg!(debug_assertions) {
        eprintln!(
            "WARNING: built in debug mode — numbers will be unrealistically low (use --release)"
        );
    }
    println!("(localhost numbers; real deployments behind TLS/CDN will be lower)\n");

    // --- Prepare isolated root with synthetic images ---
    let tmp = tempdir()?;
    let root = tmp.path().to_str().unwrap().to_string();

    let jpeg_path = tmp.path().join("bench.jpg");
    let png_path = tmp.path().join("bench.png");
    write_synthetic_jpeg(&jpeg_path, 1920, 1080);
    write_synthetic_png(&png_path, 1920, 1080);

    // --- Config + metrics + optional cache ---
    // Keep other tunables at defaults (qualities, speeds, etc.)
    let config = Arc::new(EncodingConfig {
        root_path: root,
        ..EncodingConfig::default()
    });

    let (prometheus_registry, pipeline_duration, request_count) = setup_metrics();

    let hybrid_cache: Option<foyer::HybridCache<String, Bytes>> = if enable_cache {
        image_proxy::cache::setup_cache(&config, &prometheus_registry).await?
    } else {
        None
    };

    // --- Bind random port using listener so we can discover the address ---
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    let local_addr = listener.local_addr()?;
    let base_url = format!("http://{}", local_addr);

    // --- Build server (modeled directly after main.rs) ---
    let config_for_server = config.clone();
    let registry_for_server = prometheus_registry.clone();
    let pd_for_server = pipeline_duration.clone();
    let rc_for_server = request_count.clone();
    let cache_for_server = hybrid_cache.clone();

    let server = HttpServer::new(move || {
        // Fresh awc client per worker (same pattern as main)
        let http_client = awc::ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .finish();

        App::new()
            .app_data(web::Data::new(http_client))
            .app_data(web::Data::new(config_for_server.clone()))
            .app_data(web::Data::new(cache_for_server.clone()))
            .app_data(web::Data::new(registry_for_server.clone()))
            .app_data(web::Data::new(pd_for_server.clone()))
            .app_data(web::Data::new(rc_for_server.clone()))
            // Logger intentionally omitted for clean benchmark output and lower noise.
            // Production (main.rs) includes it.
            .service(metrics_handler)
            .service(process_image_request)
    })
    .listen(listener)?
    .run();

    // --- Determine target URL for the scenario (before entering LocalSet) ---
    let (target_path, query) = match scenario.as_str() {
        "passthrough" => ("bench.jpg", ""),
        "resize" | "resize-jpeg" => ("bench.jpg", "?size=400&format=jpeg"),
        "resize-webp" => ("bench.jpg", "?size=400&format=webp"),
        "resize-avif" => ("bench.jpg", "?size=400&format=avif"),
        "resize-png" => ("bench.jpg", "?size=400&format=png"),
        "dpr" | "with-dpr" => ("bench.jpg", "?size=300&dpr=2"),
        "resize-jpeg-cached" => ("bench.jpg", "?size=400&format=jpeg"),
        "resize-avif-cached" => ("bench.jpg", "?size=400&format=avif"),
        other => {
            eprintln!("Unknown scenario '{}', falling back to passthrough", other);
            ("bench.jpg", "")
        }
    };
    let target_url = format!("{}/{}{}", base_url, target_path, query);

    // Everything that touches spawn_local or holds !Send values across awaits must run
    // inside a LocalSet because we use current_thread runtime.
    let local = tokio::task::LocalSet::new();

    let counter = Arc::new(AtomicUsize::new(0));
    let start_for_load = Arc::new(std::sync::Mutex::new(Instant::now())); // for result after

    local
        .run_until(async {
            // --- Start server inside LocalSet context ---
            let server_handle = server.handle();
            let _server_task = tokio::task::spawn_local(server);

            // Give the server a moment to become ready
            sleep(Duration::from_millis(120)).await;

            // --- Optional warm-up when cache is enabled ---
            if enable_cache {
                let warm_client = awc::ClientBuilder::new()
                    .timeout(Duration::from_secs(30))
                    .finish();
                println!("Warming cache with 300 requests...");
                for _ in 0..300 {
                    if let Ok(mut resp) = warm_client.get(&target_url).send().await {
                        let _ = resp.body().limit(8 * 1024 * 1024).await;
                    }
                }
                println!("Warm-up complete.\n");
            }

            // --- Sustained concurrent load (all inside LocalSet) ---
            *start_for_load.lock().unwrap() = Instant::now();
            let duration = Duration::from_secs(duration_secs);
            let deadline = Instant::now() + duration;

            let mut handles = Vec::with_capacity(concurrency);

            for _ in 0..concurrency {
                let c = counter.clone();
                let url = target_url.clone();

                handles.push(tokio::task::spawn_local(async move {
                    // Fresh client per worker (awc::Client is !Send).
                    let cl = awc::ClientBuilder::new()
                        .timeout(Duration::from_secs(30))
                        .finish();

                    while Instant::now() < deadline {
                        match cl.get(&url).send().await {
                            Ok(mut resp) => {
                                if resp.status().is_success() {
                                    // Fully consume body for realistic accounting + connection reuse
                                    let _ = resp.body().limit(16 * 1024 * 1024).await;
                                    c.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            Err(_) => {
                                // transient error under heavy load — ignore for throughput measurement
                            }
                        }
                    }
                }));
            }

            for h in handles {
                let _ = h.await;
            }

            // Stop server while still in local context
            server_handle.stop(true).await;
        })
        .await;

    let start = *start_for_load.lock().unwrap();
    let actual_finish = Instant::now();
    let elapsed = actual_finish.duration_since(start).as_secs_f64();
    let total = counter.load(Ordering::Relaxed);
    let images_per_sec = total as f64 / elapsed;

    // --- Results ---
    println!("----------------------------------------");
    println!("Scenario: {}", scenario);
    println!("Images processed: {}", total);
    println!("Elapsed: {:.2}s", elapsed);
    println!("Throughput: {:.1} images/sec", images_per_sec);
    println!("----------------------------------------\n");

    // tempdir is dropped here automatically
    println!("Benchmark complete. (temp data cleaned up)");

    Ok(())
}

// --- Synthetic image helpers (adapted from tests/common/mod.rs) ---

fn write_synthetic_jpeg(path: &std::path::Path, w: u32, h: u32) {
    let buf = make_test_jpeg_bytes(w, h);
    std::fs::write(path, buf).expect("failed to write synthetic jpeg");
}

fn write_synthetic_png(path: &std::path::Path, w: u32, h: u32) {
    let buf = make_test_png_with_alpha_bytes(w, h);
    std::fs::write(path, buf).expect("failed to write synthetic png");
}

fn make_test_jpeg_bytes(w: u32, h: u32) -> Vec<u8> {
    let rgba = image::RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba([
            (x.wrapping_mul(37) & 0xff) as u8,
            (y.wrapping_mul(51) & 0xff) as u8,
            140,
            255,
        ])
    });
    let rgb = image::DynamicImage::ImageRgba8(rgba).into_rgb8();
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 80);
    image::ImageEncoder::write_image(encoder, rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8)
        .expect("jpeg encode");
    buf
}

fn make_test_png_with_alpha_bytes(w: u32, h: u32) -> Vec<u8> {
    let rgba = image::RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba([
            (x.wrapping_mul(29) & 0xff) as u8,
            (y.wrapping_mul(43) & 0xff) as u8,
            110,
            ((x + y) & 0xff) as u8,
        ])
    });
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        rgba.as_raw(),
        w,
        h,
        image::ExtendedColorType::Rgba8,
    )
    .expect("png encode");
    buf
}
