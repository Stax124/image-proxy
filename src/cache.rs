use foyer::{
    BlockEngineConfig, DeviceBuilder, EvictionConfig, FsDeviceBuilder, HybridCacheBuilder,
    LruConfig, PsyncIoEngineConfig,
};
use mixtrics::registry::prometheus::PrometheusMetricsRegistry;

pub async fn setup_cache(
    config: &crate::config::EncodingConfig,
    prometheus_registry: &prometheus::Registry,
) -> anyhow::Result<Option<foyer::HybridCache<String, Vec<u8>>>> {
    if !config.enable_cache {
        return Ok(None);
    }

    if config.enable_disk_cache {
        let dev = FsDeviceBuilder::new(&config.cache_disk_path)
            .with_capacity(config.cache_disk_size)
            .build()?;

        let cache_memory_max_item_size = config.cache_memory_max_item_size;
        let hybrid_cache = HybridCacheBuilder::new()
            .with_metrics_registry(Box::new(PrometheusMetricsRegistry::new(
                prometheus_registry.clone(),
            )))
            .memory(config.cache_memory_size)
            .with_eviction_config(EvictionConfig::Lru(LruConfig {
                high_priority_pool_ratio: 0.8,
            }))
            .with_weighter(|key: &String, value: &Vec<u8>| key.len() + value.len())
            .with_filter(move |_, value: &Vec<u8>| value.len() <= cache_memory_max_item_size)
            .storage()
            .with_io_engine_config(PsyncIoEngineConfig::default())
            .with_engine_config(BlockEngineConfig::new(dev))
            .build()
            .await?;

        Ok(Some(hybrid_cache))
    } else {
        let cache_memory_max_item_size = config.cache_memory_max_item_size;
        let hybrid_cache = HybridCacheBuilder::new()
            .with_metrics_registry(Box::new(PrometheusMetricsRegistry::new(
                prometheus_registry.clone(),
            )))
            .memory(config.cache_memory_size)
            .with_eviction_config(EvictionConfig::Lru(LruConfig {
                high_priority_pool_ratio: 0.8,
            }))
            .with_weighter(|key: &String, value: &Vec<u8>| key.len() + value.len())
            .with_filter(move |_, value: &Vec<u8>| value.len() <= cache_memory_max_item_size)
            .storage()
            .build()
            .await?;

        Ok(Some(hybrid_cache))
    }
}
