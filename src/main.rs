extern crate crossbeam;
extern crate libc;
extern crate plaintalk;
extern crate unix_socket;
extern crate users;

mod client;
mod peer_credentials;
mod server;
mod tcp;
mod unix;

use std::io::{Read,Write};
use std::sync::mpsc::{channel,Sender};
use std::thread;
use client::ProtocolExtensions;
use server::ShoutMessage;

pub fn connect_client
	<R: Read, W: Write+Send, P: ProtocolExtensions>
	(read: R, write: W, protocol_extensions: P, remote: &str, tx: Sender<ShoutMessage>
) {
	println!("{} Client connected", remote);

	match client::client(read, write, protocol_extensions, tx) {
		Ok(()) => println!("{} Connection closed", remote),
		Err(e) => println!("{} Connection terminated with error: {:?}", remote, e),
	}
}

fn main() {
	let (tx, rx) = channel();
	{
		let tx = tx.clone();
		thread::spawn(move || tcp::acceptor(tx));
	}
	thread::spawn(move || unix::acceptor(tx));
	server::server(rx);
}
