extern crate plaintalk;

use std::convert;
use std::io::{self,Read,BufReader,BufWriter};
use std::net::{TcpListener, TcpStream};
use std::thread;
use plaintalk::{pullparser, pushgenerator};
use plaintalk::pullparser::PullParser;
use plaintalk::pushgenerator::PushGenerator;

#[derive(Debug)]
enum ClientError {
	Io(io::Error),
	St(&'static str),
	PushGenerator(plaintalk::pushgenerator::Error),
	PullParser(plaintalk::pullparser::Error),
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

enum ProtocolError {
	InvalidCommand(&'static [u8]),
	PlaintalkError(ClientError),
}

impl convert::From<pullparser::Error> for ProtocolError {
	fn from(err: pullparser::Error) -> ProtocolError {
		ProtocolError::PlaintalkError(ClientError::from(err))
	}
}

impl convert::From<pushgenerator::Error> for ProtocolError {
	fn from(err: pushgenerator::Error) -> ProtocolError {
		ProtocolError::PlaintalkError(ClientError::from(err))
	}
}

impl convert::From<&'static str> for ProtocolError {
	fn from(err: &'static str) -> ProtocolError {
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

struct ClientConnection {
	nick: String,
	command_buf: [u8; 10],
}

impl ClientConnection {
	fn new() -> ClientConnection {
		ClientConnection {
			nick: String::new(),
			command_buf: [0u8; 10],
		}
	}

	fn handle_message(
		&mut self,
		msg_id: &[u8],
		message: &mut pullparser::Message,
		generator: &mut pushgenerator::PushGenerator
	) ->
		Result<(), ProtocolError>
	{
		static BASIC_STRUCTURE:&'static[u8] =
			b"Invalid format. Basic structure of all messages is: <message-ID> <command> [command arguments...]";

		match try!{expect(message.read_field_as_slice(&mut self.command_buf), BASIC_STRUCTURE)} {
			b"protocol" => {
				try!{message.ignore_rest()};
				try!{generator.write_message(&[b"*", b"note", b"'protocol' currently has no effect"])};
				try!{generator.write_message(&[ &msg_id, b"protocol", b"chattalk" ])};
			},
			b"join" => {
				static USAGE:&'static[u8] = b"Usage: <msg-id> join <channel-name>";

				let channel = try!{expect(message.read_field_as_string(), USAGE)};
				try!{expect_end(&message, USAGE)};

				try!{generator.write_message(&[b"*", b"note", b"'join' currently has no effect"])};
				try!{generator.write_message(&[b"*", b"join", &self.nick.as_bytes(), &channel.into_bytes()])};
				try!{generator.write_message(&[&msg_id, b"ok"])};
			},
			b"nick" => {
				static USAGE:&'static[u8] = b"Usage: <msg-id> nick <new-nick>";

				let new_nick = try!{expect(message.read_field_as_string(), USAGE)};
				try!{expect_end(&message, USAGE)};

				try!{generator.write_message(&[b"*", b"nick", &self.nick.as_bytes(), &new_nick.as_bytes()])};
				self.nick = new_nick;

				try!{generator.write_message(&[&msg_id, b"ok"])};
			},
			command => {
				try!{message.ignore_rest()};
				try!{generator.write_message(&[
					&msg_id, b"error", b"invalid-command",
					&format!("unknown command: {}", String::from_utf8_lossy(command)).into_bytes()
				])};
			},
		};

		Ok(())
	}
}

fn client_core(stream: TcpStream) -> Result<(), ClientError> {
	let mut buf_reader = BufReader::new(&stream);
	let mut parser = PullParser::new(&mut buf_reader);

	let mut buf_writer = BufWriter::new(&stream);
	let mut generator = PushGenerator::new(&mut buf_writer);

	let mut msg_id_buf = [0u8; 10];

	let mut client_connection = ClientConnection::new();

	while let Some(mut message) = try!{parser.get_message()} {
		let msg_id = try!{message.read_field_as_slice(&mut msg_id_buf)}
			.expect("PlainTalk parser yielded a message with zero fields");

		match client_connection.handle_message(msg_id, &mut message, &mut generator) {
			Ok(()) => (),
			Err(ProtocolError::InvalidCommand(usage)) => {
				try!{generator.write_message(
					&[&msg_id, b"error", b"invalid-command", &usage])};
				try!{message.ignore_rest()};
			},
			Err(ProtocolError::PlaintalkError(err)) => return Err(err),
		}
	}

	Ok(())
}

fn handle_client(stream: TcpStream) {
	println!("Client connected");

	match client_core(stream) {
		Ok(()) => println!("Connection closed"),
		Err(e) => println!("Connection terminated with error: {:?}", e),
	}
}

fn main() {
	let listener = TcpListener::bind("127.0.0.1:2203").unwrap();
	println!("Listening to {}", listener.local_addr().unwrap());

	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				thread::spawn(move|| {
					handle_client(stream)
				});
			}
			Err(e) => {
				println!("Failed connection attempt: {:?}", e);
			}
		}
	}

	drop(listener);
}
