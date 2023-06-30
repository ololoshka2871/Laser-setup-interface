mod pb_error;

pub mod protobuf_md_codec;

pub mod messages;

pub use pb_error::Error;

pub fn new_request() -> messages::Request {
    let mut rng = rand::thread_rng();

    messages::Request {
        id: rand::Rng::gen(&mut rng),
        device_id: messages::Info::LaserSetupId as u32,
        protocol_version: messages::Info::ProtocolVersion as u32,

        ..Default::default()
    }
}