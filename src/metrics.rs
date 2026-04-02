use prometheus::Registry;

pub fn setup_metrics() -> Registry {
    let prometheus_registry = Registry::new();

    return prometheus_registry;
}
