use std::net::{Ipv4Addr, SocketAddr};

use anyhow::{Result, bail};
use axum::Router;
use axum::http::StatusCode;
use axum::routing::{any, get_service};
use camino::Utf8Path;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tower_http::services::{ServeDir, ServeFile};

use crate::screenshotter::args::PAGE_PATH;
use crate::screenshotter::logger::Logger;
use crate::screenshotter::webdriver::pick_free_port;

pub async fn start_static_server(
    logger: &Logger,
    root: &Utf8Path,
    requested_port: u16,
) -> Result<(SocketAddr, oneshot::Sender<()>, tokio::task::JoinHandle<()>)> {
    let katex_dir = root.join("KaTeX");
    let katex_dist_dir = katex_dir.join("dist");
    let katex_css = katex_dist_dir.join("katex.min.css");
    let katex_min_js = katex_dist_dir.join("katex.min.js");
    let katex_fonts = katex_dist_dir.join("fonts");
    let katex_additional_fonts = katex_dir.join("test/screenshotter/fonts");
    let khan_image = katex_dir.join("website/static/img/khan-academy.png");
    let wasm_pkg_dir = root.join("crates/wasm-binding/pkg");
    let assets_dir = root.join("xtask/assets");

    let test_page = assets_dir.join("screenshot.html");
    if !test_page.exists() {
        bail!(
            "screenshotter test page missing at {}. Ensure the repository is up to date.",
            test_page
        );
    }

    if !katex_css.exists() || !katex_min_js.exists() {
        bail!(
            "KaTeX CSS or JS not found at {}. Re-run with --build auto/always after building the KaTeX submodule.",
            katex_css
        );
    }

    if !katex_fonts.exists() {
        bail!(
            "KaTeX fonts not found at {}. Ensure the KaTeX dist assets have been built.",
            katex_fonts
        );
    }

    if !khan_image.exists() {
        logger.warn(format!(
            "KaTeX website image missing at {}. Includegraphics cases may fail.",
            khan_image
        ));
    }

    if !wasm_pkg_dir.join("katex.js").exists() {
        bail!(
            "wasm-pack artifacts not found at {}. Run with --build auto/always to rebuild them.",
            wasm_pkg_dir.join("katex.js")
        );
    }

    let logger_clone = logger.clone();

    let router = Router::new()
        .route_service(
            PAGE_PATH,
            get_service(ServeFile::new(test_page.as_std_path().to_path_buf())),
        )
        .route_service(
            "/katex.min.css",
            get_service(ServeFile::new(katex_css.as_std_path().to_path_buf())),
        )
        .route_service(
            "/katex.min.js",
            get_service(ServeFile::new(katex_min_js.as_std_path().to_path_buf())),
        )
        .route_service(
            "/website/static/img/khan-academy.png",
            get_service(ServeFile::new(khan_image.as_std_path().to_path_buf())),
        )
        .nest_service(
            "/pkg",
            get_service(ServeDir::new(wasm_pkg_dir.as_std_path().to_path_buf())),
        )
        .nest_service(
            "/fonts",
            get_service(ServeDir::new(katex_fonts.as_std_path().to_path_buf())),
        )
        .nest_service(
            "/KaTeX/test/screenshotter/fonts/",
            get_service(ServeDir::new(
                katex_additional_fonts.as_std_path().to_path_buf(),
            )),
        )
        .fallback_service(any(move |req: axum::http::Request<axum::body::Body>| {
            let logger = logger_clone.clone();
            async move {
                logger.warn(format!("Static asset not found: {}", req.uri().path()));
                (StatusCode::NOT_FOUND, "Not Found")
            }
        }));

    let port = if requested_port == 0 {
        pick_free_port()?
    } else {
        requested_port
    };

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
        {
            eprintln!("Static server error: {err}");
        }
    });

    Ok((addr, shutdown_tx, handle))
}
