extern crate plaintalk;

mod client;

use std::io::{BufReader,BufWriter};
use std::net::{TcpListener,TcpStream};
use std::thread;
use plaintalk::pullparser::PullParser;
use plaintalk::pushgenerator::PushGenerator;

fn client_core(stream: TcpStream) -> Result<(), client::ClientError> {
	let mut buf_reader = BufReader::new(&stream);
	let parser = PullParser::new(&mut buf_reader);

	let mut buf_writer = BufWriter::new(&stream);
	let generator = PushGenerator::new(&mut buf_writer);

	let mut client_connection = client::ClientConnection::new(generator);

	client_connection.handle_protocol(parser)
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
