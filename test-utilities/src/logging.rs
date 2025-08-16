#[ctor::ctor(anonymous)]
fn test_init() {
    let _ = env_logger::builder()
        .parse_env(env_logger::Env::default().default_filter_or("info"))
        .format_level(true)
        .format_source_path(true)
        .format_module_path(false)
        .format_timestamp_micros()
        .is_test(true)
        .try_init();
}
