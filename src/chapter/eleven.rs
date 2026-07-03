//在 Rust 应用中处理时间是非常常见的需求。chrono 是 Rust 生态中最流行的日期时间库，
//但 SQLite 没有原生的日期时间类型，我们需要将 chrono::DateTime<Utc> 映射到 SQLite 支持的存储类型。
//1.整数时间戳:INTEGER,优点：紧凑（8字节）、索引效率高、时区无关,缺点：不可读、精度受限（秒/毫秒）
//2.ISO 8601 字符串：TEXT，优点：可读性好、支持任意精度，缺点：占用空间大（~30字节）、字符串比较慢
//取舍建议
// 需要范围查询或排序（如 WHERE created_at > '2026-01-01'）：使用整数时间戳，索引效率更高。
// 需要可读性或调试：使用ISO 8601 字符串。
// 需要毫秒/微秒精度：整数时间戳可存储为毫秒（i64）。
//性能考量
// 整数时间戳的序列化/反序列化几乎是零成本（仅整数转换）。
// ISO 字符串需要格式化和解析，有一定开销，但可读性更好。

//Uuid（通用唯一标识符）广泛用于分布式系统中的主键或唯一标识。SQLite 没有原生 UUID 类型，
//我们需要将 uuid::Uuid 映射到 SQLite 的存储类型。
//二进制 BLOB -	BLOB    紧凑（16字节）、解析快、索引效率高	      不可读、调试困难
//文本字符串 -	TEXT    可读性好（如 550e8400-e29b-41d4-a716-446655440000）           占用空间大（36字节）、解析有开销
//取舍建议
// 主键或索引列：强烈推荐使用 BLOB，更小的存储空间和更快的比较速度。
// 需要人工查看或调试：使用 TEXT，但牺牲存储和性能。
// 数据库迁移或兼容性：TEXT 更通用，便于不同数据库间迁移。

//在业务开发中，我们经常需要存储复杂的嵌套结构体,这些结构体字段多、层级深，不适合拆分成多张表。此时，将整个结构体序列化后存入单个列是常见做法。
//1.JSON - TEXT - 大（约 150~200 字节/记录） - 可读性最好、调试方便、广泛支持 - 解析慢（文本解析）
//2.MessagePack - BLOB - 中（约 120~150 字节/记录） - 紧凑的二进制格式、跨语言支持 - 可读性差、需要额外库
//3.Bincode - BLOB - 最小（约 80~100 字节/记录）- 极其紧凑、Rust 原生、解析最快 - 仅 Rust 生态、版本兼容性需注意
//取舍建议
// 需要与其他语言（如 Python、JavaScript）共享数据：使用 JSON 或 MessagePack。
// 追求极致性能且仅在 Rust 内部使用：使用 Bincode。
// 需要人工查看或调试数据：使用 JSON（但 BLOB 列无法直接查看，需要转换）。
// 存储空间敏感：优先 Bincode，其次是 JSON

//在实现 ToSql 和 FromSql 时，我们通常直接使用 String 或 Vec<u8> 作为中间载体。但这会引入不必要的内存分配和拷贝，尤其在高吞吐场景下，这些开销会累积成性能瓶颈。
//SQLite 提供了直接操作底层存储的接口，rusqlite 也提供了相应的零拷贝抽象。
//1.ToSqlOutput::Borrowed 避免分配
// ToSql::to_sql 的返回类型是 ToSqlOutput<'a>，它可以是：
//  Owned（拥有数据，如 String、Vec<u8>）—— 会分配新内存。
//  Borrowed（借用已有数据，如 &[u8]、&str）—— 零拷贝
// 如果你的数据已经存在于某个 &[u8] 或 &str 中（如序列化后的字节切片），可以直接返回 Borrowed。
// 低效：每次都创建新的 Vec<u8>
// impl ToSql for MyStruct {
//     fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
//         let bytes = bincode::serialize(self).unwrap(); // 分配
//         Ok(ToSqlOutput::Owned(bytes)) // 再次转移所有权（但实际是移动，非拷贝）
//     }
// }
// 高效：借用已有的序列化数据（如果已经缓存）
// impl ToSql for MyStruct {
//     fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
//         // 如果 self 内部已经缓存了序列化结果，可以直接借用
//         Ok(ToSqlOutput::Borrowed(&self.cached_bytes))
//     }
// }
// 使用条件：数据必须已经存在于某个生命周期足够长的切片中。如果序列化结果需要临时生成，Owned 是不可避免的。
//2.使用 ValueRef::as_blob() 直接读取 BLOB
// FromSql::column_result 接收 ValueRef<'_>，它是对 SQLite 内部数据的借用。
// ValueRef::as_blob() 返回 &[u8]，直接指向 SQLite 的页面缓存，零拷贝。
// 从 &[u8] 反序列化（如 bincode::deserialize）时，可以避免中间的 Vec<u8> 分配。
// impl FromSql for MyStruct {
//     fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
//         let bytes = value.as_blob()?.to_vec(); // 拷贝。低效：先拷贝到 Vec<u8>
//         let bytes = value.as_blob()?; // 直接借用，无拷贝。高效：直接从借用切片反序列化
//         let s = bincode::deserialize(&bytes).unwrap();
//         Ok(s)
//     }
// }
//3.Option<T> 的零成本处理，rusqlite为Option<T>实现了ToSql和FromSql，当T是借用类型（如 &str）时，同样可以利用Borrowed。
// ToSqlOutput::Borrowed	收益：减少序列化后的内存分配	前提：已有缓存数据
// ValueRef::as_blob() 直接反序列化	    收益：减少一次Vec<u8>拷贝	 前提：所有BLOB列读取
// Option<&[u8]> 配合 Borrowed	    收益：零成本处理NULL	    前提：可为空的BLOB列
//注意：这些优化通常在数据量大（如 >1KB）或高频操作（如 >10000 QPS）时效果明显。
// 对于小数据和低频操作，差异可忽略，优先保证代码可读性。

