mod counter;

use std::{env, str::FromStr, sync::Arc, thread, time::Duration};

use anyhow::Result;
use axum::{routing::get, Router};
use counter::Counter;
use tokio::sync::{mpsc::unbounded_channel, Mutex};
use tower_http::services::ServeDir;

#[derive(serde::Serialize)]
pub struct CounterData {
    pub count: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    tracing_subscriber::fmt::init();
    let port = env::var("COWCOUNTER_PORT").unwrap_or_else(|_| "7788".into());
    let name_re = env::var("COWCOUNTER_NAME_RE").unwrap_or_else(|_| "^RelicCardinal.exe$".into());
    let savefile = env::var("COWCOUNTER_SAVEFILE").unwrap_or_else(|_| "counter.txt".into());
    let backup_interval = env::var("COWCOUNTER_BACKUP_INTERVAL").unwrap_or_else(|_| "1m".into());
    let backup_interval: Duration = humantime::Duration::from_str(&backup_interval)
        .expect("bad format for backup interval")
        .into();

    let counter = Arc::new(Counter::new(&name_re, &savefile)?);
    if let Err(e) = counter.load().await {
        tracing::warn!(err = ?e, "unable to load previous count");
    }

    {
        let counter = counter.clone();
        tokio::spawn(async move {
            tokio::time::sleep(backup_interval).await;
            if let Err(e) = counter.save().await {
                tracing::error!(err = ?e, "failed to save counter")
            }
        });
    }

    let (schan, rchan) = unbounded_channel();
    let _listener = thread::spawn(move || counter::listen(schan));

    {
        let counter = counter.clone();
        counter.do_count(Arc::new(Mutex::new(rchan))).await;
    }

    let app = Router::new()
        .route(
            "/count",
            get(|| async move {
                axum::Json(CounterData {
                    count: counter.get_count().await,
                })
            }),
        )
        .fallback_service(ServeDir::new("assets"));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
