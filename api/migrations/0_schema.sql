create table profile(
    profile_id       int unsigned auto_increment primary key,
    created_at       datetime     not null default current_timestamp
);

create table profile_email(
    profile_id       int unsigned,
    email            varchar(255) not null unique,
    email_normalized varchar(255) not null unique,
    active           bool         not null,

    primary key (profile_id, email_normalized),
    index (active),
    unique (profile_id, active),
    foreign key (profile_id) references profile(profile_id)
);

create table profile_ip(
    profile_id int unsigned  not null,
    ip         varbinary(16) not null,
    first_seen datetime      not null default current_timestamp,
    last_seen  datetime      not null default current_timestamp,

    primary key (profile_id, ip),
    index (ip),
    foreign key (profile_id) references profile(profile_id)
);

create table profile_moderation_log(
    log_id     int unsigned auto_increment primary key,
    profile_id int unsigned                       not null,
    action     enum('suspend','unsuspend','warn') not null,
    reason     text,
    acted_by   int unsigned                           null,
    created_at datetime                           not null default current_timestamp,
    expires_at datetime                               null,

    index (profile_id, created_at),
    foreign key (profile_id) references profile(profile_id)
);

create table account(
    account_id        int unsigned auto_increment primary key,
    profile_id        int unsigned       not null,
    username          varchar(15) binary          unique,
    username_archive  varchar(15)            null,
    password_hash     varchar(255)       not null,
    created_at        datetime           not null default current_timestamp,
    last_active       datetime           not null default current_timestamp,
    is_admin          bool               not null default false,
    deleted_at        datetime               null,

    unique (profile_id, account_id),
    foreign key (profile_id) references profile(profile_id)
);

create table account_stats(
    account_id     int unsigned    not null,
    mode           enum('sp','mp') not null,
    games_won      int unsigned    not null default 0,
    games_lost     int unsigned    not null default 0,
    total_bid      int unsigned    not null default 0,
    bids_won       int unsigned    not null default 0,
    pts_won        int unsigned    not null default 0,
    pts_lost       int unsigned    not null default 0,
    game_pts_won   int unsigned    not null default 0,
    game_pts_lost  int unsigned    not null default 0,
    num_hearts     int unsigned    not null default 0,
    num_diamonds   int unsigned    not null default 0,
    num_clubs      int unsigned    not null default 0,
    num_spades     int unsigned    not null default 0,
    total_card_num int unsigned    not null default 0,

    primary key (account_id, mode),
    foreign key (account_id) references account(account_id) on delete cascade
);

create table refresh_token(
    token_hash   binary(32)   not null primary key,
    profile_id   int unsigned not null,
    account_id   int unsigned not null,
    created_at   datetime     not null default current_timestamp,
    last_used_at datetime     not null default current_timestamp,
    expires_at   datetime     not null,

    index (expires_at),
    foreign key (profile_id, account_id) references account(profile_id, account_id) on delete cascade
);

create table stage_account(
    username           varchar(15) binary primary key,
    email_normalized   varchar(255)       not null unique,
    email              varchar(255)       not null unique,
    password_hash      varchar(255)       not null,
    tmp_token_hash     binary(32)         not null unique,
    expires_at         datetime           not null,

    index (tmp_token_hash),
    index (expires_at)
);

create table stage_email_change(
    profile_id           int unsigned primary key,
    new_email            varchar(255) not null unique,
    new_email_normalized varchar(255) not null unique,
    tmp_token_hash       binary(32)   not null unique,
    expires_at           datetime     not null,

    index (tmp_token_hash),
    index (expires_at),
    foreign key (profile_id) references profile(profile_id)
);

create table stage_password_change(
    account_id       int unsigned primary key,
    tmp_token_hash   binary(32)   not null unique,
    expires_at       datetime     not null,

    index (expires_at),
    foreign key (account_id) references account(account_id) on delete cascade
);

create event rm_expired
    on schedule every 1 hour
    do
    begin
        delete from refresh_token         where expires_at < current_timestamp;
        delete from stage_account         where expires_at < current_timestamp;
        delete from stage_email_change    where expires_at < current_timestamp;
        delete from stage_password_change where expires_at < current_timestamp;
    end;