//Rust代码部分：
use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Result, Connection,
};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, TimeZone};
use uuid::Uuid;

// ========== 1. 枚举映射 ==========
#[derive(Debug, Clone, Copy, PartialEq)]
enum UserStatus {
    Inactive = 0,
    Active = 1,
    Banned = 2,
}
impl ToSql for UserStatus {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as i32))
    }
}
impl FromSql for UserStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_i64()? {
            0 => Ok(UserStatus::Inactive),
            1 => Ok(UserStatus::Active),
            2 => Ok(UserStatus::Banned),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

// ========== 2. 结构体 JSON 映射 ==========
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UserSettings {
    theme: String,
    notifications: bool,
    language: String,
}
impl ToSql for UserSettings {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        let json = serde_json::to_string(self).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;
        Ok(ToSqlOutput::from(json))
    }
}
impl FromSql for UserSettings {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let json = value.as_str()?;
        serde_json::from_str(json).map_err(|_| FromSqlError::InvalidType)
    }
}

// ========== 3. chrono::DateTime 映射 ==========
// 策略A：整数时间戳（秒）
struct DateTimeSeconds(DateTime<Utc>);
impl ToSql for DateTimeSeconds {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.timestamp()))
    }
}
impl FromSql for DateTimeSeconds {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let ts = value.as_i64()?;
        match Utc.timestamp_opt(ts, 0).single() {
            Some(dt) => Ok(DateTimeSeconds(dt)),
            None => Err(FromSqlError::InvalidType),
        }
    }
}

// 策略B：ISO 8601 字符串
struct DateTimeString(DateTime<Utc>);
impl ToSql for DateTimeString {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.to_rfc3339()))
    }
}
impl FromSql for DateTimeString {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;
        match DateTime::parse_from_rfc3339(s) {
            Ok(dt) => Ok(DateTimeString(dt.with_timezone(&Utc))),
            Err(_) => Err(FromSqlError::InvalidType),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(unused)]
struct UuidWrapper(Uuid);

impl UuidWrapper {
    // 创建新的 v4 UUID
    #[allow(unused)]
    fn new_v4() -> Self {
        UuidWrapper(Uuid::new_v4())
    }
    
    // 从 Uuid 创建
    #[allow(unused)]
    fn from_uuid(uuid: Uuid) -> Self {
        UuidWrapper(uuid)
    }
    
    // 获取内部 Uuid 的引用
    #[allow(unused)]
    fn as_uuid(&self) -> &Uuid {
        &self.0
    }
    
    // 获取内部 Uuid（消耗包裹）
    #[allow(unused)]
    fn into_uuid(self) -> Uuid {
        self.0
    }
}

// ========== 4. uuid::Uuid 映射 ==========
// 策略A：BLOB（16字节）
impl ToSql for UuidWrapper {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        let bytes: &[u8] = self.0.as_bytes();
        Ok(ToSqlOutput::from(bytes))
    }
}

impl FromSql for UuidWrapper {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let bytes = value.as_blob()?;
        if bytes.len() == 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(bytes);
            match Uuid::from_slice(&arr) {
                Ok(uuid) => Ok(UuidWrapper(uuid)),
                Err(_) => Err(FromSqlError::InvalidType),
            }
        } else {
            // 也支持从 TEXT 解析（可选）
            if let Ok(s) = value.as_str() {
                match Uuid::parse_str(s) {
                    Ok(uuid) => Ok(UuidWrapper(uuid)),
                    Err(_) => Err(FromSqlError::InvalidType),
                }
            } else {
                Err(FromSqlError::InvalidType)
            }
        }
    }
}

