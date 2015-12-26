use std::fs;
use std::sync::mpsc::Sender;
use std::thread;
use peer_credentials::PeerCredentials;
use server::ShoutMessage;
use users;
use unix_socket::UnixListener;

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
					super::connect_client(&stream, &stream, &remote, tx)
				});
			}
			Err(e) => {
				println!("Failed connection attempt: {:?}", e);
			}
		}
	}
}
