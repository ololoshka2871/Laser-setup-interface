use clap::Parser;

use laser_setup_interface::{self, I2c};

/// Laser setup CLI controller
#[derive(Parser, Debug)]
#[allow(non_snake_case)]
struct Cli {
    /// Serial port name
    #[clap(short, long)]
    port: String,

    /// Serial timeout in milliseconds
    #[clap(short, long, default_value = "100")]
    timeout: u64,

    /// Select channel
    #[clap(short('B'), long)]
    bus: Option<u32>,

    /// List avalable i2c buses
    #[clap(short('L'), long, default_value = "false")]
    list: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), laser_setup_interface::Error> {
    env_logger::init();

    let args = Cli::parse();

    let mut interface = laser_setup_interface::LaserSetup::new(
        &args.port,
        std::time::Duration::from_millis(args.timeout),
    );

    if args.list {
        println!("Enumerating i2c buses on device port {}", args.port);

        let buses = interface.enumerate_i2c_buses().await?;
        for bus in buses {
            println!("I2c bus {}: speed: {}", bus.id, bus.speed);
        }
        return Ok(());
    }

    if let Some(bus) = args.bus {
        interface.select_i2c_bus(bus);
        println!(
            "Detecting i2c devices on device port {}, bus: {}",
            args.port,
            bus
        );

        let mut total_found = 0;
        for addr in 0..=(u8::MAX >> 1) {
            let mut buf = [0; 1];

            let mut ops = [laser_setup_interface::Operation::Read(&mut buf); 1];

            match interface.transaction(addr, &mut ops).await {
                Ok(_) => {
                    println!("Found i2c device at address: 0x{:02x}", addr);
                    total_found += 1;
                }
                Err(laser_setup_interface::Error::I2C(
                    embedded_hal_async::i2c::ErrorKind::NoAcknowledge(_),
                )) => { /* Nak: no device present */}
                Err(e) => panic!("Error: {:?}", e),
            }
        }
        log::info!("Found {} i2c devices", total_found);

        Ok(())
    } else {
        log::error!("No i2c bus selected!");
        Err(laser_setup_interface::Error::UnexpectedEndOfStream)
    }
}
