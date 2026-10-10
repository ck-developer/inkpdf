//! `inkpdf` binary: `inkpdf [serve]` starts the service, `inkpdf healthcheck` queries
//! `/health` (used by the Docker `HEALTHCHECK`, since the image has neither a shell nor curl).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::ExitCode;
use std::time::Duration;

use inkpdf::config::{Config, LogFormat};
use inkpdf::{AppState, build_app, registry};
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        None | Some("serve") => serve(),
        Some("healthcheck") => healthcheck(),
        Some(other) => {
            eprintln!("unknown command `{other}`; usage: inkpdf [serve|healthcheck]");
            ExitCode::from(2)
        }
    }
}

fn serve() -> ExitCode {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("configuration error: {e}");
            return ExitCode::from(2);
        }
    };
    init_tracing(config.log_format);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    match runtime.block_on(run(config)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!(error = %e, "server error");
            ExitCode::FAILURE
        }
    }
}

async fn run(config: Config) -> std::io::Result<()> {
    let listen = config.listen;
    let state = AppState::new(config);

    let started = std::time::Instant::now();
    let registry = state.registry.clone();
    tokio::task::spawn_blocking(move || registry.scan_all())
        .await
        .expect("initial scan task");
    state.registry.set_ready();
    tracing::info!(
        event = "registry.ready",
        templates = state.registry.list().len(),
        valid = state.registry.valid_count(),
        durationMs = started.elapsed().as_millis() as u64,
    );

    let _watcher = registry::watcher::spawn(state.registry.clone(), state.config.rescan_interval);

    let listener = tokio::net::TcpListener::bind(listen).await?;
    tracing::info!(event = "server.listening", address = %listen);
    axum::serve(listener, build_app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!(event = "server.shutdown");
}

fn init_tracing(format: LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match format {
        LogFormat::Json => builder.json().flatten_event(true).init(),
        LogFormat::Pretty => builder.pretty().init(),
    }
}

/// Minimal HTTP client (no client dependency): `GET /health` on the configured port.
fn healthcheck() -> ExitCode {
    let port = Config::from_env().map(|c| c.listen.port()).unwrap_or(3000);
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let timeout = Duration::from_secs(3);

    let result = (|| -> std::io::Result<bool> {
        let mut stream = TcpStream::connect_timeout(&address, timeout)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        stream
            .write_all(b"GET /health HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
        let mut response = Vec::new();
        stream.take(4096).read_to_end(&mut response)?;
        let status_line = response.split(|b| *b == b'\n').next().unwrap_or_default();
        Ok(status_line.starts_with(b"HTTP/1.") && status_line.get(9..12) == Some(b"200"))
    })();

    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!("healthcheck: unexpected response from {address}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("healthcheck: {address}: {e}");
            ExitCode::FAILURE
        }
    }
}
