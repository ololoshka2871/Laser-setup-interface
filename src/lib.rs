#![feature(async_fn_in_trait)]

use std::time::Duration;

use embedded_hal_async::i2c::ErrorKind;
use protobuf::messages::i2c_request;
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

pub use embedded_hal_async::i2c::{I2c, Operation};

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

#[derive(Debug, Clone, Copy)]
pub struct I2CBus {
    pub id: u32,
    pub speed: u32,
}

pub struct LaserSetup {
    io: tokio_util::codec::Framed<tokio_serial::SerialStream, ProtobufMDCodec>,
    timeout: Duration,

    selected_i2c_bus: u32,
}

impl LaserSetup {
    pub fn new<'a>(port: impl Into<std::borrow::Cow<'a, str>>, timeout: Duration) -> Self {
        let port = tokio_serial::new(port, 1500000)
            .open_native_async()
            .unwrap();
        Self {
            io: ProtobufMDCodec.framed(port),
            timeout,
            selected_i2c_bus: 0,
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

    pub fn select_i2c_bus(&mut self, bus_id: u32) {
        self.selected_i2c_bus = bus_id;
    }

    pub async fn enumerate_i2c_buses(&mut self) -> Result<Vec<I2CBus>, Error> {
        let mut req = protobuf::new_request();

        let mut i2c_request = protobuf::messages::I2cRequest::default();
        i2c_request.request = Some(i2c_request::Request::Enumerate(
            protobuf::messages::Empty {},
        ));

        req.i2c.replace(i2c_request);

        self.io.send(req).await?;
        let resp = self.read_responce().await?;

        if resp.global_status != Status::Ok as i32 {
            return Err(Error::Protocol(
                Status::from_i32(resp.global_status).unwrap(),
            ));
        }

        match resp.i2c {
            Some(protobuf::messages::I2cResponse {
                response:
                    Some(protobuf::messages::i2c_response::Response::Enumerate(
                        protobuf::messages::I2cEnumerateResponse { buses },
                    )),
            }) => Ok(buses
                .into_iter()
                .map(|b| I2CBus {
                    id: b.bus,
                    speed: b.max_speed,
                })
                .collect()),
            _ => Err(Error::Protocol(Status::I2c)),
        }
    }
}

impl embedded_hal_async::i2c::ErrorType for LaserSetup {
    type Error = Error;
}

impl I2c for LaserSetup {
    async fn transaction(
        &mut self,
        address: u8,
        operations: &mut [embedded_hal_async::i2c::Operation<'_>],
    ) -> Result<(), Self::Error> {
        use crate::protobuf::messages::{
            i2c_response::Response, i2c_result::Operation, I2cOperation, I2cResult,
        };
        use protobuf::messages::i2c_operation::Operation as I2cOperationType;

        fn status2_err(status: i32) -> Result<(), Error> {
            match I2cResultCode::from_i32(status).unwrap() {
                I2cResultCode::I2cInvalidBus => Err(Error::I2C(ErrorKind::Bus)),
                I2cResultCode::I2cTooLongData => Err(Error::I2C(ErrorKind::Overrun)),
                I2cResultCode::I2cNak => Err(Error::I2C(ErrorKind::NoAcknowledge(
                    embedded_hal_async::i2c::NoAcknowledgeSource::Unknown,
                ))),
                _ => Ok(()),
            }
        }

        use protobuf::messages::I2cResultCode;

        let mut req = protobuf::new_request();

        let req_operations: Vec<I2cOperation> = operations
            .into_iter()
            .map(|o| match o {
                embedded_hal_async::i2c::Operation::Write(w) => I2cOperation {
                    operation: Some(I2cOperationType::Write(
                        protobuf::messages::I2cWriteRequest {
                            address: address as u32,
                            data: w.to_vec(),
                        },
                    )),
                },
                embedded_hal_async::i2c::Operation::Read(r) => I2cOperation {
                    operation: Some(I2cOperationType::Read(protobuf::messages::I2cReadRequest {
                        address: address as u32,
                        length: r.len() as u32,
                    })),
                },
            })
            .collect();

        let req_len = req_operations.len();

        let req_sequence = protobuf::messages::I2cRequest {
            request: Some(protobuf::messages::i2c_request::Request::Sequence(
                protobuf::messages::I2cSequence {
                    bus: self.selected_i2c_bus,
                    address: address as u32,
                    operations: req_operations,
                },
            )),
        };

        req.i2c = Some(req_sequence);

        self.io.send(req).await?;
        let resp = self.read_responce().await?;

        match Status::from_i32(resp.global_status).unwrap() {
            Status::Ok => {}
            Status::I2c => {
                if let Some(r) = resp.i2c {
                    if let Some(crate::protobuf::messages::i2c_response::Response::Sequence(s)) =
                        r.response
                    {
                        let is_nak = s.operations.iter().any(|o| match o.operation {
                            Some(Operation::Write(status)) => {
                                status == I2cResultCode::I2cNak as i32
                            }
                            Some(Operation::Read(protobuf::messages::I2cReadResponse {
                                status,
                                ..
                            })) => status == I2cResultCode::I2cNak as i32,
                            _ => false,
                        });
                        if is_nak {
                            return Err(Error::I2C(ErrorKind::NoAcknowledge(
                                embedded_hal_async::i2c::NoAcknowledgeSource::Unknown,
                            )));
                        }
                    }
                }
                return Err(Error::I2C(ErrorKind::Bus));
            }
            e => return Err(Error::Protocol(e)),
        }

        match resp.i2c {
            Some(protobuf::messages::I2cResponse {
                response:
                    Some(Response::Sequence(protobuf::messages::I2cSequenceResult {
                        operations: res_operations,
                        bus,
                        address,
                    })),
            }) => {
                if req_len != res_operations.len()
                    || bus != self.selected_i2c_bus
                    || address != address as u32
                {
                    return Err(Error::Protocol(Status::I2c));
                }

                for v in operations.iter_mut().zip(res_operations.iter()) {
                    match v {
                        (
                            embedded_hal_async::i2c::Operation::Read(buf),
                            I2cResult {
                                operation:
                                    Some(Operation::Read(protobuf::messages::I2cReadResponse {
                                        data,
                                        status,
                                    })),
                            },
                        ) => {
                            status2_err(*status)?;
                            buf.copy_from_slice(data);
                        }
                        (
                            embedded_hal_async::i2c::Operation::Write(_),
                            I2cResult {
                                operation: Some(Operation::Write(status)),
                            },
                        ) => {
                            status2_err(*status)?;
                        }
                        _ => return Err(Error::Protocol(Status::I2c)),
                    }
                }
            }
            _ => return Err(Error::Protocol(Status::I2c)),
        }
        Ok(())
    }
}
