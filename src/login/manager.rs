use pam::Client;
use std::{
	collections::HashMap,
	error::Error,
	sync::{Arc, Mutex},
	thread::JoinHandle,
};
#[derive(Clone, Copy)]
pub enum LoginState {
	Logging,
	Failed,
	Authenticated(u32),
}
pub struct LoginManager {
	login_state_map: Arc<Mutex<HashMap<String, LoginState>>>,
}
impl Clone for LoginManager {
	fn clone(&self) -> Self {
		Self {
			login_state_map: Arc::clone(&self.login_state_map),
		}
	}
}
impl LoginManager {
	pub fn new() -> Self {
		Self {
			login_state_map: Default::default(),
		}
	}

	pub fn start_login(
		&self,
		name: impl Into<String>,
		password: impl Into<String>,
		open_session: bool,
	) -> bool {
		let name = name.into();
		let password = password.into();
		{
			let mut login_map_lock = self.login_state_map.lock().unwrap();
			match login_map_lock.get(&name) {
				Some(LoginState::Logging) => return false,
				_ => {
					let login_map = Arc::clone(&self.login_state_map);
					login_map_lock.insert(name.clone(), LoginState::Logging);
					std::thread::spawn(move || {
						let error = || {
							let Ok(mut login_map_lock) = login_map.lock() else {
								return;
							};
							login_map_lock.insert(name.clone(), LoginState::Failed);
							return;
						};
						let mut client = match Client::with_password("login") {
							Ok(client) => client,
							Err(_) => {
								return error();
							}
						};
						client.conversation_mut().set_credentials(&name, &password);
						if let Err(e) = client.authenticate() {
							println!("[ERROR] Failed to authenticate: {e:#?}");
							return error();
						}
						if open_session {
							if let Err(e) = client.open_session() {
								println!("[ERROR] Failed to open PAM session: {e:#?}");
								return error();
							}
						}
						let uid = uzers::get_user_by_name(&name).unwrap().uid();
						let Ok(mut login_map_lock) = login_map.lock() else {
							return;
						};

						println!("[INFO] Logged into {uid}");
						login_map_lock.insert(name.clone(), LoginState::Authenticated(uid));
					});
				}
			}
		}
		true
	}

	pub fn get_current_login_state(&self, name: impl Into<String>) -> Option<LoginState> {
		self.login_state_map.lock().ok()?.get(&name.into()).copied()
	}
}
