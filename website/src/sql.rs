use {
	sqlx::{
		MySql, Sqlite, Error, Pool,
		mysql::MySqlPool,
		sqlite::SqlitePool,
	},
	chrono::NaiveDate
};

#[cfg(debug_assertions)]
pub type Database = SqliteDatabase;

#[cfg(not(debug_assertions))]
pub type Database = MySqlDatabase;

pub enum SqlError {
	SqlXError(Error),
	Custom(String)
}

impl From<Error> for SqlError {
	fn from(value: Error) -> Self {
		SqlError::SqlXError(value)
	}
}

#[derive(sqlx::FromRow)]
struct Account {
	account_id: u32,
	username: String,
	password: String,
	last_active: NaiveDate
}

pub trait DB: Sized {
	const SCHEMA: &'static str = include_str!("../schema.sql");
	type SqlxDatabase: sqlx::Database;
	async fn connect(database_url: &str) -> Result<Self, Error>;
	async fn create_account(&self, username: String, password: String) -> Result<(), Error>;
	async fn renew_activity(&self, account_id: u32) -> Result<(), Error>;
	async fn verify_login(&self, username: String, password: String) -> Result<u32, SqlError>;
	async fn update_username(&self, account_id: u32, username: String) -> Result<(), Error>;
	async fn update_password(&self, account_id: u32, password: String) -> Result<(), Error>;
	async fn delete_account(&self, account_id: u32) -> Result<(), Error>;
	async fn delete_inactive_accounts(&self) -> Result<u64, Error>;
	fn get_pool(&self) -> &Pool<Self::SqlxDatabase>;
	async fn create_if_not_exists(&self) -> Result<(), Error>;
}

pub struct MySqlDatabase {pool: MySqlPool}

impl DB for MySqlDatabase {
	type SqlxDatabase = MySql;
	
	async fn connect(database_url: &str) -> Result<Self, Error> {
		Ok(Self {pool: MySqlPool::connect(database_url).await?})
	}
	
	async fn create_account(&self, username: String, password: String) -> Result<(), Error> {
		sqlx::query("insert into Accounts (username, password, last_active) values (?, ?, CURRENT_DATE())")
			.bind(username)
			.bind(password)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	
	async fn renew_activity(&self, account_id: u32) -> Result<(), Error> {
		sqlx::query("update Accounts set last_active = CURRENT_DATE() where account_id = ?")
			.bind(account_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	
	async fn verify_login(&self, username: String, password: String) -> Result<u32, SqlError> {
		let account: Account = sqlx::query_as::<_, Account>("select * from Accounts where username = ?")
			.bind(username)
			.fetch_one(&self.pool)
			.await?;
		if account.password != password {
			return Err(SqlError::Custom(String::from("Invalid username or password")));
		}
		Ok(account.account_id)
	}
	
	async fn update_username(&self, account_id: u32, username: String) -> Result<(), Error> {
		sqlx::query("update Accounts set username = ? where account_id = ?")
			.bind(username)
			.bind(account_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	
	async fn update_password(&self, account_id: u32, password: String) -> Result<(), Error> {
		sqlx::query("update Accounts set password = ? where account_id = ?")
			.bind(password)
			.bind(account_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	
	async fn delete_account(&self, account_id: u32) -> Result<(), Error> {
		sqlx::query("delete from Accounts where account_id = ?")
			.bind(account_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	
	async fn delete_inactive_accounts(&self) -> Result<u64, Error> {
		//TODO use query! macro when I've created the actual database and updated the .env.release
		Ok(sqlx::query("delete from Accounts where last_active <= DATE_SUB(CURDATE(), INTERVAL 5 YEAR)")
			.execute(&self.pool)
			.await?.rows_affected()
		)
	}
	
	fn get_pool(&self) -> &Pool<Self::SqlxDatabase> {&self.pool}
	
	async fn create_if_not_exists(&self) -> Result<(), Error> {
		sqlx::query::<Self::SqlxDatabase>(Self::SCHEMA)
			.execute(self.get_pool())
			.await?;
		Ok(())
	}
}

pub struct SqliteDatabase {pool: SqlitePool}

impl DB for SqliteDatabase {
	type SqlxDatabase = Sqlite;
	
	async fn connect(database_url: &str) -> Result<Self, Error> {
		Ok(Self {pool: SqlitePool::connect(database_url).await?})
	}
	
	async fn create_account(&self, username: String, password: String) -> Result<(), Error> {
		sqlx::query("insert into Accounts (username, password, last_active) values (?1, ?2, CURRENT_DATE)")
			.bind(username)
			.bind(password)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	
	async fn renew_activity(&self, account_id: u32) -> Result<(), Error> {
		sqlx::query("update Accounts set last_active = CURRENT_DATE where account_id = ?1")
			.bind(account_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	
	async fn verify_login(&self, username: String, password: String) -> Result<u32, SqlError> {
		let account: Account = sqlx::query_as::<_, Account>("select * from Accounts where username = ?1")
			.bind(username)
			.fetch_one(&self.pool)
			.await?;
		if account.password != password {
			return Err(SqlError::Custom(String::from("Invalid username or password")));
		}
		Ok(account.account_id)
	}
	
	async fn update_username(&self, account_id: u32, username: String) -> Result<(), Error> {
		sqlx::query("update Accounts set username = ?1 where account_id = ?2")
			.bind(username)
			.bind(account_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	
	async fn update_password(&self, account_id: u32, password: String) -> Result<(), Error> {
		sqlx::query("update Accounts set password = ?1 where account_id = ?2")
			.bind(password)
			.bind(account_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}

    async fn update_password(&self, account_id: u32, password: String) -> Result<(), Error> {
		sqlx::query(format!("update Accounts set password = {} where account_id = {}", password, account_id))
			.execute(&self.pool)
			.await?;
		Ok(())
	}

	async fn delete_account(&self, account_id: u32) -> Result<(), Error> {
		sqlx::query("delete from Accounts where account_id = ?1")
			.bind(account_id)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
	
	async fn delete_inactive_accounts(&self) -> Result<u64, Error> {
		//TODO use query! macro when I've created the actual database and updated the .env.release
		Ok(sqlx::query("delete from Accounts where last_active <= date('now', '-5 years')")
			.execute(&self.pool)
			.await?.rows_affected()
		)
	}
	
	fn get_pool(&self) -> &Pool<Self::SqlxDatabase> {&self.pool}
	
	async fn create_if_not_exists(&self) -> Result<(), Error> {
		sqlx::query::<Self::SqlxDatabase>(Self::SCHEMA)
			.execute(self.get_pool())
			.await?;
		Ok(())
	}
}
