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
            
            if src[0] == super::messages::Info::Magick as u8 {
                break;
            } else {
                src.advance(1);
            }
        }

        let remaning = src.remaining();
        let mut buf: BytesMut = src.clone();
        buf.advance(1); // skip magick
        
        match super::messages::Response::decode_length_delimited(&mut buf) {
            Ok(msg) => {
                src.advance(remaning - buf.remaining());
                Ok(Some(msg))
            }
            Err(e) => {
                if (e == prost::DecodeError::new("buffer underflow")) || buf.is_empty() {
                    Ok(None)
                } else {
                    Err(e.into())
                }
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
