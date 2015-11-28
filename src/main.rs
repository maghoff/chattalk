extern crate plaintalk;

use std::convert;
use std::io::{self,Read,Write,BufReader,BufWriter};
use std::net::{TcpListener, TcpStream};
use std::thread;
use plaintalk::pullparser::PullParser;
use plaintalk::pushgenerator::PushGenerator;

#[derive(Debug)]
enum ClientError {
	Io(io::Error),
	St(&'static str),
	PushGenerator(plaintalk::pushgenerator::Error),
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

fn client_core(stream: TcpStream) -> Result<(), ClientError> {
	let mut buf_reader = BufReader::new(&stream);
	let mut parser = PullParser::new(&mut buf_reader);

	let mut buf_writer = BufWriter::new(&stream);
	let mut generator = PushGenerator::new(&mut buf_writer);

	while let Some(mut message) = try!{parser.get_message()} {
		let mut reply = try!{generator.next_message()};
		while let Some(mut field) = try!{message.get_field()} {
			let mut buffer = String::new();
			try!{field.read_to_string(&mut buffer)};
			try!{try!{reply.next_field()}.write_all(&buffer.into_bytes())};
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
