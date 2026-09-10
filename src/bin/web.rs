use std::process::ExitCode;
use virtual_life::{launch, web};

async fn run() -> Result<(), String> {
    let launch = launch::parse(std::env::args().skip(1), true)?;
    if launch.help {
        println!(
            "Usage: web {} [--port N] [--running] [--tick-ms N] [--sample-every N]\nStarts paused at http://127.0.0.1:7878. Ctrl+C stops and joins the worker.\nClosing or refreshing a browser does not stop/reset the experiment. Port 0 chooses a free port.",
            launch::OPTIONS
        );
        return Ok(());
    }
    launch::describe(&launch.config)?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, launch.port))
        .await
        .map_err(|e| e.to_string())?;
    println!(
        "VirtualLife http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    println!("Ctrl+C shuts down the server and worker.");
    web::serve(listener, launch.config, async {
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
