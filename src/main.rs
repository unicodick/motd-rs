mod config;
mod protocol;
mod response;
mod server;
mod stats;

use std::{fmt, io, process::ExitCode};

use chrono::Utc;
use tokio::net::TcpListener;

use config::Config;
use response::Responses;

const CONFIG: &str = "config.json";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}

async fn run() -> Result<(), ()> {
    let config = Config::load(CONFIG).map_err(|err| {
        let reason = match err.kind() {
            io::ErrorKind::InvalidData => "invalid_value",
            io::ErrorKind::NotFound => "file_not_found",
            _ => "read_failed",
        };
        log(
            "config_load_failed",
            format_args!("reason={reason} file={CONFIG} detail={:?}", err.to_string()),
        );
    })?;
    let addr = config.listen;
    let icon = response::icon(&config.icon).map_err(|err| {
        log(
            "config_load_failed",
            format_args!(
                "reason=icon_read_failed file={CONFIG} field=icon path={:?} detail={:?}",
                config.icon,
                err.to_string()
            ),
        );
    })?;
    let responses = Responses::new(config, icon).map_err(|err| {
        log(
            "config_load_failed",
            format_args!(
                "reason=invalid_response file={CONFIG} detail={:?}",
                err.to_string()
            ),
        );
    })?;
    let listener = TcpListener::bind(addr).await.map_err(|err| {
        log(
            "start_failed",
            format_args!(
                "reason=bind_failed address={addr} detail={:?}",
                err.to_string()
            ),
        );
    })?;

    let stats = stats::Stats::open(stats::PATH).map_err(|err| {
        log(
            "stats_load_failed",
            format_args!(
                "reason=open_failed file={} detail={:?}",
                stats::PATH,
                err.to_string()
            ),
        );
    })?;

    let addr = listener.local_addr().unwrap_or(addr);
    log("started", format_args!("address={addr}"));
    server::run(listener, responses, stats)
        .await
        .map_err(|err| {
            log(
                "listener_failed",
                format_args!("reason=accept_failed detail={:?}", err.to_string()),
            );
        })
}

fn log(event: &str, fields: fmt::Arguments<'_>) {
    println!(
        "{} event={event} component=motd {fields}",
        Utc::now().format("%Y-%m-%d %H:%M:%SZ")
    );
}
