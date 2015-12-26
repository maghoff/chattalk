extern crate crossbeam;
extern crate libc;
extern crate plaintalk;
extern crate unix_socket;
extern crate users;

mod client;
mod peer_credentials;
mod server;

use std::fs;
use std::io::{Read,Write};
use std::net::TcpListener;
use std::sync::mpsc::{channel,Sender};
use std::thread;
use peer_credentials::PeerCredentials;
use server::ShoutMessage;
use unix_socket::UnixListener;

fn connect_client<R: Read, W: Write+Send>(read: R, write: W, remote: &str, tx: Sender<ShoutMessage>) {
	println!("{} Client connected", remote);

	match client::client(read, write, tx) {
		Ok(()) => println!("{} Connection closed", remote),
		Err(e) => println!("{} Connection terminated with error: {:?}", remote, e),
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
					connect_client(&stream, &stream, &remote, tx)
				});
			}
			Err(e) => {
				println!("Failed connection attempt: {:?}", e);
			}
		}
	}
}

fn unix_acceptor(tx : Sender<ShoutMessage>) {
	let _ = fs::remove_file("socket");
	let listener = UnixListener::bind("socket").unwrap();
	println!("Listening to {:?}", listener.local_addr().unwrap());

	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				let tx = tx.clone();
				thread::spawn(move || {
					let remote_uid = stream.get_peer_uid().unwrap();
					let remote = match users::get_user_by_uid(remote_uid) {
						Some(ucred) => format!("{}", ucred.name),
						None => format!("{}", remote_uid),
					};
					connect_client(&stream, &stream, &remote, tx)
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
	server::server(rx);
}
