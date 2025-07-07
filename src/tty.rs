use nix::libc;
use rustamarine::Rustamarine;
use std::{
	fs::{File, OpenOptions},
	mem::MaybeUninit,
	os::fd::AsRawFd,
};

pub struct TTYInfo {
	pub fd: File,
	pub number: u16,
}
impl TTYInfo {
	pub fn new(i: u16) -> Option<Self> {
		OpenOptions::new()
			.read(true)
			.write(true)
			.open(&format!("/dev/tty{i}"))
			.ok()
			.map(|f| TTYInfo { fd: f, number: i })
	}
	pub fn make_current(&self, rmar: &mut Rustamarine) {
		rmar.go_to_tty(self.number);
	}

	pub fn get_active_tty_number() -> u16 {
		const VT_GETSTATE: libc::c_ulong = 0x5603;
		#[repr(C)]
		#[derive(Debug)]
		struct vt_stat {
			v_active: libc::c_ushort,
			v_signal: libc::c_ushort,
			v_state: libc::c_ushort,
		}
		let Ok(file) = File::open("/dev/console") else {
			return 2;
		};
		let fd = file.as_raw_fd();
		let mut vt: MaybeUninit<vt_stat> = MaybeUninit::uninit();
		unsafe { libc::ioctl(fd, VT_GETSTATE, vt.as_mut_ptr()) };
		unsafe { vt.assume_init().v_active }
	}
}
