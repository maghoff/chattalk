extern crate plaintalk;
extern crate crossbeam;

mod client;

use std::io::{BufReader,BufWriter};
use std::net::{TcpListener,TcpStream};
use std::thread;
use std::sync::{Arc,Mutex};
use std::sync::mpsc::{channel,Sender};
use plaintalk::pullparser::PullParser;
use plaintalk::pushgenerator::PushGenerator;

fn client_core(stream: TcpStream, tx: Sender<ShoutMessage>) -> Result<(), client::ClientError> {
	let mut buf_reader = BufReader::new(&stream);
	let parser = PullParser::new(&mut buf_reader);

	let buf_writer = BufWriter::new(&stream);
	let generator = Arc::new(Mutex::new(PushGenerator::new(buf_writer)));

	let (tx2, rx2) = channel::<ClientMessage>();
	tx.send(ShoutMessage::Join(tx2.clone())).unwrap();

	let mut client_connection = client::ClientConnection::new(generator.clone(), tx);

	crossbeam::scope(move |scope| {
		scope.spawn(move || {
			while let Ok(message) = rx2.recv() {
				match message {
					ClientMessage::Shout(nick, statement) => {
						let mut generator = generator.lock().unwrap();
						generator.write_message(&[b"*", b"shout", nick.as_bytes(), statement.as_bytes()]).unwrap();
					}
					ClientMessage::Terminate => return
				}
			}
		});

		let result = client_connection.handle_protocol(parser);
		let _ = tx2.send(ClientMessage::Terminate);
		result
	})
	//tx.send(ShoutMessage::Part(tx2)).unwrap();
}

fn handle_client(stream: TcpStream, tx: Sender<ShoutMessage>) {
	let remote = stream.peer_addr().unwrap();
	println!("{} Client connected", remote);

	match client_core(stream, tx) {
		Ok(()) => println!("{} Connection closed", remote),
		Err(e) => println!("{} Connection terminated with error: {:?}", remote, e),
	}
}

pub enum ClientMessage {
	Shout(String, String),
	Terminate,
}

pub enum ShoutMessage {
	Join(Sender<ClientMessage>),
	Shout(String, String),
}

fn main() {
	let listener = TcpListener::bind("127.0.0.1:2203").unwrap();
	println!("Listening to {}", listener.local_addr().unwrap());

	let (tx, rx) = channel();
	thread::spawn(move|| {
		let mut clients = Vec::new();
		while let Ok(message) = rx.recv() {
			match message {
				ShoutMessage::Join(client) => {
					clients.push(client);
				}
				ShoutMessage::Shout(nick, statement) => {
					for client in clients.iter() {
						match client.send(ClientMessage::Shout(nick.clone(), statement.clone())) {
							Ok(()) => (),
							Err(_) => {
								// Ignore errors.
								// TODO: Remove the client from `clients`.
							}
						}
					}
				}
			}
		}
	});

	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				let tx = tx.clone();
				thread::spawn(move || { handle_client(stream, tx) });
			}
			Err(e) => {
				println!("Failed connection attempt: {:?}", e);
			}
		}
	}
}
