extern crate plaintalk;

use std::convert;
use std::io::{self,Read,Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use plaintalk::pullparser::PullParser;

#[derive(Debug)]
enum ClientError {
	Io(io::Error),
	St(&'static str),
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

fn client_core(mut stream: TcpStream) -> Result<(), ClientError> {
	let mut parser = PullParser::new(&mut stream);

	while let Some(mut message) = try!{parser.get_message()} {
		while let Some(mut field) = try!{message.get_field()} {
			let mut buffer = String::new();
			try!{field.read_to_string(&mut buffer)};
			println!("Field: {}", &buffer);
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
