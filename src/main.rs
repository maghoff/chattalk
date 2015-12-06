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

fn read_field_as_string(message: &mut pullparser::Message) -> Result<Option<String>, pullparser::Error> {
	let mut string = String::new();
	match try!{message.get_field()} {
		Some(mut field) => {
			try!{field.read_to_string(&mut string)};
			Ok(Some(string))
		},
		None => Ok(None)
	}
}

fn read_field_as_slice<'a, 'b: 'a+'b>(message: &mut pullparser::Message, buffer: &'b mut[u8]) -> Result<Option<&'a [u8]>, pullparser::Error> {
	match try!{message.read_field(buffer)} {
		Some(len) => {
			Ok(Some(&buffer[0..len]))
		},
		None => Ok(None)
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

fn client_core(stream: TcpStream) -> Result<(), ClientError> {
	let mut buf_reader = BufReader::new(&stream);
	let mut parser = PullParser::new(&mut buf_reader);

	let mut buf_writer = BufWriter::new(&stream);
	let mut generator = PushGenerator::new(&mut buf_writer);

	let mut msg_id_buf = [0u8; 10];
	let mut command_buf = [0u8; 10];

	while let Some(mut message) = try!{parser.get_message()} {
		let msg_id = try!{read_field_as_slice(&mut message, &mut msg_id_buf)}.unwrap();

		match || -> Result<(), ProtocolError> {
			let command = try!{
					try!{read_field_as_slice(&mut message, &mut command_buf)}
					.ok_or(ProtocolError::InvalidCommand(BASIC_STRUCTURE))
				};

			match command {
				b"protocol" => {
					// TODO Implement protocol negotiation
					try!{message.ignore_rest()};
					try!{generator.write_message(&[ &msg_id, b"protocol", b"chattalk" ])};
				},
				b"join" => {
					let channel = try!{
							try!{read_field_as_string(&mut message)}
							.ok_or(ProtocolError::InvalidCommand(CMD_JOIN))
						};

					match try!{message.get_field()} {
						Some(mut extra_field) => {
							try!{extra_field.ignore_rest()};
							return Err(ProtocolError::InvalidCommand(CMD_JOIN));
						},
						None => ()
					}

					try!{generator.write_message(&[b"*", b"join", b"user", &channel.into_bytes()])};
					try!{generator.write_message(&[&msg_id, b"ok"])};
				},
				_ => {
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
