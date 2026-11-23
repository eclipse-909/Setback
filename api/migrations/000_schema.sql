create type mod_action  as enum('Suspend','Unsuspend','Warn');
create type game_mode   as enum('Sp','Mp');
create type report_type as enum('Stalling','Hacking','Cheating');

create table profile(
	profile_id      integer generated always as identity primary key,
	suspended_at    timestamptz,
	suspended_until timestamptz,
	created_at      timestamptz not null default current_timestamp,

	check (suspended_until is null or (suspended_at is not null and suspended_until >= suspended_at))
);

create table profile_email(
	profile_id       integer      references profile(profile_id),
	email            varchar(255) not null unique,
	email_normalized varchar(255) not null unique,
	active           bool         not null default true,

	primary key (profile_id, email_normalized)
);

create unique index profile_email_index on profile_email(profile_id) where active;

create table profile_ip(
	profile_id integer     not null references profile(profile_id),
	ip         inet        not null,
	first_seen timestamptz not null default current_timestamp,
	last_seen  timestamptz not null default current_timestamp,

	primary key (profile_id, ip)
);

create index profile_ip_index on profile_ip(ip);

create table account(
	account_id       integer generated always as identity primary key,
	profile_id       integer      not null references profile(profile_id),
	username         varchar(15)           unique,
	username_archive varchar(15),
	password_hash    text         not null,
	created_at       timestamptz  not null default current_timestamp,
	last_active      timestamptz  not null default current_timestamp,
	is_admin         bool         not null default false,
	deleted_at       timestamptz,

	unique (profile_id, account_id)
);

create table profile_moderation_log(
	log_id     bigint generated always as identity primary key,
	profile_id integer     not null references profile(profile_id),
	action     mod_action  not null,
	reason     text,
	acted_by   integer              references account(account_id) on delete set null,
	created_at timestamptz not null default current_timestamp,
	expires_at timestamptz
);

create index profile_moderation_log_index on profile_moderation_log(profile_id, created_at);

create table account_stats(
	account_id     integer   not null references account(account_id) on delete cascade,
	mode           game_mode not null,
	games_won      bigint    not null default 0,
	games_lost     bigint    not null default 0,
	total_bid      bigint    not null default 0,
	bids_won       bigint    not null default 0,
	pts_won        bigint    not null default 0,
	pts_lost       bigint    not null default 0,
	game_pts_won   bigint    not null default 0,
	game_pts_lost  bigint    not null default 0,
	num_hearts     bigint    not null default 0,
	num_diamonds   bigint    not null default 0,
	num_clubs      bigint    not null default 0,
	num_spades     bigint    not null default 0,
	total_card_num bigint    not null default 0,

	primary key (account_id, mode)
);

create table refresh_token(
	token_hash   bytea       primary key check (octet_length(token_hash) = 32),
	profile_id   integer     not null,
	account_id   integer     not null,
	created_at   timestamptz not null default current_timestamp,
	last_used_at timestamptz not null default current_timestamp,
	expires_at   timestamptz not null,

	foreign key (profile_id, account_id) references account(profile_id, account_id) on delete cascade
);

create index refresh_token_id_index on refresh_token(profile_id, account_id);
create index refresh_token_exp_index on refresh_token(expires_at);

create table stage_account(
	username         varchar(15)  primary key,
	email_normalized varchar(255) not null unique,
	email            varchar(255) not null unique,
	password_hash    text         not null,
	tmp_token_hash   bytea        not null unique,
	expires_at       timestamptz  not null
);

create index stage_account_exp_index on stage_account(expires_at);

create table stage_email_change(
	profile_id           integer      primary key references profile(profile_id),
	new_email            varchar(255) not null unique,
	new_email_normalized varchar(255) not null unique,
	tmp_token_hash       bytea        not null unique,
	expires_at           timestamptz  not null
);

create index stage_email_change_index on stage_email_change(expires_at);

create table stage_password_change(
	account_id     integer     primary key references account(account_id) on delete cascade,
	tmp_token_hash bytea       not null unique,
	expires_at     timestamptz not null
);

create index stage_password_change_index on stage_password_change(expires_at);

create table feedback(
	feedback_id integer generated always as identity primary key,
	profile_id  integer     not null references profile(profile_id),
	created_at  timestamptz not null default current_timestamp,
	description text        not null
);

create table game(
	game_id    uuid primary key,
	game_code  varchar(7)  not null,
	mode       game_mode   not null,
	events     bytea       not null,
	started_at timestamptz not null default now(),
	ended_at   timestamptz
);

create unique index game_index on game(game_code) where ended_at is null;

create table game_player(
	game_id    uuid      not null references game(game_id) on delete cascade,
	account_id integer   not null references account(account_id),
	seat       smallint,
	won        bool,

	primary key (game_id, account_id)
);

create index game_player_account_index on game_player(account_id);

create table player_report(
	report_id     integer generated always as identity primary key,
	game_id       uuid        not null references game(game_id) on delete restrict,
	reporter_id   integer              references account(account_id) on delete restrict,
	target_id     integer     not null references account(account_id) on delete restrict,
	report_reason report_type not null,
	description   text        not null,
	created_at    timestamptz not null default current_timestamp,
	reviewed_by   integer              references account(account_id) on delete set null,
	resolved_at   timestamptz,

	check (reporter_id <> target_id),
	unique (game_id, reporter_id, target_id, report_reason)
);

create index player_report_index on player_report(target_id);