use std::time::Duration;

use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Decoder;

use futures::{SinkExt, StreamExt};

mod protobuf;
use protobuf::messages::{ControlRequest, Status};

pub use protobuf::messages::ActuatorState as CameraState;
pub use protobuf::messages::ValveState;
pub use protobuf::Error;

use protobuf::protobuf_md_codec::ProtobufMDCodec;


pub const CHANNELS_COUNT: u32 = 16;

pub trait ControlState {
    /// Is vacuum?
    fn valve(&self) -> Option<ValveState>;
    /// Channel number
    fn channel(&self) -> Option<u32>;
    /// Is camera opened?
    fn camera(&self) -> Option<CameraState>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CurrentControlState {
    /// Is vacuum enabled?
    pub valve: ValveState,
    /// Selected channel
    pub channel: u32,
    /// Is camera opened?
    pub camera: CameraState,
}

pub struct LaserSetup {
    io: tokio_util::codec::Framed<tokio_serial::SerialStream, ProtobufMDCodec>,
    timeout: Duration,
}

impl LaserSetup {
    pub fn new<'a>(port: impl Into<std::borrow::Cow<'a, str>>, timeout: Duration) -> Self {
        let port = tokio_serial::new(port, 1500000)
            .open_native_async()
            .unwrap();
        Self {
            io: ProtobufMDCodec.framed(port),
            timeout,
        }
    }

    pub async fn read_responce(&mut self) -> Result<protobuf::messages::Response, Error> {
        let res = tokio::time::timeout(self.timeout, self.io.next()).await;
        match res {
            Ok(Some(r)) => r,
            Ok(None) => Err(Error::UnexpectedEndOfStream),
            Err(_) => Err(Error::Timeout),
        }
    }

    fn decode_current_state(
        ctrl: &Option<protobuf::messages::ControlResponse>,
    ) -> CurrentControlState {
        if let Some(ctrl) = ctrl {
            CurrentControlState {
                valve: ValveState::from_i32(ctrl.valve_state).unwrap(),
                channel: ctrl.selected_channel,
                camera: CameraState::from_i32(ctrl.actuator_state).unwrap(),
            }
        } else {
            panic!("No control field in response")
        }
    }

    pub async fn write(
        &mut self,
        request: &impl ControlState,
    ) -> Result<CurrentControlState, Error> {
        let mut req = protobuf::new_request();

        let mut ctrl = protobuf::messages::ControlRequest::default();

        if let Some(valve) = request.valve() {
            ctrl.valve_state = Some(valve as i32);
        }
        if let Some(camera) = request.camera() {
            ctrl.actuator_state = Some(camera as i32);
        }
        ctrl.select_channel = request.channel();

        req.control = Some(ctrl);

        self.io.send(req).await?;

        let resp = self.read_responce().await?;
        match Status::from_i32(resp.global_status).unwrap() {
            Status::Ok => Ok(Self::decode_current_state(&resp.control)),
            e => Err(Error::Protocol(e)),
        }
    }

    pub async fn read(&mut self) -> Result<CurrentControlState, Error> {
        let mut req = protobuf::new_request();
        req.control = Some(ControlRequest::default());
        self.io.send(req).await?;

        let resp = self.read_responce().await?;
        match Status::from_i32(resp.global_status).unwrap() {
            Status::Ok => Ok(Self::decode_current_state(&resp.control)),
            e => Err(Error::Protocol(e)),
        }
    }
}
