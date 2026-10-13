set sql_mode = 'TRADITIONAL';
set time_zone = '+00:00';

create table staging_account(
    staging_account_id int unsigned auto_increment primary key,
    email              varchar(255)       not null unique,
    email_normalized   varchar(255)       not null unique,
    username           varchar(15) binary not null unique,
    password_hash      varchar(255)       not null,
    tmp_token_hash     binary(32)         not null unique,
    expires_at         datetime           not null,
    
    index (expires_at),
    index (tmp_token_hash)
);

create table account_owner(
    owner_id         int unsigned auto_increment primary key,
    email            varchar(255) not null unique,
    email_normalized varchar(255) not null unique,
    created_at       datetime     not null default current_timestamp,
    suspended_at     datetime         null,
    suspended_until  datetime         null,
    suspend_reason   text             null,

    check (suspended_until is null or suspended_until >= suspended_at)
);

create table account(
    account_id        int unsigned auto_increment primary key,
    owner_id          int unsigned       not null,
    username          varchar(15) binary          unique,
    original_username varchar(15)            null,
    password_hash     varchar(255)       not null,
    created_at        datetime           not null default current_timestamp,
    last_active       datetime           not null default current_timestamp,
    is_admin          bool               not null default false,
    deleted_at        datetime               null,

    unique (owner_id, account_id),
    foreign key (owner_id) references account_owner(owner_id) on delete restrict
);

create table owner_moderation_log(
    log_id     int unsigned auto_increment primary key,
    owner_id   int unsigned                       not null,
    action     enum('suspend','unsuspend','warn') not null,
    reason     text,
    acted_by   int unsigned                           null,
    created_at datetime                           not null default current_timestamp,
    expires_at datetime                               null,

    index (owner_id, created_at),
    foreign key (owner_id) references account_owner(owner_id) on delete cascade
);

create table owner_ip(
    owner_id   int unsigned  not null,
    ip         varbinary(16) not null,
    first_seen datetime      not null default current_timestamp,
    last_seen  datetime      not null default current_timestamp,

    primary key (owner_id, ip),
    index (ip)
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
    owner_id     int unsigned not null,
    account_id   int unsigned not null,
    created_at   datetime     not null default current_timestamp,
    last_used_at datetime     not null default current_timestamp,
    expires_at   datetime     not null,

    index (expires_at),
    foreign key (owner_id, account_id) references account(owner_id, account_id) on delete cascade
);

create table staging_email_change(
    account_id     int unsigned primary key,
    tmp_token_hash binary(32)   not null unique,
    expires_at     datetime     not null,

    index (expires_at),
    foreign key (account_id) references account(account_id) on delete cascade
);

create table staging_password_change(
    account_id       int unsigned primary key,
    email            varchar(255) not null unique,
    email_normalized varchar(255) not null unique,
    expires_at       datetime     not null,

    index (expires_at),
    foreign key (account_id) references account(account_id) on delete cascade
);

create event rm_expired
    on schedule every 1 hour
    do
    begin
        delete from staging_account         where expires_at < current_timestamp;
        delete from refresh_token           where expires_at < current_timestamp;
        delete from staging_email_change    where expires_at < current_timestamp;
        delete from staging_password_change where expires_at < current_timestamp;
    end;