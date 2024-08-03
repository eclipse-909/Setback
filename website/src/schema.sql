create table if not exists Accounts (
	account_id integer primary key, --alias for rowid
	email text not null unique,
	username text not null unique,
	password text not null,
	last_active date not null
)