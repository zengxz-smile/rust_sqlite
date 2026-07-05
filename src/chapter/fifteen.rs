//安全类 PRAGMA 概览
// PRAGMA	                            作用	                默认值	                    推荐值
// synchronous	                   控制数据同步级别	              FULL	                NORMAL（WAL 模式）
// foreign_keys	                   启用/禁用外键约束	          OFF	                      ON
// ignore_check_constraints	       临时忽略CHECK 约束	          OFF	               OFF（仅迁移时 ON）
// recursive_triggers	            启用递归触发器	              OFF	                  ON（如需递归）

//synchronous —— 同步级别
// ┌─────────────────────────────────────────────────────────────────
// │                    synchronous = OFF
// │  行为：事务提交时，数据留在操作系统缓存，不调用 fsync()
// │  风险：操作系统崩溃 → 数据可能损坏
// │  性能：★★★★★ 最快
// │  适用：临时数据库、只读副本、可重新生成的数据
// ├─────────────────────────────────────────────────────────────────
// │                    synchronous = NORMAL
// │  行为：WAL 模式下，仅 fsync WAL 文件，不 fsync 主数据库
// │  风险：操作系统崩溃 → 可能丢失 WAL 中未刷新的数据
// │  性能：★★★★ 较快
// │  适用：生产环境（与 WAL 配合使用）← 推荐
// ├─────────────────────────────────────────────────────────────────
// │                    synchronous = FULL
// │  行为：每次事务提交，fsync WAL 文件 + 主数据库文件
// │  风险：操作系统崩溃 → 零数据丢失
// │  性能：★★★ 较慢
// │  适用：金融、医疗、强一致性场景
// └─────────────────────────────────────────────────────────────────
//关键理解：
// fsync() 是系统调用，强制将数据从操作系统缓存写入物理磁盘
// 每次 fsync() 耗时约 0.5-2ms（SSD）或 5-10ms（HDD）
// FULL 模式每次事务提交执行 2 次 fsync，NORMAL 只执行 1 次

//foreign_keys —— 外键约束
// 默认 OFF（为了向后兼容）
// 启用后，INSERT/UPDATE/DELETE 会检查外键约束
// 检查开销：每次操作增加 5-15% CPU 开销

//ignore_check_constraints —— 临时忽略 CHECK 约束
// -- 定义表时包含 CHECK 约束
// CREATE TABLE users (
//     id INTEGER PRIMARY KEY,
//     age INTEGER CHECK (age >= 0 AND age <= 150)
// );
// -- 临时忽略 CHECK 约束
// PRAGMA ignore_check_constraints = ON;
// INSERT INTO users (age) VALUES (-1); -- 原本会失败，现在成功
// PRAGMA ignore_check_constraints = OFF;
//Rust伪代码：
// fn with_check_constraints_ignored<F, T>(conn: &Connection, f: F) -> Result<T>
// where
//     F: FnOnce(&Connection) -> Result<T>,
// {
//     conn.pragma_update(None, "ignore_check_constraints", true)?;
//     let result = f(conn)?;
//     conn.pragma_update(None, "ignore_check_constraints", false)?;
//     Ok(result)
// }

//recursive_triggers —— 递归触发器
//Rust伪代码：
// fn enable_recursive_triggers(conn: &Connection) -> Result<()> {
//     conn.pragma_update(None, "recursive_triggers", true)?;
//     let status: i32 = conn.pragma_query_value(None, "recursive_triggers", |row| row.get(0))?;
//     println!("recursive_triggers = {}", if status == 1 { "ON" } else { "OFF" });
//     Ok(())
// }

