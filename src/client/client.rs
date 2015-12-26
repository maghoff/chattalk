extern crate plaintalk;

use std::sync::{Arc,Mutex};
use std::sync::mpsc::Sender;
use std::convert;
use std::io::{Read,Write};
use plaintalk::pullparser::{self,PullParser};
use plaintalk::pushgenerator::PushGenerator;
use super::client_error::ClientError;
use super::protocol_error::ProtocolError;
use server::ShoutMessage;


pub trait ProtocolExtensions {
	fn supports_auth_unix(&self) -> bool;
	fn auth_unix(&self) -> Option<String>;
}


fn expect<T, E>(field: Result<Option<T>, E>, err: &'static [u8]) -> Result<T, ProtocolError>
	where ProtocolError : convert::From<E>
{
	try!(field).ok_or(ProtocolError::InvalidCommand(err))
}

fn expect_end(message: &pullparser::Message, err: &'static [u8]) -> Result<(), ProtocolError> {
	match message.at_end() {
		true => Ok(()),
		false => Err(ProtocolError::InvalidCommand(err))
	}
}


pub struct ClientConnection<T: Write, P: ProtocolExtensions> {
	nick: String,
	generator: Arc<Mutex<PushGenerator<T>>>,
	protocol_extensions: P,
	tx: Sender<ShoutMessage>,
}

impl<T: Write, P: ProtocolExtensions> ClientConnection<T, P> {
	pub fn new(generator: Arc<Mutex<PushGenerator<T>>>, protocol_extensions: P, tx: Sender<ShoutMessage>) -> ClientConnection<T, P> {
		ClientConnection {
			nick: String::new(),
			generator: generator,
			protocol_extensions: protocol_extensions,
			tx: tx,
		}
	}

	fn cmd_help(&mut self, msg_id: &[u8], message: &mut pullparser::Message) -> Result<(), ProtocolError> {
		static USAGE:&'static[u8] = b"Usage: <msg-id> help";

		static HELP:&'static[u8] =
			b"Available commands:\n\
			<msgid> protocol {<feature> ...}    Protocol negotiation (not implemented)\n\
			<msgid> join <channel>              Join (not implemented)\n\
			<msgid> nick <new nick>             Set your nick to <new nick>\n\
			<msgid> shout <statement>           Shout a statement to all connected clients";

		try!{expect_end(&message, USAGE)};

		let mut generator = try!{self.generator.lock()};
		try!{generator.write_message(&[b"*", b"note", HELP])};
		try!{generator.write_message(&[msg_id, b"ok"])};

		Ok(())
	}

	fn cmd_protocol(&mut self, msg_id: &[u8], message: &mut pullparser::Message) -> Result<(), ProtocolError> {
		try!{message.ignore_rest()};
		let mut generator = try!{self.generator.lock()};
		try!{generator.write_message(&[b"*", b"note", b"'protocol' currently has no effect"])};
		try!{generator.write_message(&[msg_id, b"ok", b"chattalk"])};

		Ok(())
	}

	fn cmd_join(&mut self, msg_id: &[u8], message: &mut pullparser::Message) -> Result<(), ProtocolError> {
		static USAGE:&'static[u8] = b"Usage: <msg-id> join <channel-name>";

		let channel = try!{expect(message.read_field_as_string(), USAGE)};
		try!{expect_end(&message, USAGE)};

		let mut generator = try!{self.generator.lock()};
		try!{generator.write_message(&[b"*", b"note", b"'join' currently has no effect"])};
		try!{generator.write_message(&[b"*", b"join", &self.nick.as_bytes(), &channel.into_bytes()])};
		try!{generator.write_message(&[msg_id, b"ok"])};

		Ok(())
	}

	fn cmd_nick(&mut self, msg_id: &[u8], message: &mut pullparser::Message) -> Result<(), ProtocolError> {
		static USAGE:&'static[u8] = b"Usage: <msg-id> nick <new-nick>";

		let new_nick = try!{expect(message.read_field_as_string(), USAGE)};
		try!{expect_end(&message, USAGE)};

		let mut generator = try!{self.generator.lock()};
		try!{generator.write_message(&[b"*", b"nick", &self.nick.as_bytes(), &new_nick.as_bytes()])};
		self.nick = new_nick;

		try!{generator.write_message(&[msg_id, b"ok"])};

		Ok(())
	}

	fn cmd_shout(&mut self, msg_id: &[u8], message: &mut pullparser::Message) -> Result<(), ProtocolError> {
		static USAGE:&'static[u8] = b"Usage: <msg-id> shout <statement>";

		let statement = try!{expect(message.read_field_as_string(), USAGE)};
		try!{expect_end(&message, USAGE)};

		let mut generator = try!{self.generator.lock()};
		try!{self.tx.send(ShoutMessage::Shout(self.nick.clone(), statement))};
		try!{generator.write_message(&[msg_id, b"ok"])};

		Ok(())
	}

	fn cmd_auth(&mut self, msg_id: &[u8], message: &mut pullparser::Message) -> Result<(), ProtocolError> {
		static USAGE:&'static[u8] = b"Usage: <msg-id> auth <auth-method> ...";

		let mut auth_method_buf = [0u8; 10];
		match try!{expect(message.read_field_as_slice(&mut auth_method_buf), USAGE)} {
			b"unix" => {
				static USAGE:&'static[u8] = b"Usage: <msg-id> auth unix";
				try!{expect_end(&message, USAGE)};

				match self.protocol_extensions.auth_unix() {
					Some(user) => {
						let mut generator = try!{self.generator.lock()};
						self.nick = user;
						try!{generator.write_message(&[
							msg_id, b"ok", &self.nick.as_bytes()
						])};
					}
					None => {
						let mut generator = try!{self.generator.lock()};
						try!{generator.write_message(&[
							msg_id, b"error", b"auth-failed", b"Unix authentication failed"
						])};
					}
				}
			},
			method => {
				try!{message.ignore_rest()};
				let mut generator = try!{self.generator.lock()};
				try!{generator.write_message(&[
					msg_id, b"error", b"unknown-method",
					&format!("unknown authentication method: {}", String::from_utf8_lossy(method)).into_bytes()
				])};
			}
		}

		Ok(())
	}

	fn handle_message(
		&mut self,
		msg_id: &[u8],
		message: &mut pullparser::Message,
	) ->
		Result<(), ProtocolError>
	{
		static BASIC_STRUCTURE:&'static[u8] =
			b"Invalid format. Basic structure of all messages is: \
			<message-ID> <command> [command arguments...] \
			(try `0 help`)";

		let mut command_buf = [0u8; 10];

		match try!{expect(message.read_field_as_slice(&mut command_buf), BASIC_STRUCTURE)} {
			b"auth"     => self.cmd_auth(msg_id, message),
			b"help"     => self.cmd_help(msg_id, message),
			b"join"     => self.cmd_join(msg_id, message),
			b"nick"     => self.cmd_nick(msg_id, message),
			b"protocol" => self.cmd_protocol(msg_id, message),
			b"shout"    => self.cmd_shout(msg_id, message),
			command => {
				try!{message.ignore_rest()};
				let mut generator = try!{self.generator.lock()};
				try!{generator.write_message(&[
					msg_id, b"error", b"invalid-command",
					&format!("unknown command: {}", String::from_utf8_lossy(command)).into_bytes()
				])};
				Ok(())
			},
		}
	}

	pub fn handle_protocol<R: Read>(&mut self, mut parser: PullParser<R>) -> Result<(), ClientError> {
		let mut msg_id_buf = [0u8; 10];

		while let Some(mut message) = try!{parser.get_message()} {
			let msg_id = try!{message.read_field_as_slice(&mut msg_id_buf)}
				.expect("PlainTalk parser yielded a message with zero fields");

			if msg_id == b"" && message.at_end() { break }

			match self.handle_message(msg_id, &mut message) {
				Ok(()) => (),
				Err(ProtocolError::InvalidCommand(usage)) => {
					let mut generator = try!{self.generator.lock()};
					try!{generator.write_message(
						&[msg_id, b"error", b"invalid-command", usage])};
					try!{message.ignore_rest()};
				},
				Err(ProtocolError::PlaintalkError(err)) => return Err(err),
			}
		}

		Ok(())
	}
}

use std::io::{BufReader, BufWriter};
use std::sync::mpsc::channel;
use crossbeam;

pub enum ClientMessage {
	Shout(String, String),
	Terminate,
}

pub fn client
	<R: Read, W: Write+Send, P: ProtocolExtensions>
	(read: R, write: W, protocol_extensions: P, tx: Sender<ShoutMessage>)
	-> Result<(), ClientError>
{
	let parser = PullParser::new(BufReader::new(read));

	let buf_writer = BufWriter::new(write);
	let generator = Arc::new(Mutex::new(PushGenerator::new(buf_writer)));

	let (tx2, rx2) = channel::<ClientMessage>();
	tx.send(ShoutMessage::Join(tx2.clone())).unwrap();

	let mut client_connection = ClientConnection::new(generator.clone(), protocol_extensions, tx);

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
}
