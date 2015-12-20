use std::{mem, io, error};
use std::os::unix::io::AsRawFd;
use unix_socket::UnixStream;
use users;

// Things that should exist in libc:
mod libc {
	pub use libc::*;

	#[cfg(target_os = "linux")]
	#[repr(C)]
	pub struct ucred {
		pub pid: pid_t,
		pub uid: uid_t,
		pub gid: gid_t,
	}

	#[cfg(target_os = "linux")]
	pub const SO_PEERCRED: c_int = 17;
}

fn cvt(v: libc::c_int) -> io::Result<libc::c_int> {
    if v < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(v)
    }
}

pub trait LinuxExt {
	fn get_peer_credentials(&self) -> io::Result<libc::ucred>;
}

pub trait GetPeerUser {
	fn get_peer_uid(&self) -> Result<libc::uid_t, Box<error::Error>>;

	fn get_peer_user(&self) -> Result<users::User, Box<error::Error>> {
		Ok(
			users::get_user_by_uid(
				try!(self.get_peer_uid())
			).unwrap() // TODO .ok_or(an Error)
		)
	}
}

#[cfg(target_os = "linux")]
impl LinuxExt for UnixStream {
	fn get_peer_credentials(&self) -> io::Result<libc::ucred> {
		let fd = self.as_raw_fd();

		unsafe {
			let mut credentials: libc::ucred = mem::zeroed();
			let mut size = mem::size_of::<libc::ucred>() as libc::socklen_t;
			try!(cvt(libc::getsockopt(
				fd,
				libc::SOL_SOCKET,
				libc::SO_PEERCRED,
				&mut credentials as *mut _ as *mut _,
				&mut size as *mut _ as *mut _)));
			Ok(credentials)
		}
	}
}

#[cfg(target_os = "linux")]
impl GetPeerUser for UnixStream {
	fn get_peer_uid(&self) -> Result<libc::uid_t, Box<error::Error>> {
		Ok(try!(self.get_peer_credentials()).uid)
	}
}

