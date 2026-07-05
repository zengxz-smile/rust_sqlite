//回顾Rollback Journal 的三大痛点：
//痛点 1：两次 fsync 导致写入性能差 - 每次 fsync 约耗时 0.5-2ms，这意味着每秒最多只能处理 500-2000 个事务。
//痛点 2：读写互斥（写阻塞读，读阻塞写）- 在高并发读写场景下，连接池中的线程会频繁阻塞等待
//痛点 3：频繁的文件创建/删除（在 DELETE 模式下） - 频繁的文件系统元数据操作会消耗 CPU，并导致文件系统缓存失效。
// Rollback Journal: 先备份旧数据，再写新数据
// 修改前 → 备份到 journal → 修改主数据库 → 提交时 fsync

//SQLite使用WAL模式避免了Rollback Journal的三大痛点。什么是WAL - Write-Ahead Logging（预写日志）。
// WAL: 直接追加到 WAL 文件，主数据库暂不更新
// 修改 → 追加到 WAL 文件 → 提交时 fsync(WAL) → 主数据库延迟更新
//WAL模式：仅WAL文件、读不阻塞写，写不阻塞读、WAL文件持久保留
//SQLite 官方文档明确指出："WAL mode is generally faster and more reliable than the traditional rollback journal."
//根据官方基准数据：
// 写入性能：WAL 模式比 DELETE 模式快 2-10 倍
// 读取并发：WAL 模式下，一个写事务和多个读事务可以同时进行（Rollback 模式下会被阻塞）
// IO 次数：WAL 模式减少约 50% 的磁盘写入次数

//WAL模式的代价
//WAL 文件膨胀 - .db-wal 文件会持续增长，直到 checkpoint
//Checkpoint 开销 - 执行 checkpoint 时可能产生 I/O 尖峰
//内存占用增加 - 需要额外内存缓存 WAL 页
//读取复杂性 - 	读操作需要合并主数据库和 WAL 文件
//备份变复杂 - 备份时需要特别注意 WAL 文件

//WAL 的文件结构
//当一个数据库启用 WAL 模式后，目录中会多出两个文件：
// 项目目录/
// ├── myapp.db          # 主数据库文件（存储已提交的完整数据） - 连接时创建，持久存在
// ├── myapp.db-wal      # WAL 文件（存储尚未合并到主数据库的修改） - 第一个写连接时创建，连接关闭时可能保留
// └── myapp.db-shm      # 共享内存文件（协调多连接的访问） - 与 WAL 文件同时创建/删除

//WAL写入流程（从Rust到磁盘）
// Rust->>SQLite: BEGIN / UPDATE / INSERT
// SQLite->>WAL: 1. 将修改追加到 WAL 文件
// SQLite->>SQLite: 2. 维护内存中的页面缓存
// Rust->>SQLite: COMMIT
// SQLite->>WAL: 3. 写入 COMMIT 记录
// SQLite->>WAL: 4. fsync(WAL) 强制刷盘
// SQLite-->>Rust: 提交成功
// Note over WAL,DB: 主数据库暂未更新！
// SQLite->>DB: 5. Checkpoint 时合并到主数据库
//步骤 1-4 是每个事务必须执行的（所以 WAL 也有一次 fsync）
//步骤 5 是延迟执行的（不阻塞事务提交）
//写入流程比 Rollback Journal 少一次 fsync（因为不需要刷主数据库）

//WAL读取流程（如何看到最新数据）
// A[读请求] --> B{数据在 WAL 中？}
// B -->|是| C[从 WAL 文件读取]
// B -->|否| D[从主数据库读取]
// C --> E[合并结果]
// D --> E
// E --> F[返回最新数据]
//读取时，SQLite 按以下优先级查找数据：
// 1.WAL 文件尾部（最新修改，尚未 checkpoint）
// 2.WAL 文件中部（较旧的修改）
// 3.主数据库文件（已 checkpoint 的数据）
//WAL 文件内部通过 帧索引（Frame Index） 快速定位特定页面的最新版本。这个索引存储在 .db-shm 文件中，由所有连接共享。

//Checkpoint 机制（WAL 的“合并”过程）
//Checkpoint 是 WAL 模式的“幕后管家”。当满足特定条件时，SQLite 会自动将 WAL 中的修改合并到主数据库文件。
//Checkpoint 触发条件
//  1.自动 Checkpoint：当 WAL 文件达到 wal_autocheckpoint 阈值时（默认 1000 页 ≈ 4MB）
//  2.手动 Checkpoint：应用主动调用 PRAGMA wal_checkpoint
//  3.连接关闭时：最后一个连接关闭时，SQLite 会尝试执行 checkpoint
//Checkpoint 的三种模式
//  PASSIVE - 尽可能执行，但不阻塞其他操作 - 最小，但可能无法完成
//  FULL - 完全执行，阻塞写入 - 中等，会短暂阻塞
//  RESTART - 完全执行，并清空 WAL 文件 - 较大，写操作会被阻塞
//Checkpoint 过程:
// 时间线：
// ┌──────────────┬──────────────┬──────────────┬──────────────┐
// │   事务1      │   事务2      │   Checkpoint │   事务3      │
// │   写入 WAL   │   写入 WAL   │   合并到 DB  │   写入 WAL   │
// └──────────────┴──────────────┴──────────────┴──────────────┘
//      ↑               ↑              ↑              ↑
//    WAL 增长       WAL 增长       WAL 清空       WAL 重新增长

//Rust代码部分
use rusqlite::{Connection, Result};
use std::path::Path;

