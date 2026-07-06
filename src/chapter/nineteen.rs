//连接池与并发策略 
//第一部分：SQLite 并发模型
//一、SQLite 锁机制回顾
//SQLite 使用基于页面的锁来管理并发访问。锁的升级路径是单向的：从低到高逐步获取。
// 锁升级路径（Rollback Journal 模式）：
// ┌──────────────────────────────────────────────────────────────────
// │                                                                  
// │  UNLOCKED ──→ SHARED ──→ RESERVED ──→ PENDING ──→ EXCLUSIVE   
// │      ↑           ↑            ↑            ↑            ↑      
// │      │           │            │            │            │      
// │   初始状态    读操作      准备写      等待读者      写操作      
// │                                                                  
// └──────────────────────────────────────────────────────────────────
// 锁级别	        含义	            持有条件	                        并发能力
// UNLOCKED	       未锁定	            初始状态	                        无限制
// SHARED	        读锁	          SELECT 操作	                  多个连接可同时持有
// RESERVED	       准备写	    UPDATE/INSERT/DELETE 开始	    只能一个连接持有，但允许新的 SHARED 锁
// PENDING	       等待写	        准备升级为 EXCLUSIVE	    阻止新的 SHARED 锁，等待现有 SHARED 锁释放
// EXCLUSIVE	    写锁	           写入数据页	                  独占，阻止所有其他操作
//一个写事务的完整锁生命周期
// 时间线：
// ┌─────────────────────────────────────────────────────────────────────————
// │  UNLOCKED ──→ SHARED ──→ RESERVED ──→ PENDING ──→ EXCLUSIVE ──→ UNLOCKED 
// │      ↑           ↑            ↑            ↑            ↑            ↑    
// │      │           │            │            │            │            │    
// │  连接打开   执行 BEGIN  执行 UPDATE    尝试提交      执行 fsync     提交完成   
// │             或第一个读    获得写锁     等待读者         获得写锁      释放锁     
// │                                        释放锁                         
// └─────────────────────────────────────────────────────────────────────————
// 当前锁 \ 请求锁	    SHARED	    RESERVED	    PENDING	        EXCLUSIVE
// SHARED	            允许	     允许	          阻塞	            阻塞
// RESERVED	            允许	    已持有	          阻塞              阻塞
// PENDING	            阻塞	     阻塞	          阻塞	            阻塞
// EXCLUSIVE	        阻塞	     阻塞	          阻塞	            阻塞
//关键洞察：多个读者可以并发，但一旦有一个写者，所有新读者都会被阻塞。
//二、WAL 模式下的并发革命
// Rollback Journal 模式：
// ┌──────────┐     ┌──────────┐     ┌──────────┐
// │  读者 A  │────→│  SHARED  │     │          │
// ├──────────┤     ├──────────┤     │          │
// │  读者 B  │────→│  SHARED  │     │  写者等待 │
// ├──────────┤     ├──────────┤     │          │
// │  写者 C  │────→│ EXCLUSIVE│────→│ 读者释放  │
// └──────────┘     └──────────┘     └──────────┘
// WAL 模式：
// ┌──────────┐     ┌──────────┐
// │  读者 A  │────→│  读取 DB │
// ├──────────┤     ├──────────┤
// │  读者 B  │────→│  读取 DB │  ← 不阻塞
// ├──────────┤     ├──────────┤
// │  写者 C  │────→│  写 WAL  │  ← 不阻塞读者
// └──────────┘     └──────────┘
//WAL 模式下的锁变化：
// 读操作不需要获取 SHARED 锁（直接读取主数据库 + WAL 文件）
// 写操作不需要等待 SHARED 锁释放（直接追加到 WAL 文件）
// 读写真正并发：读者看到的是 WAL 开始前的快照

