//PRAGMA 调优参数 —— 性能类 PRAGMA II（temp_store、locking_mode、journal_size_limit）
//一、temp_store —— 临时表存储位置
//temp_store 控制 SQLite 在处理临时表、临时索引和子查询结果集时，将数据存储在何处。
//temp_store有三种模式：DEFAULT、FILE、MEMORY
// 模式 - 值 - 存储位置 - 性能 - 内存占用 - 适用场景
// DEFAULT - 0 - 由 temp_store_directory 决定（通常是磁盘） - 较慢 - 低 - 内存受限环境
// FILE - 1 - 磁盘文件（<数据库名>-journal 同目录） - 中等 - 低 - 大型临时数据
// MEMORY - 2 - 内存（RAM）	极快 - 高 - 推荐生产环境
//关键理解：
// 临时表在内存中时，操作速度可达磁盘的 10-100 倍
// 但内存有限，大型临时表可能导致 SQLITE_NOMEM 错误
// 操作系统会使用交换空间，所以 MEMORY 模式不保证永不失败
//temp_store = DEFAULT 时，SQLite 使用 PRAGMA temp_store_directory 指定的目录创建临时文件。
//建议（仅供参考）
// 场景 - 推荐 - temp_store - 说明
// 临时表 < 100MB - MEMORY - 内存足够，速度最快
// 临时表 > 100MB - FILE -	避免内存耗尽
// 内存受限（< 512MB） - DEFAULT 或 FILE - 保守策略
// 生产环境 - MEMORY - 默认推荐
//Rust配置方法：conn.pragma_update(None, "temp_store", value)?;

//二、locking_mode —— 锁定模式
// locking_mode 控制 SQLite 如何管理数据库锁，影响多个连接之间的并发行为。
// 模式 - 值 - 行为 - 并发能力 - 适用场景
// NORMAL - 0 - 正常锁定，按需升级/降级锁 - 多连接读写 - 多用户应用
// EXCLUSIVE - 1 - 事务开始时获取排他锁，直到结束 - 单连接读写 - 单用户写入场景
//关键理解：
// NORMAL 模式下，锁会按需升级：UNLOCKED → SHARED → RESERVED → PENDING → EXCLUSIVE
// EXCLUSIVE 模式下，事务开始时直接获取 EXCLUSIVE 锁，避免锁升级的开销
// 但 EXCLUSIVE 会阻塞其他连接的所有操作
//建议（仅供参考）
// 场景 - 推荐 locking_mode - 说明
// 单用户应用 - EXCLUSIVE - 减少锁开销，提升性能
// 多用户应用 - NORMAL - 允许并发读取
// 读写分离 - NORMAL - 允许读取
// 高并发读取 - NORMAL - 共享锁允许并发读取
//Rust配置方法：conn.pragma_update(None, "locking_mode", value)?;

//三、journal_size_limit —— 日志文件大小限制
// journal_size_limit 控制 WAL 文件（或回滚日志文件）的最大大小。当日志文件达到此限制时，
// SQLite 会自动执行 checkpoint（WAL 模式）或截断日志（回滚模式）。
//关键理解：
// 在 WAL 模式下，此参数限制 .db-wal 文件的大小
// 当 WAL 文件达到限制时，SQLite 强制执行 checkpoint
// 防止 WAL 文件无限膨胀
//默认值：
// WAL 模式：无限制（-1）
// 回滚模式：无限制（-1）
//Rust配置方法：conn.pragma_update(None, "journal_size_limit", value)?;
//建议（仅供参考）
// 数据库大小 - 推荐 journal_size_limit - 说明
// < 100MB - 32MB - 较小的限制
// 100MB - 1GB - 64MB - 平衡
// 1GB - 10GB - 128MB - 较大限制
// > 10GB - 256MB - 512MB - 需要足够空间

//PRAGMA 调优参数 —— 诊断类 PRAGMA
//SQLite 的诊断类 PRAGMA——这些参数帮助您检测数据库健康状态、监控变化、验证完整性。它们是生产环境运维的“眼睛”和“耳朵”。
//一、诊断类 PRAGMA 概览
// PRAGMA	-作用	-执行速度	-生产环境使用频率
// integrity_check	-完整检查数据库结构一致性	-慢（全表扫描）	-定期维护（如每日）
// quick_check	-快速检查数据库结构	-快（抽样检查）	-高频检查（如每小时）
// data_version	-数据变更版本号	-极快（读整数）	-缓存失效判断
// schema_version	-Schema 变更版本号	-极快（读整数）	-迁移检测