#[allow(unused)] //最简单的方式
fn enable_wal(conn: &Connection) -> Result<()> {
    // 切换日志模式为 WAL
    conn.pragma_update(None, "journal_mode", "WAL")?;
    Ok(())
}

#[allow(unused)]   //验证是否生效
fn verify_wal(conn: &Connection) -> Result<bool> {
    let mode: String = conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    Ok(mode == "wal")
}

#[allow(unused)] //使用示例
fn test_case() -> anyhow::Result<()> { 
    let conn = Connection::open("app.db")?;
    enable_wal(&conn)?;
    assert!(verify_wal(&conn)?);
    println!("✅ WAL 模式已成功启用");
    Ok(())
}
#[allow(unused)]
fn configure_wal_checkpoint(conn: &Connection, pages: u32) -> Result<()> {
    //wal_autocheckpoint - 自动 Checkpoint 阈值，WAL 文件达到多少页时触发自动 checkpoint
    //pages - 0（禁用自动 checkpoint）到任意正整数，推荐：生产环境 1000-5000；开发环境 100-500
    conn.pragma_update(None, "wal_autocheckpoint", pages)?;
    Ok(())
}

#[allow(unused)]
fn configure_wal_size_limit(conn: &Connection, limit_bytes: u32) -> Result<()> {
    //journal_size_limit - WAL 文件最大大小（字节），默认值：-1（无限制）
    //limit_bytes - 0（删除 WAL 文件）到任意正整数，推荐：32MB - 256MB
    //SQLite 会在 WAL 达到limit_bytes大小时自动执行 checkpoint
    conn.pragma_update(None, "journal_size_limit", limit_bytes)?;
    Ok(())
}

#[allow(unused)]
fn configure_sync(conn: &Connection) -> Result<()> {
    //synchronous - 数据写入磁盘的同步程度，可选值：OFF、NORMAL、FULL。默认值：FULL
    // WAL 模式下推荐 NORMAL（安全性与性能的平衡）
    //FULL - 安全最高，性能最慢
    //NORMAL - 安全高，性能较快
    //OFF - 安全低，性能最快，仅用于只读或临时数据库
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(())
}

#[allow(unused)]
fn switch_journal_mode(conn: &Connection, mode: &str) -> Result<String> {
    conn.pragma_update(None, "journal_mode", mode)?;
    let mode: String = conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    Ok(mode)
}

#[allow(unused)] // 使用示例
fn demo_switch_modes(conn: &Connection) -> Result<()> {
    // 切换到 DELETE 模式
    let mode = switch_journal_mode(conn, "DELETE")?;
    println!("切换到: {}", mode);
    
    // 切换回 WAL
    let mode = switch_journal_mode(conn, "WAL")?;
    println!("切换回: {}", mode);
    
    Ok(())
}

#[allow(unused)]
struct WalConfig {
    autocheckpoint_pages: u32,
    journal_size_limit_mb: u32,
    synchronous: String,
    cache_size_pages: u32,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            autocheckpoint_pages: 1000,
            journal_size_limit_mb: 64,
            synchronous: "NORMAL".to_string(),
            cache_size_pages: 2000,
        }
    }
}

#[allow(unused)]
fn initialize_database_with_wal(path: &Path, config: &WalConfig) -> Result<Connection> {
    let conn = Connection::open(path)?;
    
    // 1. 启用 WAL
    conn.pragma_update(None, "journal_mode", "WAL")?;
    let mode: String = conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    if mode != "wal" {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
            Some("无法启用 WAL 模式".to_string()),
        ));
    }
    println!("✅ WAL 模式已启用");
    
    // 2. 配置自动 checkpoint
    conn.pragma_update(None, "wal_autocheckpoint", config.autocheckpoint_pages)?;
    println!("✅ 自动 checkpoint: {} 页", config.autocheckpoint_pages);
    
    // 3. 配置 WAL 大小限制
    let limit_bytes = config.journal_size_limit_mb * 1024 * 1024;
    conn.pragma_update(None, "journal_size_limit", limit_bytes)?;
    println!("✅ WAL 大小限制: {} MB", config.journal_size_limit_mb);
    
    // 4. 配置同步级别
    conn.pragma_update(None, "synchronous", &config.synchronous)?;
    println!("✅ 同步级别: {}", config.synchronous);
    
    // 5. 配置缓存大小
    conn.pragma_update(None, "cache_size", config.cache_size_pages)?;
    println!("✅ 缓存大小: {} 页", config.cache_size_pages);
    
    Ok(conn)
}

#[allow(unused)]
pub fn show() -> rusqlite::Result<()>{
    let config = WalConfig::default();
    let conn = initialize_database_with_wal(Path::new("app.db"), &config)?;
    
    println!("\n📊 当前数据库配置:");
    // 验证所有配置
    let mode: String = conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    println!("  journal_mode: {}", mode);
    let auto_ckpt: u32 = conn.pragma_query_value(None, "wal_autocheckpoint", |row| row.get(0)).unwrap();
    println!("  wal_autocheckpoint: {}", auto_ckpt);
    let size_limit: u32 = conn.pragma_query_value(None, "journal_size_limit", |row| row.get(0))?;
    println!("  journal_size_limit: {} MB", size_limit / (1024 * 1024));
    let sync: u32 = conn.pragma_query_value(None, "synchronous", |row| row.get(0)).unwrap();
    println!("  synchronous: {}", sync);
    let cache: u32 = conn.pragma_query_value(None, "cache_size", |row| row.get(0))?;
    println!("  cache_size: {} 页", cache);

    Ok(())
}