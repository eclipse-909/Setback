set sql_mode = 'TRADITIONAL';

create table if not exists staging_account(
    staging_account_id int unsigned auto_increment primary key,
    email              varchar(255) not null unique,
    username           varchar(15)  not null unique,
    password_hash      varchar(255) not null,
    tmp_token_hash     binary(32)   not null,
    expires_at         datetime     not null,
    
    index (expires_at)
);

create table if not exists account_owner(
    owner_id        int unsigned auto_increment primary key,
    email           varchar(255) not null unique,
    created_at      datetime     not null default current_timestamp,
    suspended_at    datetime              default null,
    suspended_until datetime              default null,
    suspend_reason  text                  default null
);

create table if not exists account(
    account_id        int unsigned auto_increment primary key,
    owner_id          int unsigned not null,
    username          varchar(15)  unique,
    original_username varchar(15)  null,
    password_hash     varchar(255) not null,
    created_at        datetime     not null default current_timestamp,
    last_active       datetime     not null default current_timestamp,
    is_admin          bool         not null default false,
    deleted_at        datetime     null,

    unique (owner_id, account_id),
    foreign key (owner_id) references account_owner(owner_id) on delete restrict
);

create table if not exists account_stats(
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

create table if not exists refresh_token(
    token_hash  binary(32)   not null primary key,
    owner_id    int unsigned not null,
    account_id  int unsigned not null,
    expires_at  datetime     not null,

    index (expires_at),
    foreign key (owner_id, account_id) references account(owner_id, account_id) on delete cascade
);

create table if not exists staging_password_change(
    account_id     int unsigned primary key,
    tmp_token_hash binary(32)   not null,
    expires_at     datetime     not null,

    index (expires_at),
    foreign key (account_id) references account(account_id) on delete cascade
);