//二、integrity_check —— 完整性检查
//PRAGMA integrity_check 扫描整个数据库，验证：
// 所有页面的校验和
// B-tree 结构的完整性
// 索引与数据的一致性
// 外键约束的完整性（如果启用）
//执行流程：1. 检查数据库头 → 2. 检查所有页面 → 3. 验证 B-tree → 4. 验证索引 → 5. 返回结果
//返回值： "ok"：一切正常; 其他字符串：错误描述（如 "*** in database main page 123: ..."）
//Rust操作示例：let result: String = conn.pragma_query_value(None, "integrity_check", |row| row.get(0))?;

//三、quick_check —— 快速完整性检查
// PRAGMA quick_check 是 integrity_check 的轻量级版本，它只检查 B-tree 的结构而不深入验证每个数据页。
//Rust操作示例：let result: String = conn.pragma_query_value(None, "quick_check", |row| row.get(0))?;

//四、data_version —— 数据版本号
//PRAGMA data_version 返回一个64 位整数，每次数据库内容变化时递增。它可用于：
// 缓存失效：当版本号变化时，通知应用刷新缓存
// 变更检测：检测其他连接对数据库的修改
// 增量同步：检测哪些数据需要同步
//关键理解：
// 版本号在每个事务提交时递增
// 读操作（SELECT）不会改变版本号
// 版本号仅在当前连接可见（不同连接看到的值不同）
//Rust操作示例：let result: String = conn.pragma_query_value(None, "data_version", |row| row.get(0))?;

//五、schema_version —— Schema 版本号
//PRAGMA schema_version 返回一个32 位整数，每次 Schema 变更时递增。它用于：
// 迁移检测：检测 Schema 是否与代码期望的一致
// DDL 操作追踪：监控 CREATE/ALTER/DROP 操作
// 连接一致性：确保所有连接使用相同的 Schema
//Rust操作示例：let result: String = conn.pragma_query_value(None, "schema_version", |row| row.get(0))?;

use rusqlite::{Connection, Result};
use std::path::Path;

#[allow(unused)]
#[derive(Debug, Clone)]
struct DatabaseConfig {
    // 安全类
    synchronous: i32, // 0=OFF, 1=NORMAL, 2=FULL
    foreign_keys: bool,
    ignore_check_constraints: bool,
    recursive_triggers: bool,

    // 性能类
    cache_size_pages: u32,
    mmap_size_bytes: u32,
    temp_store: i32,   // 0=DEFAULT, 1=FILE, 2=MEMORY
    locking_mode: i32, // 0=NORMAL, 1=EXCLUSIVE
    journal_size_limit_bytes: u32,
    wal_autocheckpoint_pages: u32,
}

#[allow(unused)]
impl DatabaseConfig {
    /// 通用生产环境配置（推荐）
    fn production_general(total_memory_mb: u64) -> Self {
        let cache_pages = ((total_memory_mb * 15 / 100) * 1024 * 1024) / 4096;
        let cache_pages = cache_pages.clamp(2000, 500000) as u32;

        Self {
            // 安全类
            synchronous: 1, // NORMAL
            foreign_keys: true,
            ignore_check_constraints: false,
            recursive_triggers: false,

            // 性能类
            cache_size_pages: cache_pages,
            mmap_size_bytes: 64 * 1024 * 1024,
            temp_store: 2,   // MEMORY
            locking_mode: 0, // NORMAL
            journal_size_limit_bytes: 64 * 1024 * 1024,
            wal_autocheckpoint_pages: 1000,
        }
    }

    /// 读密集型配置（报表、分析）
    fn read_heavy(total_memory_mb: u64) -> Self {
        let cache_pages = ((total_memory_mb * 25 / 100) * 1024 * 1024) / 4096;
        let cache_pages = cache_pages.clamp(2000, 500000) as u32;

        Self {
            synchronous: 1,
            foreign_keys: true,
            ignore_check_constraints: false,
            recursive_triggers: false,

            cache_size_pages: cache_pages,      // 大缓存
            mmap_size_bytes: 256 * 1024 * 1024, // 大内存映射
            temp_store: 2,
            locking_mode: 0,
            journal_size_limit_bytes: 128 * 1024 * 1024,
            wal_autocheckpoint_pages: 2000, // 更少 checkpoint
        }
    }

    /// 写密集型配置（日志、采集）
    fn write_heavy(total_memory_mb: u64) -> Self {
        let cache_pages = ((total_memory_mb * 10 / 100) * 1024 * 1024) / 4096;
        let cache_pages = cache_pages.clamp(1000, 200000) as u32;

        Self {
            synchronous: 1,
            foreign_keys: true,
            ignore_check_constraints: false,
            recursive_triggers: false,

            cache_size_pages: cache_pages, // 较小缓存
            mmap_size_bytes: 0,            // 禁用 mmap（写入场景）
            temp_store: 2,
            locking_mode: 0,
            journal_size_limit_bytes: 32 * 1024 * 1024, // 更小的 WAL 限制
            wal_autocheckpoint_pages: 500,              // 更频繁 checkpoint
        }
    }

