use std::fs;
use std::sync::mpsc::Sender;
use std::thread;
use client::ProtocolExtensions;
use peer_credentials::PeerCredentials;
use server::ShoutMessage;
use unix_socket::{UnixStream, UnixListener};
use users;

struct UnixProtocolExtensions<'a> {
	stream: &'a UnixStream,
}

impl<'a> UnixProtocolExtensions<'a> {
	fn new(stream: &'a UnixStream) -> UnixProtocolExtensions<'a> {
		UnixProtocolExtensions {
			stream: stream,
		}
	}
}

impl<'a> ProtocolExtensions for UnixProtocolExtensions<'a> {
	fn supports_auth_unix(&self) -> bool { true }

	fn auth_unix(&self) -> Option<String> {
		let uid = self.stream.get_peer_uid().unwrap();
		users::get_user_by_uid(uid).map(|ucred| ucred.name)
	}
}

pub fn acceptor(tx : Sender<ShoutMessage>) {
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
					let protocol_extensions = UnixProtocolExtensions::new(&stream);
					super::connect_client(&stream, &stream, protocol_extensions, &remote, tx)
				});
			}
			Err(e) => {
				println!("Failed connection attempt: {:?}", e);
			}
		}
	}
}
