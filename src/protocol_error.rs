use std::convert;
use client_error::ClientError;

pub enum ProtocolError {
	InvalidCommand(&'static [u8]),
	PlaintalkError(ClientError),
}

impl<T> convert::From<T> for ProtocolError
	where ClientError : convert::From<T>
{
	fn from(err: T) -> ProtocolError {
		ProtocolError::PlaintalkError(ClientError::from(err))
	}
}
