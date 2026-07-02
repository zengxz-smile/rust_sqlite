use criterion::Criterion;
use rusqlite::{Connection, params, types::{ToSql, FromSql, ToSqlOutput, ValueRef, FromSqlError, FromSqlResult}};
use serde::{Deserialize, Serialize};
use serde_json;
use std::io::{Error,ErrorKind};

#[allow(unused)]
#[derive(Debug, PartialEq, Clone, Copy)]
enum UserStatus {
    Inactive = 0,
    Active = 1,
    Banned = 2,
}
// 1. 实现 ToSql（Rust -> SQLite）
impl ToSql for UserStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as i32))
    }
}
// 2. 实现 FromSql（SQLite -> Rust）
impl FromSql for UserStatus {
    fn column_result(value: ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_i64()? {
            0 => Ok(UserStatus::Inactive),
            1 => Ok(UserStatus::Active),
            2 => Ok(UserStatus::Banned),
            _ => Err(rusqlite::types::FromSqlError::OutOfRange(3)),
        }
    }
}

#[allow(unused)]
//复杂结构体映射为JSON
#[derive(Debug, Serialize, Deserialize)]
struct UserSettings {
    theme: String,
    notifications: bool,
}

// 实现 ToSql（转为 TEXT JSON）
impl ToSql for UserSettings {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let json = serde_json::to_string(self)
            .map_err(|_| rusqlite::Error::ToSqlConversionFailure(Box::new(Error::new(ErrorKind::Other, "JSON serialize error"))))?;
        Ok(ToSqlOutput::from(json))
    }
}

// 实现 FromSql（从 TEXT 解析）
impl FromSql for UserSettings {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let json = value.as_str()?;
        serde_json::from_str(json)
            .map_err(|_| FromSqlError::Other(Box::new(Error::new(ErrorKind::Other, "JSON deserialize error"))))
    }
}

#[allow(unused)]
pub fn bench_raw_i32(c: &mut Criterion) {
    c.bench_function("raw_i32_status", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute("CREATE TABLE test (status INTEGER)", []).unwrap();
            for i in 0..1000 {
                conn.execute("INSERT INTO test VALUES (?1)", params![i % 3]).unwrap();
            }
            let _: Vec<i32> = conn.prepare("SELECT status FROM test").unwrap()
                .query_map([], |row| row.get(0)).unwrap()
                .collect::<Result<Vec<_>, _>>().unwrap();
        })
    });
}

#[allow(unused)]
pub fn bench_enum_mapping(c: &mut Criterion) {
    c.bench_function("enum_mapping_status", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute("CREATE TABLE test (status INTEGER)", []).unwrap();
            let statuses = [UserStatus::Inactive, UserStatus::Active, UserStatus::Banned];
            for i in 0..1000 {
                conn.execute("INSERT INTO test VALUES (?1)", params![statuses[i % 3]]).unwrap();
            }
            let _: Vec<UserStatus> = conn.prepare("SELECT status FROM test").unwrap()
                .query_map([], |row| row.get(0)).unwrap()
                .collect::<Result<Vec<_>, _>>().unwrap();
        })
    });
}

#[allow(unused)]
pub fn bench_json_mapping(c: &mut Criterion) {
    c.bench_function("json_mapping", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute("CREATE TABLE test (settings TEXT)", []).unwrap();
            let settings = UserSettings { theme: "dark".into(), notifications: true };
            for _ in 0..1000 {
                conn.execute("INSERT INTO test VALUES (?1)", params![&settings]).unwrap();
            }
            let _: Vec<UserSettings> = conn.prepare("SELECT settings FROM test")
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        })
    });
}