//PRAGMA 调优参数（cache_size、page_size、mmap_size）
// 一、cache_size —— 页面缓存大小
// cache_size 控制 SQLite 页面缓存（Page Cache） 中存储的页面数量。页面缓存是 SQLite 的"内存缓冲区"，存储最近访问的数据库页面。
//关键理解：
// 缓存命中 → 直接从内存读取（~100ns）
// 缓存未命中 → 从磁盘读取（~10ms SSD / ~100ms HDD）
// 缓存命中率直接影响读取性能
//默认值：2000 页（约 8MB，按 4KB 页面计算）
//如何选择cache_size？
//系统内存	    推荐 cache_size	            缓存大小
// 512MB	      4000-8000	               16-32MB
// 1GB	          8000-20000	           32-80MB
// 2GB	         20000-50000	           80-200MB
// 4GB+	         50000-100000	          200-400MB
//二、page_size —— 页面大小
// page_size 定义 SQLite 数据库文件的基本 I/O 单位。所有数据库读写都以页面为单位进行。
//关键理解：
// 页面大小决定了单个 I/O 操作的数据量
// 更大的页面 → 更少的 B-tree 层级 → 更快的查询
// 但更大的页面 → 缓存可容纳的页面数更少 → 可能降低缓存命中率
//默认值为 4096 字节（与大多数文件系统一致）
//三、mmap_size —— 内存映射 I/O
// mmap_size 控制 SQLite 使用内存映射 I/O 读取数据库文件的最大大小。
// 传统读取：
// ┌─────────┐    read()    ┌─────────┐
// │  SQLite │ ───────────→ │  内核   │ → 磁盘
// │         │ ←─────────── │  缓存   │
// └─────────┘   返回数据    └─────────┘
// 每次 read() 系统调用都有开销
// 内存映射：
// ┌─────────┐   直接访问   ┌─────────┐
// │  SQLite │ ───────────→ │  内存   │ ← 页面错误 → 磁盘
// │         │              │  映射   │
// └─────────┘              └─────────┘
// 减少系统调用，由操作系统管理页面
//缺点：
// 32 位系统虚拟地址空间有限
// 写入时可能导致页面错误
// 内存映射区域过大可能导致内存压力
//推荐配置 - 仅在 64 位系统上启用
// 数据库大小	        推荐 mmap_size	                说明
// < 100MB	              0（禁用）	               映射开销不划算
// 100MB - 1GB	           256MB	               映射前 256MB
// 1GB - 10GB	        512MB - 1GB	                映射前 1GB
// > 10GB	             1GB - 2GB	                 按需映射
//Rust伪代码
// fn configure_mmap(conn: &Connection, size_mb: u32) -> Result<()> {
//     // 仅在 64 位系统上启用
//     #[cfg(target_pointer_width = "64")]
//     {
//         let bytes = size_mb * 1024 * 1024;
//         conn.pragma_update(None, "mmap_size", bytes)?;
//         let actual: u32 = conn.pragma_query_value(None, "mmap_size", |row| row.get(0))?;
//         println!("mmap_size = {} MB", actual / (1024 * 1024));
//     }
    
//     #[cfg(target_pointer_width = "32")]
//     {
//         eprintln!("32 位系统不建议启用 mmap");
//         conn.pragma_update(None, "mmap_size", 0)?;
//     }
//     Ok(())
// }

//Rust代码部分：
use rusqlite::{Connection, Result};

#[allow(unused)]
struct SafetyConfig {
    synchronous: i32, // 0=OFF, 1=NORMAL, 2=FULL
    foreign_keys: bool,
    ignore_check_constraints: bool,
    recursive_triggers: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            synchronous: 1, // NORMAL
            foreign_keys: true,
            ignore_check_constraints: false,
            recursive_triggers: false,
        }
    }
}

#[allow(unused)]
fn apply_safety_config(conn: &Connection, config: &SafetyConfig) -> Result<()> {
    conn.pragma_update(None, "synchronous", config.synchronous)?;
    conn.pragma_update(None, "foreign_keys", config.foreign_keys)?;
    conn.pragma_update(
        None,
        "ignore_check_constraints",
        config.ignore_check_constraints,
    )?;
    conn.pragma_update(None, "recursive_triggers", config.recursive_triggers)?;
    Ok(())
}

