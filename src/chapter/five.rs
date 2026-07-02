//映射的边界：
// SQLite 端：存储类型只有 NULL, INTEGER, REAL, TEXT, BLOB。
// Rust 端：我们想要存储枚举（如 Status::Active）、自定义结构体、或者第三方类型（如 chrono::DateTime）。
// rusqlite 的做法是：你必须定义“怎么把 Rust 类型变成 SQLite 类型（ToSql）”，以及“怎么把 SQLite 类型变回 Rust 类型（FromSql）”。
// rusqlite 已经为基本类型（i32, String, Vec<u8> 等）实现了这些 trait，我们只需要为自己的类型实现即可。

//Rust代码示例：
//枚举映射为整数
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

//复杂结构体映射为JSON
#[derive(Debug, Serialize, Deserialize)]
struct UserSettings {
    theme: String,
    notifications: bool,
}

// 实现 ToSql（转为 TEXT JSON）
impl ToSql for UserSettings {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let json = serde_json::to_string(self).
            map_err(|_| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "JSON serialize error"))))?;
        Ok(ToSqlOutput::from(json))
    }
}

// 实现 FromSql（从 TEXT 解析）
impl FromSql for UserSettings {
    fn column_result(value: ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let json = value.as_str()?;
        serde_json::from_str(json).map_err(|_| rusqlite::types::FromSqlError::Other(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "JSON deserialize error"))))
    }
}


//1. EXPLAIN QUERY PLAN 的作用
// 不执行 SQL，只展示优化器打算如何执行。
// 输出结构通常为三列：id、parent、detail，以树形结构表示执行步骤。
//SCAN TABLE - 全表扫描，性能差     SEARCH TABLE USING INDEX - 使用索引快速定位，性能好
//USE TEMP B-TREE - 需要临时表排序/分组,性能一般      CO-ROUTINE - 子查询协程执行，通常可接受       
//2. 多表连接的计划
// 优化器会显示表的连接顺序（对 INNER JOIN 会重排）。
// 若某表出现 SCAN，说明该表没有可用索引，是瓶颈所在。
//Rust使用示例：
use rusqlite::{Connection, params};

fn explain_query(conn: &Connection, sql: &str) -> Result<(), rusqlite::Error> {
    let explain_sql = format!("EXPLAIN QUERY PLAN {}", sql);
    let mut stmt = conn.prepare(&explain_sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?, row.get::<_, String>(2)?))
    })?;
    for row in rows {
        let (id, parent, detail) = row?;
        let indent = if parent == 0 { "" } else { "  " };
        println!("{}{}: {}", indent, id, detail);
    }
    Ok(())
}

//调用
// let conn = Connection::open_in_memory()?;
// // 建表、插数据...
// explain_query(&conn, "SELECT * FROM orders WHERE user_id = 123")?;