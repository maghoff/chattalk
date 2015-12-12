extern crate plaintalk;

use std::sync::{Arc,Mutex,PoisonError};
use std::sync::mpsc::{Sender,SendError};
use std::convert;
use std::io::{self,Read,Write};
use plaintalk::pullparser::{self,PullParser};
use plaintalk::pushgenerator::PushGenerator;

#[derive(Debug)]
pub enum ClientError {
	Io(io::Error),
	St(&'static str),
	PushGenerator(plaintalk::pushgenerator::Error),
	PullParser(plaintalk::pullparser::Error),
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

impl convert::From<plaintalk::pushgenerator::Error> for ClientError {
	fn from(err: plaintalk::pushgenerator::Error) -> ClientError {
		ClientError::PushGenerator(err)
	}
}

impl convert::From<plaintalk::pullparser::Error> for ClientError {
	fn from(err: plaintalk::pullparser::Error) -> ClientError {
		ClientError::PullParser(err)
	}
}

impl<T> convert::From<PoisonError<T>> for ClientError {
	fn from(_err: PoisonError<T>) -> ClientError {
		ClientError::PoisonError
	}
}

impl convert::From<SendError<::ShoutMessage>> for ClientError {
	fn from(_err: SendError<::ShoutMessage>) -> ClientError {
		ClientError::SendError
	}
}

enum ProtocolError {
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

fn expect<T, E>(field: Result<Option<T>, E>, err: &'static [u8]) -> Result<T, ProtocolError>
	where ProtocolError : convert::From<E>
{
	try!(field).ok_or(ProtocolError::InvalidCommand(err))
}

fn expect_end(message: &pullparser::Message, err: &'static [u8]) -> Result<(), ProtocolError> {
	match message.at_end() {
		true => Ok(()),
		false => Err(ProtocolError::InvalidCommand(err))
	}
}

pub struct ClientConnection<T: Write> {
	nick: String,
	command_buf: [u8; 10],
	generator: Arc<Mutex<PushGenerator<T>>>,
	tx: Sender<::ShoutMessage>,
}

impl<T: Write> ClientConnection<T> {
	pub fn new(generator: Arc<Mutex<PushGenerator<T>>>, tx: Sender<::ShoutMessage>) -> ClientConnection<T> {
		ClientConnection {
			nick: String::new(),
			command_buf: [0u8; 10],
			generator: generator,
			tx: tx,
		}
	}

	fn handle_message(
		&mut self,
		msg_id: &[u8],
		message: &mut pullparser::Message,
	) ->
		Result<(), ProtocolError>
	{
		static BASIC_STRUCTURE:&'static[u8] =
			b"Invalid format. Basic structure of all messages is: \
			<message-ID> <command> [command arguments...] \
			(try `0 help`)";

		match try!{expect(message.read_field_as_slice(&mut self.command_buf), BASIC_STRUCTURE)} {
			b"help" => {
				static USAGE:&'static[u8] = b"Usage: <msg-id> help";

				static HELP:&'static[u8] =
					b"Available commands:\n\
					<msgid> protocol {<feature> ...}    Protocol negotiation (not implemented)\n\
					<msgid> join <channel>              Join (not implemented)\n\
					<msgid> nick <new nick>             Set your nick to <new nick>\n\
					<msgid> shout <statement>           Shout a statement to all connected clients";

				try!{expect_end(&message, USAGE)};

				let mut generator = try!{self.generator.lock()};
				try!{generator.write_message(&[b"*", b"note", HELP])};
				try!{generator.write_message(&[msg_id, b"ok"])};
			},
			b"protocol" => {
				try!{message.ignore_rest()};
				let mut generator = try!{self.generator.lock()};
				try!{generator.write_message(&[b"*", b"note", b"'protocol' currently has no effect"])};
				try!{generator.write_message(&[msg_id, b"ok", b"chattalk"])};
			},
			b"join" => {
				static USAGE:&'static[u8] = b"Usage: <msg-id> join <channel-name>";

				let channel = try!{expect(message.read_field_as_string(), USAGE)};
				try!{expect_end(&message, USAGE)};

				let mut generator = try!{self.generator.lock()};
				try!{generator.write_message(&[b"*", b"note", b"'join' currently has no effect"])};
				try!{generator.write_message(&[b"*", b"join", &self.nick.as_bytes(), &channel.into_bytes()])};
				try!{generator.write_message(&[msg_id, b"ok"])};
			},
			b"nick" => {
				static USAGE:&'static[u8] = b"Usage: <msg-id> nick <new-nick>";

				let new_nick = try!{expect(message.read_field_as_string(), USAGE)};
				try!{expect_end(&message, USAGE)};

				let mut generator = try!{self.generator.lock()};
				try!{generator.write_message(&[b"*", b"nick", &self.nick.as_bytes(), &new_nick.as_bytes()])};
				self.nick = new_nick;

				try!{generator.write_message(&[msg_id, b"ok"])};
			},
			b"shout" => {
				static USAGE:&'static[u8] = b"Usage: <msg-id> shout <statement>";

				let statement = try!{expect(message.read_field_as_string(), USAGE)};
				try!{expect_end(&message, USAGE)};

				let mut generator = try!{self.generator.lock()};
				try!{self.tx.send(::ShoutMessage::Shout(self.nick.clone(), statement))};
				try!{generator.write_message(&[msg_id, b"ok"])};
			},
			command => {
				try!{message.ignore_rest()};
				let mut generator = try!{self.generator.lock()};
				try!{generator.write_message(&[
					msg_id, b"error", b"invalid-command",
					&format!("unknown command: {}", String::from_utf8_lossy(command)).into_bytes()
				])};
			},
		};

		Ok(())
	}

	pub fn handle_protocol<R: Read>(&mut self, mut parser: PullParser<R>) -> Result<(), ClientError> {
		let mut msg_id_buf = [0u8; 10];

		while let Some(mut message) = try!{parser.get_message()} {
			let msg_id = try!{message.read_field_as_slice(&mut msg_id_buf)}
				.expect("PlainTalk parser yielded a message with zero fields");

			match self.handle_message(msg_id, &mut message) {
				Ok(()) => (),
				Err(ProtocolError::InvalidCommand(usage)) => {
					let mut generator = try!{self.generator.lock()};
					try!{generator.write_message(
						&[msg_id, b"error", b"invalid-command", usage])};
					try!{message.ignore_rest()};
				},
				Err(ProtocolError::PlaintalkError(err)) => return Err(err),
			}
		}

		Ok(())
	}
}