#[allow(unused)]
fn verify_safety_config(conn: &Connection) -> Result<()> {
    println!("\n📊 安全类 PRAGMA 配置:");
    let sync: i32 = conn.pragma_query_value(None, "synchronous", |row| row.get(0))?;
    println!(
        "  synchronous: {} ({})",
        sync,
        match sync {
            0 => "OFF",
            1 => "NORMAL",
            2 => "FULL",
            _ => "UNKNOWN",
        }
    );
    let fk: i32 = conn.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
    println!("  foreign_keys: {}", if fk == 1 { "ON" } else { "OFF" });
    let ignore: i32 =
        conn.pragma_query_value(None, "ignore_check_constraints", |row| row.get(0))?;
    println!(
        "  ignore_check_constraints: {}",
        if ignore == 1 { "ON" } else { "OFF" }
    );
    let recursive: i32 = conn.pragma_query_value(None, "recursive_triggers", |row| row.get(0))?;
    println!(
        "  recursive_triggers: {}",
        if recursive == 1 { "ON" } else { "OFF" }
    );
    Ok(())
}

#[allow(unused)]
struct PerformanceConfig {
    cache_size_pages: u32,
    mmap_size_bytes: u32,
    temp_store: i32,       // 0=DEFAULT, 1=FILE, 2=MEMORY
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            cache_size_pages: 2000,
            mmap_size_bytes: 64 * 1024 * 1024, // 64MB
            temp_store: 2, // MEMORY
        }
    }
}
#[allow(unused)]
impl PerformanceConfig {
    fn for_read_heavy(total_memory_mb: u64) -> Self {
        let cache_pages = ((total_memory_mb * 20 / 100) * 1024 * 1024) / 4096;
        Self {
            cache_size_pages: cache_pages.min(500_000) as u32,
            mmap_size_bytes: 256 * 1024 * 1024,
            temp_store: 2,
        }
    }
    
    fn for_write_heavy(total_memory_mb: u64) -> Self {
        let cache_pages = ((total_memory_mb * 10 / 100) * 1024 * 1024) / 4096;
        Self {
            cache_size_pages: cache_pages.min(200_000) as u32,
            mmap_size_bytes: 0, // 写入密集型禁用 mmap
            temp_store: 2,
        }
    }
}
#[allow(unused)]
fn apply_performance_config(conn: &Connection, config: &PerformanceConfig) -> Result<()> {
    conn.pragma_update(None, "cache_size", config.cache_size_pages)?;
    conn.pragma_update(None, "mmap_size", config.mmap_size_bytes)?;
    conn.pragma_update(None, "temp_store", config.temp_store)?;
    Ok(())
}

#[allow(unused)]
pub fn show() {
    let config = SafetyConfig::default();
    let conn = Connection::open("app.db").unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    let _ = apply_safety_config(&conn, &config);
    let _ = verify_safety_config(&conn);
}

    //                     ┌─────────────────┐
    //                     │   选择配置场景    │
    //                     └────────┬────────┘
    //                              │
    //         ┌────────────────────┼────────────────────┐
    //         │                    │                    │
    //         ▼                    ▼                    ▼
    // ┌───────────────┐    ┌───────────────┐    ┌───────────────┐
    // │  读密集型应用  │    │  写密集型应用  │    │  混合型应用   │
    // └───────┬───────┘    └───────┬───────┘    └───────┬───────┘
    //         │                    │                    │
    //         ▼                    ▼                    ▼
    // ┌───────────────┐    ┌───────────────┐    ┌───────────────┐
    // │ cache_size ↑  │    │ cache_size ↓  │    │ cache_size 中 │
    // │ mmap_size ↑↑  │    │ mmap_size 0   │    │ mmap_size 中  │
    // │ wal_ckpt 中   │    │ wal_ckpt ↓    │    │ wal_ckpt 中   │
    // │ limit 中      │    │ limit ↓       │    │ limit 中      │
    // └───────────────┘    └───────────────┘    └───────────────┘
    //         │                    │                    │
    //         └────────────────────┼────────────────────┘
    //                              │
    //                              ▼
    //                     ┌───────────────┐
    //                     │  同步级别:    │
    //                     │  NORMAL       │
    //                     │  (WAL 模式)   │
    //                     └───────────────┘