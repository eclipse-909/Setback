set sql_mode = 'TRADITIONAL';

create table if not exists account_owner(
    owner_id        int unsigned auto_increment primary key,
    email           varchar(255) not null,
    suspended_at    timestamp default null,
    suspended_until timestamp default null,
    suspend_reason  text      default null
);

create table if not exists account(
    -- account info
    account_id        int unsigned auto_increment primary key,
    owner_id          int unsigned  not null,
    username          varchar(255)  not null,
    password          varchar(4096) not null,
    last_active       timestamp     not null default now(),
    is_admin          bool          not null default false,
    is_deleted        bool          not null default false,
    -- game statistics for nerds
    sp_games_won      int unsigned  not null default 0,
    sp_games_lost     int unsigned  not null default 0,
    sp_total_bid      int unsigned  not null default 0,
    sp_bids_won       int unsigned  not null default 0,
    sp_pts_won        int unsigned  not null default 0,
    sp_pts_lost       int unsigned  not null default 0,
    sp_game_pts_won   int unsigned  not null default 0,
    sp_game_pts_lost  int unsigned  not null default 0,
    sp_num_hearts     int unsigned  not null default 0,
    sp_num_diamonds   int unsigned  not null default 0,
    sp_num_clubs      int unsigned  not null default 0,
    sp_num_spades     int unsigned  not null default 0,
    sp_total_card_num int unsigned  not null default 0,

    mp_games_won      int unsigned  not null default 0,
    mp_games_lost     int unsigned  not null default 0,
    mp_total_bid      int unsigned  not null default 0,
    mp_bids_won       int unsigned  not null default 0,
    mp_pts_won        int unsigned  not null default 0,
    mp_pts_lost       int unsigned  not null default 0,
    mp_game_pts_won   int unsigned  not null default 0,
    mp_game_pts_lost  int unsigned  not null default 0,
    mp_num_hearts     int unsigned  not null default 0,
    mp_num_diamonds   int unsigned  not null default 0,
    mp_num_clubs      int unsigned  not null default 0,
    mp_num_spades     int unsigned  not null default 0,
    mp_total_card_num int unsigned  not null default 0,

    check (account_id > 0),
    foreign key (owner_id) references account_owner(owner_id) on delete restrict
);