//第二部分：连接池
//为什么需要连接池？
// 单次请求耗时对比：
// ┌─────────────────────────────────────────────────────────────────
// │  无连接池：创建连接(10ms) + 查询(1ms) + 销毁连接(2ms) = 13ms 
// │  有连接池：从池中获取(0.1ms) + 查询(1ms) = 1.1ms            
// │  性能提升：约 12 倍                                           
// └─────────────────────────────────────────────────────────────────
//采用连接池的技术选用 r2d2 + 自定义 / sqlx
//r2d2 - 通用连接池框架
//自定义的适配器
//协作流程：
// 应用代码
//    │
//    ▼
// r2d2::Pool (通用池)
//    │
//    │ 通过 ManageConnection trait 调用
//    ▼
//  自定义 (适配器)
//    │
//    │ 创建/验证/销毁
//    ▼
// rusqlite::Connection (实际连接)

//r2d2的Pool<T, M> 是连接池的核心类型，泛型参数：
// T：连接类型（即 rusqlite::Connection）
// M：管理器类型（即 SqliteConnectionManager）
//连接的获取与归还
// 获取：pool.get() 返回 PooledConnection<T>，它是一个智能指针，内部持有连接。
// 归还：当 PooledConnection 离开作用域（Drop），连接自动归还到池中。
//SqliteConnectionManager 实现了 ManageConnection trait，提供三个核心方法：
// connect：调用 rusqlite::Connection::open，并执行 PRAGMA journal_mode=WAL 等初始化逻辑。
// is_valid：执行 SELECT 1，若失败则认为连接无效并丢弃。
// has_broken：判断连接是否损坏
//r2d2::Pool 通过 r2d2::Config 结构体提供丰富的配置项：
// -- Rust伪代码
// let config = Config {
//     max_size: 20,
//     min_idle: Some(3),
//     connection_timeout: Duration::from_secs(5),
//     idle_timeout: Some(Duration::from_secs(600)),
//     max_lifetime: Some(Duration::from_secs(1800)),
//     ..Default::default()
// };
// let pool = Pool::builder()
//     .config(config)
//     .build(manager)?;
// 参数	                    含义	                默认值	                    推荐值
// max_size	           池中最大连接数	              10	               CPU 核心数 × 2 ~ 20
// min_idle	        保持的最小空闲连接数	           0	               2~5（避免冷启动延迟）
// connection_timeout	获取连接的最大等待时间	      30s	               5~10s（避免无限等待）
// idle_timeout	    空闲连接超时（超时后关闭）	      None	                10~30min（释放资源）
// max_lifetime	  连接最大生命周期（强制回收）	      None	               30min~1h（避免累积状态）
//结合 SQLite 特性选择参数
//SQLite 是单写者，过大的连接池对写入无帮助，反而消耗内存和 FD。推荐：
// 读多写少：max_size = CPU核数 × 2 + 1
// 写多读少：max_size = CPU核数 + 1
//启用 WAL 后再使用连接池
// let manager = SqliteConnectionManager::file("app.db")
//     .with_init(|conn| {
//         conn.pragma_update(None, "journal_mode", "WAL")?;
//         conn.pragma_update(None, "synchronous", "NORMAL")?;
//         Ok(())
//     });
//连接池的监控:
// let state = pool.state();
// println!("连接总数: {}", state.connections);
// println!("空闲连接: {}", state.idle_connections);
// println!("使用中: {}", state.connections - state.idle_connections);

