
use r2d2::{ManageConnection, Pool, PooledConnection};
use rusqlite::{Connection, Error as RusqliteError, Result as RusqliteResult};
use std::path::Path;
use std::time::Duration;

#[allow(unused)]
/// 自定义的 SQLite 连接管理器。
#[derive(Clone)]
pub struct SqliteConnectionManager {
    db_path: String,
}

#[allow(unused)]
impl SqliteConnectionManager {
    /// 创建一个指向文件数据库的管理器。
    pub fn file<P: AsRef<Path>>(db_path: P) -> Self {
        Self { db_path: db_path.as_ref().to_string_lossy().into_owned() }
    }

    pub fn memory() -> Self {
        Self {
            db_path: ":memory:".to_string(),
        }
    }

    // 手动回滚连接，参数类型必须是 &mut Connection
    pub fn manual_recycle(&self, conn: &mut Connection) -> RusqliteResult<()> {
        let _ = conn.execute("ROLLBACK", []);
        Ok(())
    }
}

impl ManageConnection for SqliteConnectionManager {
    type Connection = Connection;
    type Error = RusqliteError;

    /// 1. 创建并初始化连接
    fn connect(&self) -> RusqliteResult<Self::Connection> {
        let conn = Connection::open(&self.db_path)?;
        // 在这里配置 PRAGMA，确保每次新建连接时都应用
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", 2000)?;
        // ... 可以在这里添加更多 PRAGMA 配置
        Ok(conn)
    }

    /// 2. 验证连接是否有效（健康检查）
    fn is_valid(&self, conn: &mut Self::Connection) -> RusqliteResult<()> {
        conn.query_row("SELECT 1", [], |_| Ok(()))
    }

    /// 3. 判断连接是否损坏（可选项，此处保守返回 false）
    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

// ============================================================
// 2. 连接池类型别名
// ============================================================
#[allow(unused)]
pub type DbPool = Pool<SqliteConnectionManager>;
#[allow(unused)]
pub type DbConn = PooledConnection<SqliteConnectionManager>;

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_size: u32,
    pub min_idle: u32,
    pub connection_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            min_idle: 2,
            connection_timeout_secs: 5,
            idle_timeout_secs: 300,
            max_lifetime_secs: 1800,
        }
    }
}

#[allow(unused)]
impl PoolConfig {
    pub fn read_heavy() -> Self {
        Self {
            max_size: 20,
            min_idle: 4,
            connection_timeout_secs: 3,
            idle_timeout_secs: 600,
            max_lifetime_secs: 3600,
        }
    }

    pub fn write_heavy() -> Self {
        Self {
            max_size: 5,
            min_idle: 1,
            connection_timeout_secs: 5,
            idle_timeout_secs: 120,
            max_lifetime_secs: 1800,
        }
    }
}

// ============================================================
// 3. 创建连接池的工厂函数
// ============================================================
#[allow(unused)]
pub fn create_pool(db_path: &str, config: &PoolConfig) -> RusqliteResult<DbPool> {
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::builder()
        .max_size(config.max_size)
        .min_idle(Some(config.min_idle))
        .connection_timeout(Duration::from_secs(config.connection_timeout_secs))
        .idle_timeout(Some(Duration::from_secs(config.idle_timeout_secs)))
        .max_lifetime(Some(Duration::from_secs(config.max_lifetime_secs)))
        .build(manager)
        .map_err(|e| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
                Some(format!("创建连接池失败: {}", e)),
            )
        })?;

    Ok(pool)
}

// ============================================================
// 4. 便捷函数：获取连接
// ============================================================
#[allow(unused)]
pub fn get_conn(pool: &DbPool) -> RusqliteResult<DbConn> {
    pool.get().map_err(|e| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
            Some(format!("获取连接失败: {}", e)),
        )
    })
}

// ============================================================
// 5. 连接池状态监控
// ============================================================
#[allow(unused)]
pub fn print_pool_stats(pool: &DbPool) {
    let state = pool.state();
    let active = state.connections - state.idle_connections;
    println!(
        "📊 连接池状态: 总数={}, 空闲={}, 使用中={}",
        state.connections,
        state.idle_connections,
        active
    );
}