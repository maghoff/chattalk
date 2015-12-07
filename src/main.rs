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

const BASIC_STRUCTURE:&'static[u8] = b"Invalid format. Basic structure of all messages is: <message-ID> <command> [command arguments...]";
const CMD_JOIN:&'static[u8] = b"Usage: <msg-id> join <channel-name>";

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

fn client_core(stream: TcpStream) -> Result<(), ClientError> {
	let mut buf_reader = BufReader::new(&stream);
	let mut parser = PullParser::new(&mut buf_reader);

	let mut buf_writer = BufWriter::new(&stream);
	let mut generator = PushGenerator::new(&mut buf_writer);

	let mut msg_id_buf = [0u8; 10];
	let mut command_buf = [0u8; 10];

	while let Some(mut message) = try!{parser.get_message()} {
		let msg_id = try!{message.read_field_as_slice(&mut msg_id_buf)}
			.expect("PlainTalk parser yielded a message with zero fields");

		match || -> Result<(), ProtocolError> {
			match try!{expect(message.read_field_as_slice(&mut command_buf), BASIC_STRUCTURE)} {
				b"protocol" => {
					try!{message.ignore_rest()};
					try!{generator.write_message(&[b"*", b"note", b"'protocol' currently has no effect"])};
					try!{generator.write_message(&[ &msg_id, b"protocol", b"chattalk" ])};
				},
				b"join" => {
					let channel = try!{expect(message.read_field_as_string(), CMD_JOIN)};
					try!{expect_end(&message, CMD_JOIN)};

					try!{generator.write_message(&[b"*", b"note", b"'join' currently has no effect"])};
					try!{generator.write_message(&[b"*", b"join", b"user", &channel.into_bytes()])};
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
		}() {
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
