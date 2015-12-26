use std::sync::mpsc::{Sender, Receiver};
use client::ClientMessage;

pub enum ShoutMessage {
	Join(Sender<ClientMessage>),
	Shout(String, String),
}

pub fn server(rx : Receiver<ShoutMessage>) {
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
