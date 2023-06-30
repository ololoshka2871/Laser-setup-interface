use clap::Parser;

use laser_setup_interface;

/// Laser setup CLI controller
#[derive(Parser, Debug)]
#[allow(non_snake_case)]
struct Cli {
    /// Serial port name
    #[clap(short, long)]
    port: String,

    /// Timeout in milliseconds
    #[clap(short, long, default_value = "100")]
    timeout: u64,

    /// Select channel
    #[clap(short('C'), long)]
    channel: Option<u32>,

    /// Open camera
    #[clap(short, long)]
    open: bool,

    /// Close camera
    #[clap(short, long)]
    close: bool,

    /// vacuum on/off
    #[clap(short, long)]
    vacuum: Option<bool>,
}

impl laser_setup_interface::ControlState for Cli {
    fn valve(&self) -> Option<bool> {
        self.vacuum
    }

    fn channel(&self) -> Option<u32> {
        self.channel
    }

    fn camera(&self) -> Option<bool> {
        if self.open {
            Some(true)
        } else if self.close {
            Some(false)
        } else {
            None
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), laser_setup_interface::protobuf::Error> {
    let mut args = Cli::parse();

    println!("Starting laser-setup controller with args: {:?}", args);

    let mut interface = laser_setup_interface::LaserSetup::new(
        &args.port,
        std::time::Duration::from_millis(args.timeout),
    );

    if args.open && args.close {
        panic!("Open and close flags are mutually exclusive");
    }

    if Some(true) == args.vacuum {
        args.close = true; // force close camera before vacuum on
    }

    if args.open {
        args.vacuum = Some(false); // force vacuum off before open camera
    }

    if let Some(ch) = &args.channel {
        if *ch >= laser_setup_interface::CHANNELS_COUNT {
            panic!("Channel must be in range 0..15");
        }
    }

    let res = interface.write(&args).await?;
    println!("Current laser-setup state: {:?}", res);
    Ok(())
}
