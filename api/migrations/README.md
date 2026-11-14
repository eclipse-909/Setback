All sql files should be PostgreSql compatible.

Only the API will be responsible for migrations, not the server.

From [sqlx](https://docs.rs/sqlx/latest/sqlx/migrate/trait.MigrationSource.html):
> All these scripts must be stored in files with names using the format \<VERSION\>_\<DESCRIPTION\>.sql, where <VERSION> is a string that can be parsed into i64 and its value is greater than zero, and <DESCRIPTION> is a string.
> 
> Files that don’t match this format are silently ignored.