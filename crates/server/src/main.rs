use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // RUST_LOG=debug for more detail.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = server::Config::from_env();
    let addr = std::env::var("ACW_LISTEN").unwrap_or_else(|_| "127.0.0.1:3000".into());
    if !config.wizard_dir.join("index.html").exists() {
        tracing::warn!(dir = %config.wizard_dir.display(), "wizard not built; /deploy answers 503 until `trunk build` runs in crates/wizard");
    }
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("cannot bind {addr}: {e}"));
    tracing::info!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        server::app(config).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await
    .unwrap();
}

/// Ctrl-C, or SIGTERM from `docker stop` / Kubernetes.
async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
}
