// SPDX-License-Identifier: Apache-2.0 OR MIT

use ::clap::{Parser, Subcommand};

use ::double_compresso_common_client::bt::BDAddr;

#[derive(Parser)]
#[command(about, version)]
pub(crate) struct Cli {
    /// Command.
    #[command(subcommand)]
    pub(crate) subcommand: SubCommand,
}

#[derive(Subcommand)]
pub(crate) enum SubCommand {
    /// List available Bluetooth adapters.
    #[command()]
    BluetoothAdapters,

    /// Scan for Double Compresso devices.
    #[command()]
    Scan(Scan),

    /// Run "speedtest" for Double Compresso devices.
    #[command()]
    Speedtest(Speedtest),
}

#[derive(Parser)]
pub(crate) struct Scan {
    #[clap(flatten)]
    pub(crate) bt: CommonBTOpts,
}

#[derive(Parser)]
pub(crate) struct Speedtest {
    #[clap(flatten)]
    pub(crate) bt: CommonBTOpts,
    #[clap(flatten)]
    pub(crate) bt_addr: BTAddress,
}

#[derive(Parser)]
pub(crate) struct CommonBTOpts {
    /// Use Bluetooth adapter with this name. Name can be partial.
    #[arg(short = 'b', long, value_name = "NAME")]
    pub(crate) bluetooth_adapter: Option<String>,
}

#[derive(Parser)]
pub(crate) struct BTAddress {
    /// Address of a specific Double Compresso device.
    pub(crate) device: Option<BDAddr>,
}
