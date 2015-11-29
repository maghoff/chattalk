extern crate plaintalk;

use std::convert;
use std::io::{self,BufReader,BufWriter};
use std::net::{TcpListener, TcpStream};
use std::thread;
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

fn client_core(stream: TcpStream) -> Result<(), ClientError> {
	let mut buf_reader = BufReader::new(&stream);
	let mut parser = PullParser::new(&mut buf_reader);

	let mut buf_writer = BufWriter::new(&stream);
	let mut generator = PushGenerator::new(&mut buf_writer);

	let mut msg_id_buf = [0u8; 10];
	let mut command_buf = [0u8; 10];

	while let Some(mut message) = try!{parser.get_message()} {
		let msg_id_len = try!{message.read_field(&mut msg_id_buf)}.unwrap();
		let msg_id = &msg_id_buf[0..msg_id_len];

		let command = match try!{message.read_field(&mut command_buf)} {
			Some(len) => &command_buf[0..len],
			None => {
				try!{generator.write_message(&[&msg_id, b"error", BASIC_STRUCTURE])};
				continue
			}
		};

		match command {
			_ => {
				try!{message.ignore_rest()};
				try!{generator.write_message(&[&msg_id, b"error", b"unknown command"])};
			},
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
