// SPDX-License-Identifier: Apache-2.0 OR MIT

use ::std::env;
use ::std::io::stderr;
use ::std::process::exit;

use ::anyhow::Context;
use ::clap::Parser;
use ::tracing::{error, level_filters::LevelFilter};
use ::tracing_subscriber::EnvFilter;
use ::tracing_subscriber::layer::SubscriberExt;
use ::tracing_subscriber::util::SubscriberInitExt;

use ::double_compresso_common_client::bt::{self, ScanEvent, next_scan_event};
use ::double_compresso_common_client::error::Result;

mod cli;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    unsafe {
        // SAFETY: This is safe because we are doing this right at the start before creating any other threads.
        env::set_var("RUST_BACKTRACE", "full");
    }

    // Parse command line arguments.
    let args = cli::Cli::parse();

    // Configure logging.
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(stderr))
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let return_code = match start_async(args) {
        Ok(()) => 0,

        Err(error) => {
            error!("{:?}", error);
            1
        }
    };

    exit(return_code)
}

fn start_async(args: cli::Cli) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .worker_threads(1)
        .max_blocking_threads(1)
        .thread_name("amain")
        .build()
        .context("Unable to initialize Tokio async runtime")?;

    let _runtime_guard = runtime.enter();

    runtime.block_on(async_main(args))?;

    Ok(())
}

async fn async_main(args: cli::Cli) -> Result<()> {
    match &args.subcommand {
        cli::SubCommand::BluetoothAdapters => {
            let list = bt::StateList::state_new().await?;
            for name in list.adapter_names() {
                println!("{}", name);
            }
        }

        cli::SubCommand::Scan(scan) => {
            let (_scan, mut rx_scan_event) = start_scan(&scan.bt).await?;
            loop {
                let event = next_scan_event(&mut rx_scan_event).await?;
                match event {
                    ScanEvent::Found(addr, _) => {
                        println!("found {}", addr.to_string());
                    }

                    ScanEvent::Lost(addr) => {
                        println!("lost {}", addr.to_string());
                    }
                }
            }
        }

        cli::SubCommand::Speedtest(speedtest) => {
            let (scan, mut rx_scan_event) = start_scan(&speedtest.bt).await?;

            let id = loop {
                if let ScanEvent::Found(bt_addr, id) = next_scan_event(&mut rx_scan_event).await? {
                    if speedtest.bt_addr.device.is_none()
                        || (Some(bt_addr) == speedtest.bt_addr.device)
                    {
                        break id;
                    }
                }
            };

            scan.state_next(&id).await.map_err(|(e, _)| e)?;
        }
    }

    Ok(())
}

async fn start_scan(bt: &cli::CommonBTOpts) -> Result<(bt::StateScan, bt::ScanEventStream)> {
    let bluetooth_adapter = if let Some(bluetooth_adapter) = bt.bluetooth_adapter.as_ref() {
        bt::AdapterSelection::PartialName(bluetooth_adapter)
    } else {
        bt::AdapterSelection::Any
    };
    let list = bt::StateList::state_new().await?;
    list.state_next(bluetooth_adapter).await
}
