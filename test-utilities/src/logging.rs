#[ctor::ctor(anonymous)]
fn test_init() {
    let _ = env_logger::builder()
        .parse_env(env_logger::Env::default().default_filter_or("info"))
        .format_level(true)
        .format_source_path(true)
        .format_module_path(false)
        .format_timestamp_micros()
        // Panic info and stacktrace will be written to stderr
        // Log to stdout to avoid mixing with panic info
        .target(env_logger::Target::Stdout)
        .write_style(env_logger::WriteStyle::Always)
        .is_test(true)
        .try_init();
}
