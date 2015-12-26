use std::sync::mpsc::Sender;
use std::thread;
use server::ShoutMessage;
use std::net::TcpListener;

pub fn acceptor(tx : Sender<ShoutMessage>) {
	let listener = TcpListener::bind("127.0.0.1:2203").unwrap();
	println!("Listening to {}", listener.local_addr().unwrap());

	for stream in listener.incoming() {
		match stream {
			Ok(stream) => {
				let tx = tx.clone();
				thread::spawn(move || {
					let remote = format!("{}", stream.peer_addr().unwrap());
					super::connect_client(&stream, &stream, &remote, tx)
				});
			}
			Err(e) => {
				println!("Failed connection attempt: {:?}", e);
			}
		}
	}
}
