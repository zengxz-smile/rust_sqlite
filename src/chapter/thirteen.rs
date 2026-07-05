//Schema 迁移管理
//迁移策略与版本管理

//为什么需要迁移策略？ 
//随着项目迭代，数据库 Schema 的变更是不可避免的。手动管理会带来一系列问题：
// 不一致性：开发、测试、生产环境 Schema 不同步
// 不可重复性：无法在新环境中一键初始化到最新 Schema
// 回滚困难：出问题后不知道如何安全回退
// 协作冲突：多人开发时，Schema 变更难以协调
//目标：建立一套系统化的流程，使 Schema 变更像代码变更一样可控、可追溯、可自动化执行。

//1. 迁移（Migration）的定义
// 一个迁移是一个原子操作单元，它定义了如何将数据库从一个版本变更为下一个版本。通常包含：
// Up（升级）：从旧 Schema 变到新 Schema 的逻辑
// Down（降级）：从新 Schema 回退到旧 Schema 的逻辑（可选，但强烈建议）
//2. 迁移的版本管理
//有多种方式追踪当前数据库处于哪个版本，常见的有：版本号表、时间戳命名（如 20260101120000_add_age.sql）、Hash 值
//最佳实践：版本号表 + 有序迁移列表。
//通常创建一个 schema_migrations 表，用于记录已执行的迁移：
// CREATE TABLE IF NOT EXISTS schema_migrations (
//     version INTEGER PRIMARY KEY,     -- 版本号
//     applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- 执行时间
//     name TEXT                         -- 迁移名称
// );
//3. 迁移的原子性 - 每个迁移必须在一个事务内执行，以保证如果迁移失败，数据库状态不变（ACID 的原子性）。

//迁移策略：
//策略一：纯 SQL 脚本（基础版） - 最简单的方案：按顺序执行一组 SQL 文件。
// migrations/
// ├── 001_create_users_table.sql
// ├── 002_add_age_column.sql
// └── 003_add_email_unique.sql
//伪代码
// fn run_migrations(conn: &Connection) -> Result<()> {
//     let current = get_current_version(conn)?;
//     for migration in get_migrations_sorted()? {
//         if migration.version > current {
//             conn.transaction(|tx| {
//                 let sql = std::fs::read_to_string(migration.path)?;
//                 tx.execute_batch(&sql)?;
//                 set_version(tx, migration.version)?;
//                 Ok(())
//             })?;
//         }
//     }
//     Ok(())
// }
// 优点：简单、独立于语言、易于理解。
// 缺点：无法处理复杂的条件逻辑（如仅在表不存在时创建）、难以应对数据库特定的变通方案（如重建表）。

//策略二：Rust 定义的迁移（推荐）
//将每个迁移定义为 Rust 中的一段代码，而非纯 SQL 文件。这可以让你充分利用 Rust 的类型系统和逻辑控制。
// 迁移列表 - 伪代码
// struct Migration {
//     version: u32,
//     name: &'static str,
//     up: Box<dyn Fn(&Connection) -> Result<()>>,
//     down: Option<Box<dyn Fn(&Connection) -> Result<()>>>,
// }

// let  migrations = vec![
//     Migration {
//         version: 1,
//         name: "create_users_table",
//         up: Box::new(|conn| conn.execute_batch("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);")?),
//         down: None,
//     },
//     Migration {
//         version: 2,
//         name: "add_age_column",
//         up: Box::new(|conn| conn.execute_batch("ALTER TABLE users ADD COLUMN age INTEGER DEFAULT 0;")?),
//         down: None,
//     },
// ];
// 优点：
//  可以包含复杂的条件逻辑（如检查列是否存在）
//  可以利用 Rust 类型（如枚举、结构体）定义 Schema
//  易于与项目代码集成
// 缺点：
//  无法跨语言复用
//  需要重新编译项目

//策略三：混合方案
// 结合两者的优点：大部分迁移使用纯 SQL，对于复杂变更（如删除列），使用 Rust 代码实现重建表逻辑。

//实际应用中的考量
//1. 预检查与幂等性 - 迁移应该是幂等的（可以安全地重复执行）。每次执行前检查 Schema 是否已经在该状态。
//2. 避免多个实例的并发执行 - 通过数据库锁确保只有一个实例执行迁移。
// rusqlite_migration 中采用的方式：创建一个 schema_migrations 表，并通过 BEGIN EXCLUSIVE TRANSACTION 防止并发。
//3. 数据迁移 - Schema 变更通常伴随着数据迁移

use rusqlite::{Connection, Result};
use rusqlite_migration::{Migrations, M};

#[allow(unused)]
pub fn show() -> Result<()>{
    let mut conn = Connection::open("my_app.db")?;
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    // 定义迁移列表（按版本顺序）
    let migrations = Migrations::new(vec![
        // v1: 创建 users 表
        M::up(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );"
        ).down(
            "DROP TABLE users;"
        ),

        // v2: 添加 email 列（唯一约束）
        M::up(
            "CREATE TABLE users_new (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT UNIQUE
            );
            INSERT INTO users_new (id, name) SELECT id, name FROM users;
            DROP TABLE users;
            ALTER TABLE users_new RENAME TO users;"
        ).down(
            "CREATE TABLE users_old (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );
            INSERT INTO users_old (id, name) SELECT id, name FROM users;
            DROP TABLE users;
            ALTER TABLE users_old RENAME TO users;"
        ),

        // v3: 添加 age 列并填充数据
        M::up(
            "ALTER TABLE users ADD COLUMN age INTEGER;
            UPDATE users SET age = ABS(RANDOM()) % 63 + 18;
            CREATE TABLE users_new (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT UNIQUE,
                age INTEGER NOT NULL
            );
            INSERT INTO users_new (id, name, email, age)
            SELECT id, name, email, age FROM users;
            DROP TABLE users;
            ALTER TABLE users_new RENAME TO users;"
        ).down(
            "CREATE TABLE users_old (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT UNIQUE
            );
            INSERT INTO users_old (id, name, email)
            SELECT id, name, email FROM users;
            DROP TABLE users;
            ALTER TABLE users_old RENAME TO users;"
        ),
    ]);

    // 执行所有未应用的迁移
    migrations.to_latest(&mut conn).unwrap();

    // 验证数据
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    println!("✅ 迁移完成，users 表共 {} 行", count);

    // 验证 age 是否有值
    let null_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE age IS NULL",
        [],
        |row| row.get(0),
    )?;
    println!("✅ age 列非空行数: {}", count - null_count);

    Ok(())
}