//存储优化与维护 - 为什么需要存储优化？
//前几期视频都在解决一个核心问题：如何让 SQLite 跑得更快。而存储优化解决的是另一个维度的问题：如何让 SQLite 跑得更稳、更久。
//长期使用会遇到下面问题：
// 删除大量数据后，数据库文件大小不会自动缩小
// 频繁的 INSERT/UPDATE/DELETE 会产生碎片，降低查询性能
// 长期运行的数据库可能积累大量空闲空间
// 数据库损坏的风险随着时间推移而增加
//这些问题不会立即导致系统崩溃，但会像“慢性病”一样侵蚀数据库的健康。存储优化的目标就是预防和治疗这些慢性病。
// 第一部分：VACUUM 原理与用法
//SQLite 在删除数据或删除表时，并不会立即释放磁盘空间。它只是将对应的页面标记为“空闲”，并加入到空闲列表（freelist）中。
//这些页面会被后续的插入操作复用，但数据库文件大小不会自动缩小。
// 删除数据后的文件状态：
// ┌─────────────────────────────────────────────────────────────────
// │  原始数据库文件（100 MB）                                      
// │  ┌──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐   
// │  │ 页1  │ 页2  │ 页3  │ 页4  │ 页5  │ 页6  │ 页7  │ 页8  │   
// │  │ 数据 │ 数据 │ 数据 │ 数据 │ 数据 │ 数据 │ 数据 │ 数据 │   
// │  └──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘   
// └─────────────────────────────────────────────────────────────────
// 删除页3、页5、页7 后：
// ┌─────────────────────────────────────────────────────────────────
// │  数据库文件（100 MB，但只有 50 MB 有效数据）                    
// │  ┌──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐   
// │  │ 页1  │ 页2  │空闲  │ 页4  │空闲  │ 页6  │空闲  │ 页8  │   
// │  │ 数据 │ 数据 │      │ 数据 │      │ 数据 │      │ 数据 │   
// │  └──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘   
// │                   空闲列表：页3 → 页5 → 页7                     
// └─────────────────────────────────────────────────────────────────

//VACUUM 的作用
// 创建一个新的空数据库文件
// 从旧数据库中读取所有有效数据，写入新文件
// 重建索引
// 将新文件替换旧文件
//结果：数据库文件缩小到实际数据大小，碎片被消除
// VACUUM 后：
// ┌─────────────────────────────────────────────────────────────────
// │  新的数据库文件（50 MB）                                       
// │  ┌──────┬──────┬──────┬──────┬──────┐                        
// │  │ 页1  │ 页2  │ 页4  │ 页6  │ 页8  │                        
// │  │ 数据 │ 数据 │ 数据 │ 数据 │ 数据 │                        
// │  └──────┴──────┴──────┴──────┴──────┘                        
// │  空闲列表：空                                                  
// └─────────────────────────────────────────────────────────────────
//VACUUM 的执行方式 - 手动
//Rust 中手动执行 VACUUM
// fn vacuum(conn: &Connection) -> Result<()> {
//     conn.execute_batch("VACUUM;")?; - 执行完整 VACUUM（会锁表）
//     conn.execute_batch(&format!("VACUUM {};", table_name))?; - 针对特定表执行 VACUUM
//     Ok(())
// }
//查看 VACUUM 效果
// fn get_db_stats(conn: &Connection) -> Result<()> {
//     let page_count: u32 = conn.pragma_query_value(None, "page_count", |row| row.get(0))?;
//     let freelist_count: u32 = conn.pragma_query_value(None, "freelist_count", |row| row.get(0))?;
//     let page_size: u32 = conn.pragma_query_value(None, "page_size", |row| row.get(0))?;
    
//     let db_size = (page_count * page_size) as f64 / (1024.0 * 1024.0);
//     let freelist_pages = freelist_count;
//     let freelist_mb = (freelist_count * page_size) as f64 / (1024.0 * 1024.0);
    
//     println!("📊 数据库统计:");
//     println!("  - 页面数: {}", page_count);
//     println!("  - 空闲页面: {} ({:.2} MB)", freelist_pages, freelist_mb);
//     println!("  - 数据库大小: {:.2} MB", db_size);
//     println!("  - 碎片率: {:.1}%", (freelist_pages as f64 / page_count as f64) * 100.0);
//     Ok(())
// }
//VACUUM 的代价
// 代价	                            说明
// 锁表	            VACUUM 执行期间数据库被锁定，所有读写操作被阻塞
// 磁盘空间	        需要额外的磁盘空间（至少等于当前数据库大小）
// 时间	            大数据库可能需要数分钟甚至数小时
// WAL 文件	        VACUUM 会触发 checkpoint，WAL 文件会被清空

//VACUUM 的执行方式 - 自动
// 模式	            值	                行为	                    优缺点
// NONE         	0	            不自动回收空间	              默认，空间不回收
// FULL         	1	            每次事务提交时自动回收	       频繁操作，性能下降
// INCREMENTAL	    2	            增量回收，需要手动触发	        灵活，但需要额外维护

