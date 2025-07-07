use crate::login::LoginManager;
use crate::tty::*;
use color_eyre::eyre::bail;
use color_eyre::eyre::OptionExt;
use freedesktop_entry_parser::parse_entry;
use nix::libc;
use nix::libc::*;
use rustamarine::Rustamarine;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::ffi::CStr;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::process::Command;
use std::rc::Rc;
#[derive(Debug)]
pub struct DesktopEnvironmentFile {
	name: String,
	command: String,
}
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SessionStatus {
	Running,
	ShutdownGracefully,
	Crashed,
}
pub struct Session {
	process: RefCell<Child>,
	tty: TTYInfo,
	user_id: u32,
}

impl Session {
	fn new(
		uid: u32,
		tty: TTYInfo,
		session_file: &DesktopEnvironmentFile,
		rmar: &mut Rustamarine,
	) -> color_eyre::Result<Session> {
		tty.make_current(rmar);
		let process = RefCell::new(
			Command::new("bash")
				.uid(uid)
				.args(["-c", &session_file.command])
				.spawn()?,
		);
		Ok(Self {
			process,
			tty,
			user_id: uid,
		})
	}
	pub fn status(&self) -> SessionStatus {
		match self.process.borrow_mut().try_wait() {
			Ok(Some(code)) if code.success() => SessionStatus::ShutdownGracefully,
			Ok(Some(_)) => SessionStatus::Crashed,
			Ok(None) => SessionStatus::Running,
			Err(_) => SessionStatus::Crashed,
		}
	}
}

impl Drop for Session {
	fn drop(&mut self) {
		match self.status() {
			SessionStatus::Running => {
				self.process.borrow_mut().kill().ok();
				let current_tty = TTYInfo::get_active_tty_number();
				if self.tty.number == current_tty {
					println!("[WARN] Dropped session while still inside the session's tty: {current_tty}");
				}
			}
			_ => {}
		}
	}
}

pub struct SessionManager {
	sessions: HashMap<u32, Rc<Session>>,
	tibs_tty: u16,
	wayland_desktop_environments_cache: Vec<DesktopEnvironmentFile>,
}

impl SessionManager {
	fn discover_wayland_desktop_environments() -> Vec<DesktopEnvironmentFile> {
		let session_dirs = env::var("XDG_SESSION_DIRS")
			.map(|v| v.split(':').map(String::from).collect::<Vec<_>>())
			.unwrap_or_else(|_| {
				vec![
					"/usr/share/wayland-sessions".into(),
					"/run/current-system/sw/share/wayland-sessions".into(),
				]
			});

		session_dirs
			.iter()
			.filter_map(|dir| fs::read_dir(dir).ok())
			.flat_map(|entries| entries.filter_map(Result::ok))
			.filter(|entry| {
				entry
					.path()
					.extension()
					.map(|e| e == "desktop")
					.unwrap_or(false)
			})
			.filter_map(|entry| {
				let path = entry.path();
				let entry = parse_entry(&path).ok()?;
				let section = entry.section("Desktop Entry");
				let name = section.attr("Name")?.to_string();
				let command = section.attr("Exec")?.to_string();
				Some(DesktopEnvironmentFile { name, command })
			})
			.collect()
	}
	pub fn update_desktop_environments_cache(&mut self) {
		self.wayland_desktop_environments_cache = Self::discover_wayland_desktop_environments();
	}
	pub fn get_desktop_environments_list(&self) -> &[DesktopEnvironmentFile] {
		&self.wayland_desktop_environments_cache
	}
	pub fn new() -> Self {
		Self {
			sessions: Default::default(),
			tibs_tty: TTYInfo::get_active_tty_number(),
			wayland_desktop_environments_cache: Self::discover_wayland_desktop_environments(),
		}
	}

	fn next_tty(&self) -> Option<TTYInfo> {
		let used_ttys = self
			.sessions
			.values()
			.filter(|s| matches!(s.status(), SessionStatus::Running))
			.map(|s| s.tty.number)
			.collect::<HashSet<_>>();
		(0..63u16)
			.into_iter()
			.find_map(|i| (i != self.tibs_tty && !used_ttys.contains(&i)).then(|| TTYInfo::new(i)))
			.flatten()
	}
	pub fn start_session(
		&mut self,
		login_manager: &LoginManager,
		username: &str,
		session_file: &DesktopEnvironmentFile,
		rmar: &mut Rustamarine,
	) -> color_eyre::Result<Rc<Session>> {
		let Some(crate::login::LoginState::Authenticated(uid)) =
			login_manager.get_current_login_state(username)
		else {
			bail!("Tried to start session without being authenticated (user={username})");
		};
		let free_tty = self
			.next_tty()
			.ok_or_eyre("There's no free tty's left for this session.")?;
		let session = Session::new(uid, free_tty, session_file, rmar).map(Rc::new)?;
		self.sessions.insert(uid, Rc::clone(&session));
		Ok(session)
	}
	pub fn get_session_state_of_user(&self, uid: u32) -> Option<SessionStatus> {
		self.sessions.get(&uid).map(|s| s.status())
	}
	pub fn is_running(&self, uid: u32) -> bool {
		self.sessions.get(&uid).is_some_and(|s| matches!(s.status(), SessionStatus::Running))
	}
	pub fn has_crashed(&self, uid: u32) -> bool {
		self.sessions.get(&uid).is_some_and(|s| matches!(s.status(), SessionStatus::Crashed))
	}
}