    /// 单用户应用配置（嵌入式）
    fn single_user(total_memory_mb: u64) -> Self {
        let cache_pages = ((total_memory_mb * 10 / 100) * 1024 * 1024) / 4096;
        let cache_pages = cache_pages.clamp(2000, 200000) as u32;

        Self {
            synchronous: 1,
            foreign_keys: true,
            ignore_check_constraints: false,
            recursive_triggers: false,

            cache_size_pages: cache_pages,
            mmap_size_bytes: 64 * 1024 * 1024,
            temp_store: 2,
            locking_mode: 1, // EXCLUSIVE（单用户）
            journal_size_limit_bytes: 64 * 1024 * 1024,
            wal_autocheckpoint_pages: 1000,
        }
    }

    /// 内存受限配置（嵌入式设备）
    fn memory_limited(total_memory_mb: u64) -> Self {
        let cache_pages = ((total_memory_mb * 5 / 100) * 1024 * 1024) / 4096;
        let cache_pages = cache_pages.clamp(500, 20000) as u32;

        Self {
            synchronous: 1,
            foreign_keys: true,
            ignore_check_constraints: false,
            recursive_triggers: false,

            cache_size_pages: cache_pages,
            mmap_size_bytes: 0, // 禁用 mmap
            temp_store: 1,      // FILE（内存不足）
            locking_mode: 0,
            journal_size_limit_bytes: 16 * 1024 * 1024,
            wal_autocheckpoint_pages: 500,
        }
    }
}

#[allow(unused)]
impl DatabaseConfig {
    /// 应用所有配置到数据库连接
    fn apply(&self, conn: &Connection) -> Result<()> {
        // 安全类 PRAGMA
        conn.pragma_update(None, "synchronous", self.synchronous)?;
        conn.pragma_update(None, "foreign_keys", self.foreign_keys)?;
        conn.pragma_update(
            None,
            "ignore_check_constraints",
            self.ignore_check_constraints,
        )?;
        conn.pragma_update(None, "recursive_triggers", self.recursive_triggers)?;

        // 性能类 PRAGMA
        conn.pragma_update(None, "cache_size", self.cache_size_pages)?;
        conn.pragma_update(None, "mmap_size", self.mmap_size_bytes)?;
        conn.pragma_update(None, "temp_store", self.temp_store)?;
        conn.pragma_update(None, "locking_mode", self.locking_mode)?;
        conn.pragma_update(None, "journal_size_limit", self.journal_size_limit_bytes)?;
        conn.pragma_update(None, "wal_autocheckpoint", self.wal_autocheckpoint_pages)?;

        println!("✅ 数据库配置已应用");
        Ok(())
    }

    /// 验证配置是否生效
    fn verify(&self, conn: &Connection) -> Result<bool> {
        let mut all_match = true;

        // 验证每个 PRAGMA 的实际值
        let actual_sync: i32 = conn.pragma_query_value(None, "synchronous", |row| row.get(0))?;
        if actual_sync != self.synchronous {
            println!(
                "⚠️ synchronous: 期望 {}, 实际 {}",
                self.synchronous, actual_sync
            );
            all_match = false;
        }

        let actual_fk: i32 = conn.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
        let expected_fk = if self.foreign_keys { 1 } else { 0 };
        if actual_fk != expected_fk {
            println!("⚠️ foreign_keys: 期望 {}, 实际 {}", expected_fk, actual_fk);
            all_match = false;
        }

        let actual_cache: u32 = conn.pragma_query_value(None, "cache_size", |row| row.get(0))?;
        if actual_cache != self.cache_size_pages {
            println!(
                "⚠️ cache_size: 期望 {}, 实际 {}",
                self.cache_size_pages, actual_cache
            );
            all_match = false;
        }

        let actual_mmap: u32 = conn.pragma_query_value(None, "mmap_size", |row| row.get(0))?;
        if actual_mmap != self.mmap_size_bytes {
            println!(
                "⚠️ mmap_size: 期望 {}, 实际 {}",
                self.mmap_size_bytes, actual_mmap
            );
            all_match = false;
        }

        let actual_limit: u32 =
            conn.pragma_query_value(None, "journal_size_limit", |row| row.get(0))?;
        if actual_limit != self.journal_size_limit_bytes {
            println!(
                "⚠️ journal_size_limit: 期望 {}, 实际 {}",
                self.journal_size_limit_bytes, actual_limit
            );
            all_match = false;
        }

        if all_match {
            println!("✅ 所有配置验证通过");
        }
        Ok(all_match)
    }

