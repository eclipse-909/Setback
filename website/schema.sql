create table if not exists Accounts (
	account_id integer primary key, --alias for rowid
	username text not null unique,
	password text not null,
	last_active date not null default date('now'),
	is_admin boolean not null default false
)