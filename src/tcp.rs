use std::sync::mpsc::Sender;
use std::thread;
use client::ProtocolExtensions;
use server::ShoutMessage;
use std::net::TcpListener;

struct TcpProtocolExtensions;

impl ProtocolExtensions for TcpProtocolExtensions {
	fn supports_auth_unix(&self) -> bool { false }
	fn auth_unix(&self) -> Option<String> { None }
}

pub fn acceptor(tx : Sender<ShoutMessage>) {
	let listener = TcpListener::bind("127.0.0.1:2203").unwrap();
	println!("Listening to {}", listener.local_addr().unwrap());

	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				let tx = tx.clone();
				thread::spawn(move || {
					let remote = format!("{}", stream.peer_addr().unwrap());
					super::connect_client(&stream, &stream, TcpProtocolExtensions, &remote, tx)
				});
			}
			Err(e) => {
				println!("Failed connection attempt: {:?}", e);
			}
		}
	}
}
