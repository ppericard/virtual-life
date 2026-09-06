use std::{process::ExitCode, time::Duration};
use virtual_life::{runner::Config, web};

async fn run() -> Result<(), String> {
    let mut config = Config::default();
    let mut port = 7878_u16;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                println!(
                    "Usage: web [--port N] [--running] [--tick-ms N] [--sample-every N]\nStarts paused at http://127.0.0.1:7878. Ctrl+C stops and joins the worker.\nClosing or refreshing a browser does not stop/reset the experiment. Port 0 chooses a free port."
                );
                return Ok(());
            }
            "--running" => config.start_paused = false,
            "--port" => {
                port = arguments
                    .next()
                    .ok_or("missing port")?
                    .parse()
                    .map_err(|_| "invalid port")?
            }
            "--tick-ms" => {
                let ms: u64 = arguments
                    .next()
                    .ok_or("missing tick interval")?
                    .parse()
                    .map_err(|_| "invalid tick interval")?;
                if ms > 60_000 {
                    return Err("tick-ms must be between 0 and 60000".into());
                }
                config.tick_interval = Duration::from_millis(ms);
            }
            "--sample-every" => {
                config.sample_every = arguments
                    .next()
                    .ok_or("missing sample cadence")?
                    .parse()
                    .map_err(|_| "invalid sample cadence")?
            }
            _ => return Err(format!("unknown argument {argument}; use --help")),
        }
    }
    if config.sample_every == 0 {
        return Err("sample-every must be positive".into());
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|e| e.to_string())?;
    println!(
        "VirtualLife http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    println!("Scripted demonstration. Ctrl+C shuts down the server and worker.");
    web::serve(listener, config, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

#[tokio::main(worker_threads = 2)]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
