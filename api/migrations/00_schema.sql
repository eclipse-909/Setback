set sql_mode = 'TRADITIONAL';

create table if not exists accounts (
    account_id  int unsigned auto_increment primary key,
    username    varchar(255)  not null,
    email       varchar(255)  not null unique,
    password    varchar(4096) not null,
    is_active   bool          not null default true,
    last_active timestamp     not null default now(),
    is_admin    bool          not null default false,
    check (account_id > 0)
);

create table if not exists account_ips (
    account_id int unsigned,
    ip varchar(39) not null,
    primary key (account_id, ip),
    foreign key (account_id) references Accounts(account_id) on delete cascade
);