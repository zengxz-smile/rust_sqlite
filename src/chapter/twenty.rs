//扩展 - 自定义 SQL 函数
//前面已经实现了自定义聚合函数，本期视频将补齐另外两种自定义函数：
// 标量函数：对每行执行计算，返回一个值（如 UPPER、LENGTH）
// 窗口函数：在窗口帧上执行计算（如 ROW_NUMBER） - 不建议
//一、自定义标量函数（Scalar Function）
//标量函数接收若干参数，返回一个单一值。
//SELECT UPPER(name) FROM users;  -- UPPER 是内置标量函数
//SELECT MY_FUNC(col1, col2) FROM t;  -- MY_FUNC 是自定义

// pub fn create_scalar_function<F, N: Name, T>( -- 在 rusqlite 0.40.1 中，create_scalar_function是注册标量函数
//     &self,
//     fn_name: N,
//     n_arg: c_int,
//     flags: FunctionFlags,
//     x_func: F,
// ) -> Result<()>
// where
//     F: Fn(&Context<'_>) -> Result<T> + Send + 'static,
//     T: SqlFnOutput,
//返回 rusqlite::Result<Value>，其中 Value 可以是：
// Value::Null
// Value::Integer(i64)
// Value::Real(f64)
// Value::Text(String)
// Value::Blob(Vec<u8>)
// 注意事项	                    说明
// 确定性	                    如果函数对于相同输入总是返回相同结果，应设置 SQLITE_DETERMINISTIC 标志，有助于查询优化
// 不要阻塞	                    函数内避免长时间阻塞（如网络 I/O）
// 避免内存分配	                高频函数中尽量减少分配
// 使用 Rust 的类型	            优先使用 i64、f64、String 等原生类型，避免过多转换

use regex::Regex;
use rusqlite::{Connection, Result, functions::FunctionFlags}; //正则表达式

#[allow(unused)]
fn register_regexp(conn: &Connection) -> Result<()> {
    conn.create_scalar_function(
        "REGEXP", // 函数名
        2,        // 参数个数：pattern, text
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let pattern = ctx.get::<String>(0)?;
            let text = ctx.get::<String>(1)?;
            let re = Regex::new(&pattern)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            Ok(re.is_match(&text))
        },
    )?;
    Ok(())
}
#[allow(unused)]
fn register_len(conn: &Connection) -> Result<()> {
    conn.create_scalar_function(
        "LEN",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let s = ctx.get::<String>(0)?;
            Ok(s.len() as i64)
        },
    )?;
    Ok(())
}
#[allow(unused)]
fn test() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    register_regexp(&conn)?;

    // 测试
    let result: bool = conn.query_row("SELECT REGEXP('^[A-Z]', 'Hello')", [], |row| row.get(0))?;
    println!("'Hello' 是否以大写字母开头？ {}", result); // true
    Ok(())
}

//自定义 VFS（虚拟文件系统）
// VFS它负责所有与底层存储的交互：打开文件、读取数据、写入数据、管理锁等。通过自定义 VFS，你可以让 SQLite 将数据存储在任何地方：
// 内存、网络、云存储、加密存储，甚至 IndexedDB
// ┌─────────────────────────────────────────────────────────────
// │                     SQLite 核心引擎                         
// │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐         
// │  │  SQL    │ │  B-tree │ │  Pager  │ │  其他   │         
// │  │  解析器 │ │  引擎   │ │  模块   │ │  模块   │         
// │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘         
// │       └───────────┴───────────┴───────────┘               
// │                           │                                
// │                           ▼                                
// │                  ┌───────────────────┐                     
// │                  │    VFS 接口层     │  ← 这里就是 VFS    
// │                  │  (OS Interface)   │                     
// │                  └─────────┬─────────┘                    
// └────────────────────────────┼────────────────────────────────
//                              │
//                              ▼
//                   ┌───────────────────┐
//                   │   操作系统/文件系统 │
//                   └───────────────────┘
//VFS 是 SQLite 与底层存储之间的唯一接口。它让 SQLite 能够在 Windows、Linux、macOS 等不同操作系统上运行，而无需修改核心代码。
//为什么需要自定义 VFS？
// 场景	                            说明
// 内存数据库（跨连接共享）	          SQLite 内置的 :memory: 是连接私有的。自定义 VFS 可以实现多连接共享的内存数据库
// 加密存储	                        在 VFS 层实现透明加密/解密，数据落盘即加密
// 云存储集成	                    让 SQLite 直接读写 S3、MinIO 等对象存储
// 压缩存储	                        在 VFS 层实现透明压缩，减少存储空间
// 只读归档	                        从只读数据源（如 ZIP 文件）中查询数据
// 浏览器环境（WASM）	            在浏览器中用 IndexedDB 作为持久化存储
// 日志/审计	                    记录所有 I/O 操作，用于调试或审计
//二、Rust 生态中的 VFS 支持
//rusqlite 目前没有官方内置的自定义 VFS API。但社区提供了多个 crate 来填补这个空白。譬如sqlite-vfs、rsqlite-vfs等
//以sqlite-vfs作为讲解： 一个完整的 VFS 需要实现两个核心 trait：
// pub trait Vfs: Sync {
//     type File: File;
//     fn open(&self, path: &str, flags: OpenFlags) -> Result<Self::File>;
//     fn delete(&self, path: &str) -> Result<()>;
//     fn access(&self, path: &str, flags: AccessFlags) -> Result<bool>;
//     fn full_pathname(&self, path: &str) -> Result<String>;
// }
// pub trait File: Send + Sync {
//     fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize>;
//     fn write(&self, offset: u64, buf: &[u8]) -> Result<usize>;
//     fn sync(&self) -> Result<()>;
//     fn file_size(&self) -> Result<u64>;
// }
//SQLite 允许多个 VFS 同时存在，每个连接可以选择使用哪个 VFS。
// // 注册自定义 VFS（伪代码）
// let my_vfs = MyCustomVfs::new();
// register_vfs("myvfs", my_vfs);

// // 使用自定义 VFS 打开数据库
// let conn = Connection::open_with_vfs("my_db", "myvfs")?;

//加载外部扩展库
//SQLite 支持运行时动态加载预编译的共享库扩展。这些扩展可以是：
// 新的 SQL 函数（如正则表达式、UUID 生成）
// 虚拟表（如 generate_series、JSON 表）
// 虚拟文件系统
// 自定义分词器

//rusqlite 提供了 LoadExtensionGuard，这是一个 RAII 守卫，用于临时启用扩展加载功能
// fn load_my_extension(conn: &Connection) -> Result<()> {
//     // 创建守卫，临时启用扩展加载
//     // 守卫离开作用域时，扩展加载功能会被自动禁用[reference:6]
//     let _guard = LoadExtensionGuard::new(conn)?;
       
//     // 加载扩展库
//     conn.load_extension(Path::new("my_sqlite_extension"), None)?;

        // LoadExtensionGuard::new 是 unsafe 函数，因为加载扩展期间执行不受信任的 SQL 查询可能存在安全风险
        // unsafe {
        //     let _guard = LoadExtensionGuard::new(conn)?;
        //     conn.load_extension("trusted/sqlite/extension", None)?;
        // }
//     Ok(())
// }

//加密扩展 - 数据库加密是保护敏感数据的最后一道防线
//SQLCipher依赖库 - 开源，最成熟，社区活跃
//SEE - 闭源，SQLite 官方方案