//配置 AUTO_VACUUM
// 启用 FULL 模式
// fn enable_auto_vacuum(conn: &Connection) -> Result<()> {
//     // 必须在创建表之前设置
//     conn.pragma_update(None, "auto_vacuum", 1)?;
//     Ok(())
// }
// 启用 INCREMENTAL 模式
// fn enable_incremental_vacuum(conn: &Connection) -> Result<()> {
//     conn.pragma_update(None, "auto_vacuum", 2)?;
//     Ok(())
// }

// INCREMENTAL VACUUM 的增量回收
// 增量回收指定数量的页面
// fn incremental_vacuum(conn: &Connection, pages: u32) -> Result<()> {
//     conn.pragma_update(None, "incremental_vacuum", pages)?;
//     Ok(())
// }
// 回收所有空闲页面
// fn vacuum_all_incremental(conn: &Connection) -> Result<()> {
//     // 先获取空闲页面数
//     let freelist: u32 = conn.pragma_query_value(None, "freelist_count", |row| row.get(0))?;
//     if freelist > 0 {
//         conn.pragma_update(None, "incremental_vacuum", freelist)?;
//         println!("已回收 {} 个空闲页面", freelist);
//     }
//     Ok(())
// }

// 对比维度	    手动 VACUUM	            AUTO_VACUUM (FULL)	            AUTO_VACUUM (INCREMENTAL)
// 自动化	        需手动执行	                自动执行	                        需配合代码触发
// 运行时性能影响	 集中爆发	                持续轻微影响	                         可控
// 空间回收效果	 完全回收	                 完全回收	                           逐渐回收
// 锁表时间	       长	                每次事务后短暂锁定	                         可控
// 适用场景	     定期维护	                小型数据库	                           生产环境

// 开始监控
//    │
//    ▼
// 碎片率 > 20%？
//    ├── 是 → 检查数据库大小
//    │         ├── > 10GB → 考虑迁移或归档
//    │         ├── 1-10GB → 执行 VACUUM
//    │         └── < 1GB → 执行 VACUUM 或不处理
//    └── 否 → WAL 文件大小 > 64MB？
//              ├── 是 → 执行 checkpoint
//              └── 否 → 一切正常

use rusqlite::{Connection, Result};
use std::time::Instant;

#[allow(unused)]
struct VacuumManager {
    threshold_percent: f64,      // 触发 VACUUM 的碎片率阈值
    min_size_mb: u64,            // 触发 VACUUM 的最小数据库大小（MB）
}

impl Default for VacuumManager {
    fn default() -> Self {
        Self {
            threshold_percent: 20.0,  // 碎片率超过 20% 时触发
            min_size_mb: 10,          // 数据库大于 10MB 才考虑
        }
    }
}

#[allow(unused)]
impl VacuumManager {
    fn should_vacuum(&self, conn: &Connection) -> Result<bool> {
        let page_count: u32 = conn.pragma_query_value(None, "page_count", |row| row.get(0))?;
        let freelist: u32 = conn.pragma_query_value(None, "freelist_count", |row| row.get(0))?;
        let page_size: u32 = conn.pragma_query_value(None, "page_size", |row| row.get(0))?;
        
        let db_size_mb = (page_count * page_size) as f64 / (1024.0 * 1024.0);
        if db_size_mb < self.min_size_mb as f64 {
            return Ok(false); // 数据库太小，不值得 VACUUM
        }
        
        let fragmentation = (freelist as f64 / page_count as f64) * 100.0;
        Ok(fragmentation > self.threshold_percent)
    }
    
    fn vacuum_if_needed(&self, conn: &Connection) -> Result<()> {
        if !self.should_vacuum(conn)? {
            return Ok(());
        }
        
        println!("🔧 开始执行 VACUUM...");
        let start = Instant::now();
        
        // 1. 先执行 CHECKPOINT（确保 WAL 已合并）
        conn.execute_batch("PRAGMA wal_checkpoint(FULL);")?;
        
        // 2. 执行 VACUUM
        conn.execute_batch("VACUUM;")?;
        
        let duration = start.elapsed();
        println!("✅ VACUUM 完成，耗时: {:.2?}", duration);
        Ok(())
    }
}