    /// 打印当前配置报告
    fn print_report(conn: &Connection) -> Result<()> {
        let sync: i32 = conn.pragma_query_value(None, "synchronous", |row| row.get(0))?;
        let fk: i32 = conn.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
        let cache: u32 = conn.pragma_query_value(None, "cache_size", |row| row.get(0))?;
        let page_size: u32 = conn.pragma_query_value(None, "page_size", |row| row.get(0))?;
        let mmap: u32 = conn.pragma_query_value(None, "mmap_size", |row| row.get(0))?;
        let temp: i32 = conn.pragma_query_value(None, "temp_store", |row| row.get(0))?;
        let lock: String = conn.pragma_query_value(None, "locking_mode", |row| row.get(0))?;
        let limit: u32 = conn.pragma_query_value(None, "journal_size_limit", |row| row.get(0))?;
        let wal_ckpt: u32 =
            conn.pragma_query_value(None, "wal_autocheckpoint", |row| row.get(0))?;

        let cache_mb = (cache * page_size) as f64 / (1024.0 * 1024.0);
        let limit_mb = if limit == 0 {
            "无限制".to_string()
        } else {
            format!("{} MB", limit / (1024 * 1024))
        };

        println!("\n📊 当前数据库 PRAGMA 配置报告");
        println!("═══════════════════════════════════════════");
        println!("🔒 安全类:");
        println!(
            "  synchronous           = {} ({})",
            sync,
            match sync {
                0 => "OFF",
                1 => "NORMAL",
                2 => "FULL",
                _ => "UNKNOWN",
            }
        );
        println!(
            "  foreign_keys          = {}",
            if fk == 1 { "ON" } else { "OFF" }
        );
        println!();
        println!("⚡ 性能类:");
        println!(
            "  cache_size            = {} 页 ({:.1} MB)",
            cache, cache_mb
        );
        println!("  page_size             = {} 字节", page_size);
        println!("  mmap_size             = {} MB", mmap / (1024 * 1024));
        println!(
            "  temp_store            = {} ({})",
            temp,
            match temp {
                0 => "DEFAULT",
                1 => "FILE",
                2 => "MEMORY",
                _ => "UNKNOWN",
            }
        );
        println!("  locking_mode          = {lock}");
        // println!(
        //     "  locking_mode          = {} ({})",
        //     lock,
        //     match lock {
        //         0 => "NORMAL",
        //         1 => "EXCLUSIVE",
        //         _ => "UNKNOWN",
        //     }
        // );
        println!("  journal_size_limit    = {}", limit_mb);
        println!("  wal_autocheckpoint    = {} 页", wal_ckpt);
        println!();
        println!("📊 数据库统计:");
        let page_count: u32 = conn.pragma_query_value(None, "page_count", |row| row.get(0))?;
        let freelist: u32 = conn.pragma_query_value(None, "freelist_count", |row| row.get(0))?;
        let db_size = (page_count * page_size) as f64 / (1024.0 * 1024.0);
        println!("  数据库大小           = {:.2} MB", db_size);
        println!(
            "  空闲页面             = {} ({:.1}%)",
            freelist,
            if page_count > 0 {
                (freelist as f64 / page_count as f64) * 100.0
            } else {
                0.0
            }
        );

        Ok(())
    }
}

#[allow(unused)]
pub fn show() {
    let path = Path::new("app.db");
    println!("🚀 初始化数据库: {}", path.display());

    // 1. 打开连接
    let mut conn = Connection::open(path).unwrap();
    println!("✅ 数据库连接已打开");

    // 2. 启用 WAL 模式（必须先于其他配置）
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    let mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    if mode != "wal" {
        panic!("无法启用 WAL 模式");
    }
    println!("✅ WAL 模式已启用");

    //Web应用（混合负载）
    let memory_mb = 2048; // 假设 2GB 内存
    let config = DatabaseConfig::production_general(memory_mb);

    //报表系统（读密集型）
    // let memory_mb = 4096; // 假设 4GB 内存
    // let config = DatabaseConfig::read_heavy(memory_mb);

    //日志采集（写密集型）
    // let memory_mb = 1024; // 假设 1GB 内存
    // let config = DatabaseConfig::write_heavy(memory_mb);

    //嵌入式设备（内存受限）
    // let memory_mb = 256; // 假设 256MB 内存
    // let config = DatabaseConfig::memory_limited(memory_mb);

    // 3. 应用 PRAGMA 配置
    config.apply(&conn).unwrap();
    
    // 4. 验证配置
    config.verify(&conn).unwrap();
    
    // 5. 打印配置报告
    DatabaseConfig::print_report(&conn).unwrap();
}
