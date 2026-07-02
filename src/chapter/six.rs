//事务的ACID与回滚日志
//原子性 (Atomicity)：SQLite 通过回滚日志（Rollback Journal）实现。在修改数据页之前，先将原始页拷贝到 *-journal 临时文件中。
//  如果事务ROLLBACK或崩溃，SQLite用日志文件覆盖原数据库，恢复到事务开始前的状态。
//一致性（Consistency）的定义：事务必须将数据库从一个正确的状态（符合所有约束、触发器、业务规则）转变到另一个正确的状态。
//  如果事务执行过程中违反了任何规则，整个事务必须被回滚。
//持久性 (Durability)：事务 COMMIT 时，SQLite 会执行 fsync() 强制将日志和数据刷入磁盘。只有确认刷盘成功，才返回“提交成功”。
//  这就是为什么自动提交（每条 SQL 都刷盘） 那么慢的根本原因。
//隔离性(Isolation)：默认SERIALIZABLE。在事务提交前，其他连接的查询不会看到未提交的更改（除非开启Read Uncommitted，但那是危险的）。
//AID是数据库系统自身的责任（日志、锁、刷盘）。
//C是数据库系统与应用共同的责任。数据库负责维护结构一致性（如外键、唯一约束、类型检查），而业务逻辑一致性（如“总账借贷平衡”）则依赖应用层代码。

//如何保证一致性？
//1.约束（Constraints）：NOT NULL, UNIQUE, CHECK, FOREIGN KEY 在事务提交时被检查，违反则事务失败。
//2.触发器（Triggers）：可自定义业务规则，如“订单总额不能为负”。
//3.原子性联动：如果任何一条 SQL 失败（如外键冲突），且冲突解决策略设为 ROLLBACK，整个事务回滚，保证半途而废的操作不污染数据库。

//一致性示例：
//CREATE TABLE accounts ( 
//     id INTEGER PRIMARY KEY,
       //如果 Rust 代码尝试将余额更新为负值，SQLite 会直接报错，事务自动回滚（取决于冲突策略）。
//     balance REAL CHECK(balance >= 0)  -- 数据库保证余额不会为负
//);

//什么是回滚日志？
// 文件名称：<数据库文件名>-journal（如 test.db-journal）。
// 存在时机：事务开始后（BEGIN）创建，提交（COMMIT）或回滚（ROLLBACK）时删除。
// 内容：存储了被修改的数据页的原始副本。修改数据前，SQLite 先将数据页的原始内容拷贝到日志文件。
//Rollback Journal：修改前先备份旧数据。回滚时用备份覆盖原数据。
//Rollback Journal的三种模式：DELETE（默认）、TRUNCATE、PERSIST。通过 PRAGMA journal_mode 控制
//DELETE（默认) -  事务提交后删除 journal 文件，性能一般，安全性高（崩溃后可恢复）
//TRUNCATE - 提交后将 journal 文件大小截断为 0，而非删除，性能略快，安全性高
//PERSIST - 提交后保留 journal 文件，但将头部标记为“已提交”，下次复用，性能最快，安全高(启动时若 journal 头部未标记，则恢复)

//操作流程：
//1.事务开始：创建一个空的 journal 文件。
//2.修改前：将要修改的数据页从磁盘读入内存缓存，同时拷贝一份原始页到 journal 文件中。
//3.修改数据：在内存中修改数据页。
//4.提交时：
//  4.1将 journal 文件强制刷盘（fsync）——确保原始数据已安全备份。
//  4.2将脏数据页写入数据库文件（同样 fsync）。
//  4.3最后删除 journal 文件（或置空）。
//5.崩溃恢复：如果第 4 步中途崩溃，下次启动时 SQLite 检查 journal 文件是否存在。若存在，则用 journal 中的原始数据覆盖数据库文件，恢复到事务开始前（原子性）。

//只有在 journal 文件被成功删除（或被标记为完成）后，COMMIT 才返回成功。这一过程涉及到至少两次 fsync：
// fsync(journal)：确保备份已落盘。
// fsync(database)：确保新数据已落盘。
//自动提交比批量事务慢几十倍：每行都要执行两次 fsync。

//Rust事务管理 - 闭包自动管理
// conn.transaction(|tx| {
//     tx.execute("UPDATE users SET age = ? WHERE id = ?", params![30, 1])?;
//     tx.execute("DELETE FROM logs WHERE user_id = ?", params![1])?;
//     Ok(()) // 一切正常，自动提交
// })?;
// // 如果闭包中返回 Err，比如：
// conn.transaction(|tx| {
//     tx.execute("INSERT INTO users (name) VALUES (?1)", params!["Alice"])?;
//     // 假设这里触发了一个业务错误
//     if 某种条件 {
//         return Err(rusqlite::Error::SqliteFailure(...));
//     }
//     Ok(())
// })?; // 此时 Err 会传到外层，且闭包内的所有操作自动回滚。