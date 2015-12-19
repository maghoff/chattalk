extern crate plaintalk;
extern crate crossbeam;
extern crate unix_socket;

mod client;
mod client_error;
mod protocol_error;

use std::io::{BufReader,BufWriter,Read,Write};
use std::net::TcpListener;
use std::sync::mpsc::{channel,Sender,Receiver};
use std::sync::{Arc,Mutex};
use std::thread;
use plaintalk::pullparser::PullParser;
use plaintalk::pushgenerator::PushGenerator;
use unix_socket::UnixListener;

fn client_core<R: Read, W: Write+Send>(read: R, write: W, tx: Sender<ShoutMessage>) -> Result<(), client::ClientError> {
	let mut buf_reader = BufReader::new(read);
	let parser = PullParser::new(&mut buf_reader);

	let buf_writer = BufWriter::new(write);
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

fn handle_client<R: Read, W: Write+Send>(read: R, write: W, remote: &str, tx: Sender<ShoutMessage>) {
	//let remote = stream.peer_addr().unwrap();
	println!("{} Client connected", remote);

	match client_core(read, write, tx) {
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

fn server(rx : Receiver<ShoutMessage>) {
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
}

fn tcp_acceptor(tx : Sender<ShoutMessage>) {
	let listener = TcpListener::bind("127.0.0.1:2203").unwrap();
	println!("Listening to {}", listener.local_addr().unwrap());

	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				let tx = tx.clone();
				thread::spawn(move || {
					let remote = format!("{}", stream.peer_addr().unwrap());
					handle_client(&stream, &stream, &remote, tx)
				});
			}
			Err(e) => {
				println!("Failed connection attempt: {:?}", e);
			}
		}
	}
}

fn unix_acceptor(tx : Sender<ShoutMessage>) {
	let listener = UnixListener::bind("socket").unwrap();
	println!("Listening to {:?}", listener.local_addr().unwrap());

	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				let tx = tx.clone();
				thread::spawn(move || {
					let remote = format!("{:?}", stream.peer_addr().unwrap());
					handle_client(&stream, &stream, &remote, tx)
				});
			}
			Err(e) => {
				println!("Failed connection attempt: {:?}", e);
			}
		}
	}
}

fn main() {
	let (tx, rx) = channel();
	{
		let tx = tx.clone();
		thread::spawn(move || tcp_acceptor(tx));
	}
	thread::spawn(move || unix_acceptor(tx));
	server(rx);
}