// ========== 5. 复杂结构体序列化 ==========
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[allow(unused)]
pub struct UserMetadata {
    pub user_id: i64,
    pub name: String,
    pub tags: Vec<String>,
    pub settings: UserSettings,
    pub created_at: DateTime<Utc>,
}

// 序列化辅助函数
#[allow(unused)]
pub fn serialize_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(e))
    })
}
#[allow(unused)]
pub fn deserialize_json<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
    })
}

impl ToSql for UserMetadata {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        let bytes = serialize_json(self)?;
        Ok(ToSqlOutput::from(bytes))
    }
}
impl FromSql for UserMetadata {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let bytes = value.as_blob()?;
        deserialize_json(bytes).map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

// 其他序列化格式（MessagePack, Bincode）
#[allow(unused)]
pub fn serialize_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    rmp_serde::to_vec(value).map_err(|e| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(e))
    })
}
#[allow(unused)]
pub fn deserialize_msgpack<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    rmp_serde::from_read(bytes).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
    })
}
#[allow(unused)]
pub fn serialize_bincode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    bincode::serialize(value).map_err(|e| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(e))
    })
}
#[allow(unused)]
pub fn deserialize_bincode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    bincode::deserialize(bytes).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
    })
}

// ========== 6. 优化版本：使用 Borrowed ==========
#[allow(unused)]
pub struct UserMetadataOptimized {
    pub data: UserMetadata,
    pub cached_json: Option<Vec<u8>>, // 缓存序列化结果
}
impl ToSql for UserMetadataOptimized {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        if let Some(bytes) = &self.cached_json {
            // 正确写法：直接借用 Vec<u8>
            Ok(ToSqlOutput::from(bytes.as_slice()))
        } else {
            let bytes = serialize_json(&self.data)?;
            Ok(ToSqlOutput::from(bytes))
        }
    }
}
impl FromSql for UserMetadataOptimized {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let bytes = value.as_blob()?;
        let data = deserialize_json(bytes).map_err(|e| FromSqlError::Other(Box::new(e)))?;
        Ok(UserMetadataOptimized {
            data,
            cached_json: None, // 读取时不缓存
        })
    }
}

#[allow(unused)]
// ========== 7. 辅助函数：建表与插入 ==========
fn setup_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DROP TABLE IF EXISTS test;
         CREATE TABLE test (
             id INTEGER PRIMARY KEY,
             status INTEGER,
             settings TEXT,
             created_at_seconds INTEGER,
             created_at_string TEXT,
             uuid_blob BLOB,
             metadata BLOB
         );",
    )?;
    Ok(())
}

#[allow(unused)]
fn insert_sample_data(conn: &Connection) -> Result<()> {
    let status = UserStatus::Active;
    let settings = UserSettings {
        theme: "dark".to_string(),
        notifications: true,
        language: "en".to_string(),
    };
    let dt = Utc::now();
    let uuid = UuidWrapper::new_v4();
    let metadata = UserMetadata {
        user_id: 1,
        name: "Alice".to_string(),
        tags: vec!["rust".to_string(), "sqlite".to_string()],
        settings: settings.clone(),
        created_at: dt,
    };

    conn.execute(
        "INSERT INTO test (status, settings, created_at_seconds, created_at_string, uuid_blob, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            status,
            settings,
            DateTimeSeconds(dt),
            DateTimeString(dt),
            uuid,
            metadata,
        ),
    )?;
    Ok(())
}

#[allow(unused)]
fn query_sample_data(conn: &Connection) -> Result<()> {
    let row = conn.query_row(
        "SELECT status, settings, created_at_seconds, created_at_string, uuid_blob, metadata
         FROM test LIMIT 1",
        [],
        |row| {
            Ok((
                row.get::<_, UserStatus>(0)?,
                row.get::<_, UserSettings>(1)?,
                row.get::<_, DateTimeSeconds>(2)?,
                row.get::<_, DateTimeString>(3)?,
                row.get::<_, UuidWrapper>(4)?,
                row.get::<_, UserMetadata>(5)?,
            ))
        },
    )?;

    let (status, settings, dt_sec, dt_str, uuid, metadata) = row;
    println!("Status: {:?}", status);
    println!("Settings: {:?}", settings);
    println!("DateTime (seconds): {}", dt_sec.0);
    println!("DateTime (string): {}", dt_str.0);
    println!("UUID: {:?}", uuid);
    println!("Metadata: {:?}", metadata);
    Ok(())
}

#[allow(unused)]
pub fn show(){
    let conn = Connection::open_in_memory().unwrap();
    let _ = setup_table(&conn);
    let _ = insert_sample_data(&conn);
    let _ = query_sample_data(&conn);
}