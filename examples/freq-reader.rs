use clap::Parser;

use laser_setup_interface::{self, I2c};

/// Laser setup freq reader
#[derive(Parser, Debug)]
#[allow(non_snake_case)]
struct Cli {
    /// Serial port name
    #[clap(short('P'), long)]
    port: String,

    /// Serial timeout in milliseconds
    #[clap(short, long, default_value = "100")]
    timeout: u64,

    /// Update interval in milliseconds
    #[clap(short, long, default_value = "100")]
    interval: u64,

    /// Select channel
    #[clap(short('B'), long, default_value = "0")]
    bus: u32,

    /// List avalable i2c buses
    #[clap(short('L'), long, default_value = "false")]
    list: bool,

    /// Device address
    #[clap(short('A'), long, default_value = "11")]
    device_addr: u8,

    /// Device register
    #[clap(short('R'), long, default_value = "8")]
    device_reg: u8,
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

    interface.select_i2c_bus(args.bus);
    println!(
        "Reading frequency port {}, bus: {}, device: 0x{:02x}",
        args.port, args.bus, args.device_addr
    );

    loop {
        let res: Result<[u8; 4], laser_setup_interface::Error> = {
            let mut buf = [0; std::mem::size_of::<f32>()];
            let addr = [args.device_reg; 1];
            let mut ops = vec![
                laser_setup_interface::Operation::Write(&addr),
                laser_setup_interface::Operation::Read(&mut buf),
            ];
            interface
                .transaction(args.device_addr, &mut ops)
                .await
                .map(|_| buf)
        };

        match res {
            Ok(buf) => {
                let f = {
                    let byte_array: [u8; 4] = buf[0..4].try_into().unwrap();
                    f32::from_le_bytes(byte_array)
                };

                println!("Frequency: {:.2} Hz", f);
            }
            Err(e) => log::error!("Freqmeter error: {:?}", e),
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(args.interval)).await;
    }
}
