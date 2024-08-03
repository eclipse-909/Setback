use {
	rusqlite::{Error, params},
	resources::website_items::*,
	argon2::{
		password_hash::{
			rand_core::OsRng,
			PasswordHash, PasswordHasher, PasswordVerifier, SaltString
		},
		Argon2
	},
	r2d2::{Pool, PooledConnection},
	r2d2_sqlite::SqliteConnectionManager
};

pub struct Database {pool: Pool<SqliteConnectionManager>}

impl Database {
	pub fn connect() -> Self {
		let db = Database {pool: Pool::new(SqliteConnectionManager::file("setback.db")).unwrap()};
		db.get_conn().unwrap().execute(include_str!("./schema.sql"), []).unwrap();
		db
	}
	fn get_conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {self.pool.get().map_err(|_| "System busy, try again later".to_string())}
	pub fn create_account(&self, email: String, username: String, password: String) -> Result<u32, String> {
		let conn: PooledConnection<SqliteConnectionManager> = self.get_conn()?;
		match conn.prepare("select account_id from Account where email = ?1").unwrap().query_row([email.clone()], |_| {Ok(())}) {
			Err(Error::QueryReturnedNoRows) => {}
			Err(_) => {panic!();}
			Ok(()) => {return Err("Error email already exists".to_string());}
		}
		match conn.prepare("select account_id from Account where username = ?1").unwrap().query_row([username.clone()], |_| {Ok(())}) {
			Err(Error::QueryReturnedNoRows) => {}
			Err(_) => {panic!();}
			Ok(()) => {return Err("Error username already exists".to_string());}
		}
		
		todo!("verify the email is valid");
		
		conn.execute(
			"insert into Accounts (email, username, password, last_active) values (?1, ?2, ?3, CURRENT_DATE)",
			[email.clone(), username.clone(), hash(&password)?]
		).unwrap();
		Ok(conn.prepare("select account_id from Account where email = ?1 and username = ?2").unwrap().query_row([email, username], |row| {row.get(0)}).unwrap())
	}
	pub fn get_account(&self, account_id: u32) -> Result<AccountData, String> {
		self.get_conn()?
			.prepare("select account_id, email, username from Accounts where account_id = ?1").unwrap()
			.query_row([account_id], |row| {
				Ok(AccountData {
					account_id,
					email: row.get(1)?,
					username: row.get(2)?,
					//password and last_active should never be used or needed
					password: "".to_string(),
					last_active: "".to_string()
				}
			)})
			.map_err(|_| {"Account not found".to_string()})
	}
	pub fn renew_activity(&self, account_id: u32) -> Result<(), String> {
		let conn: PooledConnection<SqliteConnectionManager> = self.get_conn()?;
		conn.execute("update Accounts set last_active = CURRENT_DATE where account_id = ?1", [account_id]).unwrap();
		Ok(())
	}
	pub fn verify_login(&self, username: String, password: String) -> Result<u32, String> {
		let (account_id, password_hash) = self.get_conn()?
			.prepare("select account_id, password from Accounts where username = ?1").unwrap()
			.query_row([username], |row| {Ok((row.get(0)?, row.get(1)?))})
			.map_err(|_| "Username not found".to_string())?;
		if !verify_password(password, password_hash)? {return Err("Incorrect username or password".to_string());}
		self.renew_activity(account_id)?;
		Ok(account_id)
	}
	pub fn update_email(&self, account_id: u32, email: String) -> Result<(), String> {
		todo!("verify the email is valid");
		
		Ok(assert_eq!(self.get_conn()?.execute("update Accounts set email = ?1 where account_id = ?2", params![account_id, email]).unwrap(), 1))
	}
	pub fn update_username(&self, account_id: u32, username: String) -> Result<(), String> {
		Ok(assert_eq!(self.get_conn()?.execute("update Accounts set username = ?1 where account_id = ?2", params![account_id, username]).unwrap(), 1))
	}
	pub fn update_password(&self, account_id: u32, password: String) -> Result<(), String> {
		Ok(assert_eq!(self.get_conn()?.execute("update Accounts set password = ?1 where account_id = ?2", params![account_id, hash(&password)?]).unwrap(), 1))
	}
	pub fn delete_account(&self, account_id: u32) -> Result<(), String> {
		Ok(assert_eq!(self.get_conn()?.execute("delete from Accounts where account_id = ?1", [account_id]).unwrap(), 1))
	}
	pub fn delete_inactive_accounts(&self) -> Result<usize, String> {
		Ok(self.get_conn()?.execute("delete from Accounts where last_active <= date('now', '-5 years')", []).unwrap())
	}
}

fn hash(password: &String) -> Result<String, String> {
	Ok(Argon2::default().hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng)).map_err(|_| "Invalid password".to_string())?.to_string())
}
fn verify_password(password: String, password_hash: String) -> Result<bool, String> {
	Ok(Argon2::default().verify_password(password.as_bytes(), &PasswordHash::new(&password_hash).map_err(|_| "Invalid password".to_string())?).is_ok())
}

#[cfg(test)]
mod test_hash {
	use super::{hash, verify_password};
	#[test]
	fn test_password_hash() {
		let password: String = "wordpass".to_string();
		let password_hash: String = hash(&password).unwrap();
		assert!(verify_password(password, password_hash).unwrap())
	}
}