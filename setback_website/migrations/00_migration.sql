SET sql_mode = 'TRADITIONAL';

create table if not exists Accounts (
    account_id int unsigned auto_increment,
    username varchar(255) not null unique,
    email varchar(255) not null unique,
    password varchar(4096) not null,
    last_active timestamp not null default now(),
    is_admin bool not null default false,
    check (account_id > 0),
    primary key (account_id)
);

create table if not exists KnownDevices (
    account_id int unsigned,
    foreign key (account_id) references Accounts(account_id) on delete cascade
);

create table if not exists KnownIPs (
    account_id int unsigned,
    ip varchar(39) not null,
    ip_version enum ('v4', 'v6') not null,
    primary key (account_id, ip),
    foreign key (account_id) references Accounts(account_id) on delete cascade
);