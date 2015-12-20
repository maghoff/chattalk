use std::{io, error};

// Backported from libc 0.2.3. unix_socket keeps us at 0.2.2:
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


pub trait PeerCredentials {
	fn get_peer_uid(&self) -> Result<libc::uid_t, Box<error::Error>>;
}


fn cvt(v: libc::c_int) -> io::Result<libc::c_int> {
    if v < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(v)
    }
}

#[cfg(target_os = "linux")]
mod linux {
	use std::{error,io,mem};
	use std::os::unix::io::AsRawFd;
	use unix_socket::UnixStream;
	use super::*;
	use super::{libc,cvt};

	pub trait UCredPeerCredentials {
		fn get_peer_credentials(&self) -> io::Result<libc::ucred>;
	}

	impl UCredPeerCredentials for UnixStream {
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

	impl PeerCredentials for UnixStream {
		fn get_peer_uid(&self) -> Result<libc::uid_t, Box<error::Error>> {
			Ok(try!(self.get_peer_credentials()).uid)
		}
	}
}
