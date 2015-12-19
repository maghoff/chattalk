use std::convert;
use std::io;
use std::sync::{self,mpsc};
use plaintalk::{pushgenerator, pullparser};

#[derive(Debug)]
pub enum ClientError {
	Io(io::Error),
	St(&'static str),
	PushGenerator(pushgenerator::Error),
	PullParser(pullparser::Error),
	PoisonError,
	SendError,
}

impl convert::From<io::Error> for ClientError {
	fn from(err: io::Error) -> ClientError {
		ClientError::Io(err)
	}
}

impl convert::From<&'static str> for ClientError {
	fn from(err: &'static str) -> ClientError {
		ClientError::St(err)
	}
}

impl convert::From<pushgenerator::Error> for ClientError {
	fn from(err: pushgenerator::Error) -> ClientError {
		ClientError::PushGenerator(err)
	}
}

impl convert::From<pullparser::Error> for ClientError {
	fn from(err: pullparser::Error) -> ClientError {
		ClientError::PullParser(err)
	}
}

impl<T> convert::From<sync::PoisonError<T>> for ClientError {
	fn from(_err: sync::PoisonError<T>) -> ClientError {
		ClientError::PoisonError
	}
}

impl convert::From<mpsc::SendError<::ShoutMessage>> for ClientError {
	fn from(_err: mpsc::SendError<::ShoutMessage>) -> ClientError {
		ClientError::SendError
	}
}
