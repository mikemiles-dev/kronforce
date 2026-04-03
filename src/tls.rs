use std::io::BufReader;
use std::sync::Arc;

use rustls::ServerConfig;
use rustls::pki_types::CertificateDer;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tower_service::Service;

/// Loads TLS certificate chain and private key from PEM files.
pub fn load_tls_config(cert_path: &str, key_path: &str) -> Result<ServerConfig, String> {
    let cert_file = std::fs::File::open(cert_path)
        .map_err(|e| format!("cannot open TLS cert {cert_path}: {e}"))?;
    let key_file = std::fs::File::open(key_path)
        .map_err(|e| format!("cannot open TLS key {key_path}: {e}"))?;

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut BufReader::new(cert_file))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("invalid TLS cert: {e}"))?;

    if certs.is_empty() {
        return Err("TLS cert file contains no certificates".into());
    }

    let key = rustls_pemfile::private_key(&mut BufReader::new(key_file))
        .map_err(|e| format!("invalid TLS key: {e}"))?
        .ok_or_else(|| "TLS key file contains no private key".to_string())?;

    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("TLS config error: {e}"))
}

/// Serves an axum app over TLS with graceful shutdown support.
pub async fn serve_tls(
    listener: TcpListener,
    app: axum::Router,
    tls_config: ServerConfig,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    tokio::spawn(async move {
        shutdown.await;
        let _ = shutdown_tx.send(true);
    });

    loop {
        let mut rx = shutdown_rx.clone();
        let accept = tokio::select! {
            result = listener.accept() => result,
            _ = async { while !*rx.borrow_and_update() { rx.changed().await.ok(); } } => break,
        };

        let (tcp_stream, _remote_addr) = match accept {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("TCP accept error: {e}");
                continue;
            }
        };

        let acceptor = acceptor.clone();
        let app = app.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(tcp_stream).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::debug!("TLS handshake error: {e}");
                    return;
                }
            };

            let io = hyper_util::rt::TokioIo::new(tls_stream);
            let service =
                hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                    let mut svc = app.clone().into_service();
                    async move { svc.call(req).await }
                });

            let _ =
                hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .serve_connection(io, service)
                    .await;
        });
    }

    Ok(())
}
