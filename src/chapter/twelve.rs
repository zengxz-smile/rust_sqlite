//Schema 迁移管理
//SQLite DDL 特性与限制
//1.SQLite的Schema存储：sqlite_master表
//每个SQLite数据库都有一个名为 sqlite_master 的系统表，它存储了所有表、索引、触发器、视图的创建语句。
// -- 查看所有表
// SELECT name, sql FROM sqlite_master WHERE type='table' ORDER BY name;
// -- 查看特定表的 Schema
// SELECT sql FROM sqlite_master WHERE type='table' AND name='users';
//字段说明：
// type: 对象类型（table、index、trigger、view）
// name: 对象名称
// tbl_name: 关联的表名（对于索引/触发器）
// rootpage: B-tree 根页（内部使用）
// sql: 完整的 CREATE 语句文本
//2.ALTER TABLE 的有限支持（关键限制）
//与 PostgreSQL 或 MySQL 不同，SQLite 的 ALTER TABLE 功能非常有限：
//ALTER TABLE old RENAME TO new; - 重命名表
//ALTER TABLE table RENAME COLUMN old TO new; - 重命名列
//ALTER TABLE table ADD COLUMN column type; - 添加列
//ADD COLUMN 的列不能有 NOT NULL 约束（除非指定了 DEFAULT 值）
//ADD COLUMN 的列如果有 UNIQUE 或 PRIMARY KEY 约束，SQLite 会拒绝
//3. 复杂变更的解决方案：重建表
// 对于不支持的操作（如删除列、修改列类型），标准做法是：
// 创建新表（新 Schema）
// 将旧表数据复制到新表（转换格式）
// 删除旧表
// 将新表重命名为旧表名
// 重建索引和触发器
//-- 示例：删除 users 表的 age 列
// CREATE TABLE users_new (
//     id INTEGER PRIMARY KEY,
//     name TEXT NOT NULL,
//     email TEXT UNIQUE
// );
// INSERT INTO users_new (id, name, email)
// SELECT id, name, email FROM users;
// DROP TABLE users;
// ALTER TABLE users_new RENAME TO users;
// -- 重建索引（如果有）
// CREATE INDEX idx_users_email ON users(email);

//Rust代码部分：
use rusqlite::{Connection, Result};

//查询 sqlite_master 表
#[allow(unused)]
fn inspect_schema(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT name, sql FROM sqlite_master WHERE type='table' ORDER BY name"
    )?;
    
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    
    println!("=== 数据库 Schema ===\n");
    for row in rows {
        let (name, sql) = row?;
        println!("表名: {}", name);
        if let Some(sql) = sql {
            println!("创建语句: {}", sql);
        } else {
            println!("(系统表或视图)");
        }
        println!();
    }
    Ok(())
}

// 检查表是否存在
#[allow(unused)]
fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
        [table_name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

#[allow(unused)]
#[derive(Debug)]
struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

//获取表的列信息（使用 PRAGMA table_info）
#[allow(unused)]
fn get_table_columns(conn: &Connection, table_name: &str) -> Result<Vec<ColumnInfo>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name))?;
    let rows = stmt.query_map([], |row| {
        Ok(ColumnInfo {
            name: row.get(1)?,
            type_name: row.get(2)?,
            not_null: row.get(3)?,
            default_value: row.get(4)?,
            is_primary_key: row.get(5)?,
        })
    })?;
    rows.collect()
}

//添加列（带默认值）
#[allow(unused)]
fn add_column_age(conn: &Connection) -> Result<()> {
    // 先检查列是否存在
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = 'age'",
        [],
        |row| row.get(0),
    )?;
    
    if !exists {
        conn.execute(
            "ALTER TABLE users ADD COLUMN age INTEGER DEFAULT 0",
            [],
        )?;
        println!("✅ 已添加 age 列");
    } else {
        println!("ℹ️ age 列已存在，跳过");
    }
    Ok(())
}

//使用 PRAGMA foreign_keys 管理外键约束
#[allow(unused)]
fn with_foreign_keys_disabled<F, T>(conn: &Connection, f: F) -> Result<T>
where
    F: FnOnce(&Connection) -> Result<T>,
{
    conn.pragma_update(None, "foreign_keys", false)?;
    let result = f(conn)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(result)
}

#[allow(unused)]
pub fn show(){
    let conn = Connection::open_in_memory().unwrap();
    inspect_schema(&conn);
    table_exists(&conn,"users");
    get_table_columns(&conn,"users");
}