create type ModAction  as enum('suspend','unsuspend','warn');
create type GameMode   as enum('sp','mp');
create type ReportType as enum('Stalling','Hacking','Cheating');
create type GameEvent  as enum(
	'Init',
	'PlayerBid',
	'PlayerPlayedCard',
	'PromptBid',
	'PromptCard',
	'DealCards',
	'RoundScores',
	'GameOver'
);

create table profile(
	profile_id serial    primary key,
	created_at timestamp not null default current_timestamp
);

create table profile_email(
	profile_id       int          references profile(profile_id),
	email            varchar(255) not null unique,
	email_normalized varchar(255) not null unique,
	active           bool         not null,

	primary key (profile_id, email_normalized),
	unique (profile_id, active)
);

create index profile_email_index on profile_email(active);

create table profile_ip(
	profile_id int         not null references profile(profile_id),
	ip         varchar(39) not null,
	first_seen timestamp   not null default current_timestamp,
	last_seen  timestamp   not null default current_timestamp,

	primary key (profile_id, ip)
);

create index profile_ip_index on profile_ip(ip);

create table profile_moderation_log(
	log_id     serial    primary key,
	profile_id int       not null references profile(profile_id),
	action     ModAction not null,
	reason     text,
	acted_by   int       null,
	created_at timestamp not null default current_timestamp,
	expires_at timestamp null
);

create index profile_moderation_log_index on profile_moderation_log(profile_id, created_at);

create table account(
	account_id       serial       primary key,
	profile_id       int          not null references profile(profile_id),
	username         varchar(15)           unique,
	username_archive varchar(15)      null,
	password_hash    varchar(255) not null,
	created_at       timestamp    not null default current_timestamp,
	last_active      timestamp    not null default current_timestamp,
	is_admin         bool         not null default false,
	deleted_at       timestamp        null,

	unique (profile_id, account_id)
);

create table account_stats(
	account_id     int      not null references account(account_id) on delete cascade,
	mode           GameMode not null,
	games_won      bigint   not null default 0,
	games_lost     bigint   not null default 0,
	total_bid      bigint   not null default 0,
	bids_won       bigint   not null default 0,
	pts_won        bigint   not null default 0,
	pts_lost       bigint   not null default 0,
	game_pts_won   bigint   not null default 0,
	game_pts_lost  bigint   not null default 0,
	num_hearts     bigint   not null default 0,
	num_diamonds   bigint   not null default 0,
	num_clubs      bigint   not null default 0,
	num_spades     bigint   not null default 0,
	total_card_num bigint   not null default 0,

	primary key (account_id, mode)
);

create table refresh_token(
	token_hash   varchar(32) primary key,
	profile_id   int         not null,
	account_id   int         not null,
	created_at   timestamp   not null default current_timestamp,
	last_used_at timestamp   not null default current_timestamp,
	expires_at   timestamp   not null,

	foreign key (profile_id, account_id) references account(profile_id, account_id) on delete cascade
);

create index refresh_token_index on refresh_token(expires_at);

create table stage_account(
	username         varchar(15)  primary key,
	email_normalized varchar(255) not null unique,
	email            varchar(255) not null unique,
	password_hash    varchar(255) not null,
	tmp_token_hash   varchar(32)  not null unique,
	expires_at       timestamp    not null
);

create index stage_account_token_index on stage_account(tmp_token_hash);
create index stage_account_exp_index   on stage_account(expires_at);

create table stage_email_change(
	profile_id           int          primary key references profile(profile_id),
	new_email            varchar(255) not null unique,
	new_email_normalized varchar(255) not null unique,
	tmp_token_hash       varchar(32)  not null unique,
	expires_at           timestamp    not null
);

create index stage_email_change_token_index on stage_email_change(tmp_token_hash);
create index stage_email_change_exp_index   on stage_email_change(expires_at);

create table stage_password_change(
	account_id     int         primary key references account(account_id) on delete cascade,
	tmp_token_hash varchar(32) not null unique,
	expires_at     timestamp   not null
);

create index stage_password_change_index on stage_password_change(expires_at);

create table feedback(
	feedback_id serial    primary key,
	profile_id  int       not null,
	created_at  timestamp not null default current_timestamp,
	description text      not null
);

create table player_report(
	report_id     serial     primary key,
	reporter_id   int            null,
	target_id     int        not null,
	report_reason ReportType not null,
	created_at    timestamp  not null default current_timestamp
);

create table game_log(
	log_id     bigserial primary key,
	game_code  char(7)   not null,
	event_type GameEvent not null,
	game_event bytea     not null,
	created_at timestamp not null default current_timestamp
);

-- create event rm_expired
--     on schedule every 1 hour
--     do
--     begin
--         delete from refresh_token         where expires_at < current_timestamp;
--         delete from stage_account         where expires_at < current_timestamp;
--         delete from stage_email_change    where expires_at < current_timestamp;
--         delete from stage_password_change where expires_at < current_timestamp;
--     end;