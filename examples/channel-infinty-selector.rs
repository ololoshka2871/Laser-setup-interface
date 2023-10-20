use clap::Parser;

use laser_setup_interface;

/// Laser setup CLI controller
#[derive(Parser, Debug)]
#[allow(non_snake_case)]
struct Cli {
    /// Serial port name
    #[clap(short('P'), long)]
    port: String,

    /// Timeout in milliseconds
    #[clap(short('t'), long, default_value = "100")]
    timeout: u64,
}

struct SelectorCmd(pub u32);

impl laser_setup_interface::ControlState for SelectorCmd {
    fn valve(&self) -> Option<laser_setup_interface::ValveState> {
        None
    }

    fn channel(&self) -> Option<u32> {
        Some(self.0)
    }

    fn camera(&self) -> Option<laser_setup_interface::CameraState> {
        None
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), laser_setup_interface::Error> {
    env_logger::init();

    let args = Cli::parse();

    log::debug!("Starting laser-setup controller with args: {:?}", args);

    let mut interface = laser_setup_interface::LaserSetup::new(
        &args.port,
        std::time::Duration::from_millis(args.timeout),
    );

    let current_state = interface.read().await?;
    log::info!("Current laser-setup state: {:?}", current_state);

    loop {
        for ch in 0..laser_setup_interface::CHANNELS_COUNT {
            let cmd = SelectorCmd(ch);
            match interface.write(&cmd).await {
                Ok(res) => log::info!("Selected channel {ch}, state: {:?}", res),
                Err(e) => log::error!("{e:?}"),
            };
        }
    }
}
