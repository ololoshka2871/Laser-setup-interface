use bytes::{Buf, BufMut, BytesMut};

use prost::Message;
use tokio_util::codec::{Decoder, Encoder};

use super::messages::{Request, Response};

pub(crate) struct ProtobufMDCodec;

impl Decoder for ProtobufMDCodec {
    type Item = Response;
    type Error = super::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            if src.is_empty() {
                return Ok(None);
            }
            if src.get_u8() == super::messages::Info::Magick as u8 {
                break;
            }
        }

        match super::messages::Response::decode_length_delimited(src) {
            Ok(msg) => return Ok(Some(msg)),
            Err(e) => {
                if e == prost::DecodeError::new("unexpected EOF") {
                    return Ok(None);
                }
                return Err(e.into());
            }
        }
    }

    fn framed<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Sized>(
        self,
        io: T,
    ) -> tokio_util::codec::Framed<T, Self>
    where
        Self: Sized,
    {
        tokio_util::codec::Framed::new(io, self)
    }
}

impl Encoder<Request> for ProtobufMDCodec {
    type Error = super::Error;

    fn encode(&mut self, req_type: Request, buf: &mut BytesMut) -> Result<(), Self::Error> {
        buf.put_u8(super::messages::Info::Magick as u8);

        req_type.encode_length_delimited(buf)?;
        req_type.encode(buf)?;

        Ok(())
    }
}
