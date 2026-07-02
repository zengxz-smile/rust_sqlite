//冲突解决策略
//冲突发生场景
// 唯一约束冲突：插入或更新的值在唯一列上已经存在。
// NOT NULL 冲突：插入时显式为 NULL，但列定义为 NOT NULL。
// CHECK 约束冲突：插入或更新的值不满足 CHECK 条件。

//五种冲突解决策略（按严厉程度排序）
//策略 - 行为；事务影响；适用场景
//ROLLBACK - 整个事务回滚（若在事务内）；最严厉，终止当前事务；要求完全原子性，任何冲突都要全部撤销
//ABORT（默认）- 中止当前语句，但事务内之前的操作保留；语句级回滚，事务可继续；希望跳过冲突语句，但保留同事务其他操作
//FAIL- 中止当前语句，并返回错误，但已修改的行不会撤销（除非显式回滚）；部分修改可能残留；不推荐，容易造成数据不一致
//IGNORE - 跳过冲突行，继续执行后续操作；不影响事务；批量插入时忽略重复行（幂等）
//REPLACE - 先删除导致冲突的旧行，再插入新行（实际上是 DELETE + INSERT）；原子操作；更新或插入（Upsert）场景
//关键区别：
// ROLLBACK 和 ABORT 的区别：ROLLBACK 回滚整个事务；ABORT 只回滚当前语句。
// IGNORE 和 REPLACE：IGNORE 跳过新行，REPLACE 删除旧行并插入新行（注意：REPLACE 会触发外键级联删除，且会改变 ROWID）。

//Rust 实践（四种常用写法）
// 1. 使用 INSERT OR IGNORE（幂等插入）
// let rows_affected = conn.execute(
//     "INSERT OR IGNORE INTO users (name, age) VALUES (?1, ?2)",
//     params!["Alice", 30],
// )?;
// if rows_affected == 1 {
//     println!("新行插入成功");
// } else {
//     println!("已存在，跳过");
// }
// 2. 使用 INSERT OR REPLACE（更新或插入）
// let rows_affected = conn.execute(
//     "INSERT OR REPLACE INTO users (id, name, age) VALUES (?1, ?2, ?3)",
//     params![1, "Alice", 31],
// )?;
// 注意：替换时 rows_affected 可能为 2（删除旧行 + 插入新行）
// 3. 使用 INSERT ... ON CONFLICT DO UPDATE（标准 Upsert）
// conn.execute(
//     "INSERT INTO users (id, name, age) VALUES (?1, ?2, ?3)
        // excluded 是特殊表，代表尝试插入的新值。
//      ON CONFLICT(id) DO UPDATE SET name = excluded.name, age = excluded.age",
//     params![1, "Bob", 25],
// )?;
// 4. 使用 INSERT OR ROLLBACK
// conn.transaction(|tx| {
//     tx.execute("INSERT OR ROLLBACK INTO users (name) VALUES (?1)", params!["Alice"])?;
//     // 如果这里发生冲突，整个事务回滚，所有之前的插入也撤销
//     tx.execute("INSERT OR ROLLBACK INTO users (name) VALUES (?1)", params!["Alice"])?;
//     Ok(())
// })?;

//SQLite锁：
// 锁级别	                        含义	                            谁可以持有
// UNLOCKED	                未锁定，未读取或写入	                      初始状态
// SHARED	                    可读，不可写	                    多个连接可同时持有
// RESERVED	                   准备写，但仍可读	                 仅一个连接可持有，表示即将写入
// PENDING	            等待其他读者释放，准备升级为排它锁	     仅一个连接可持有，阻止新 SHARED 锁
// EXCLUSIVE	         排它锁，可读写，其他连接不可访问	            仅一个连接可持有

//SQLite 的默认隔离级别是 SERIALIZABLE（可串行化），它通过锁机制确保并发事务之间互不干扰。
//默认 SERIALIZABLE 行为：
// 读操作（SELECT）需要获取 SHARED 锁，多个读者可并发。
// 写操作（INSERT/UPDATE/DELETE）需要 RESERVED → PENDING → EXCLUSIVE 逐步升级。在升级过程中，如果有其他连接持有 SHARED 锁，
//    写操作会等待（SQLITE_BUSY）。
// 读写互斥：读不阻塞读，但写会阻塞所有读写（包括读）。

//SQLite 也提供了一种更宽松的隔离级别 READ UNCOMMITTED，允许“脏读”。
//当启用 PRAGMA read_uncommitted = true 时，SQLite 允许连接在不获取 SHARED 锁的情况下读取数据页。
//这意味着它可以看到其他连接未提交的更改（即脏读）。
//启用条件：
// 必须开启 共享缓存模式（Shared Cache），否则即使设置 PRAGMA 也不生效。
// 对于内存数据库，可以使用 Connection::open("file::memory:?cache=shared") 来共享缓存。
// 对于磁盘数据库，必须在所有连接打开时指定 ?cache=shared URI 参数。
//优点：
// 读操作不等待写操作，也不阻塞写操作，提高并发吞吐量。
// 适合只读报表查询，对数据新鲜度要求不高，且能容忍临时不一致。
//缺点：
// 可能读到未提交的数据，这些数据后续可能被回滚，导致查询结果不可靠。
// 违背 ACID 的隔离性，可能导致业务逻辑错误。