//SQLite 是动态类型数据库，可存储任何类型的数据。但它有类型亲和性，影响比较和排序规则。
//常用亲和性：
// INTEGER → 整数
// TEXT → 文本
// REAL → 浮点数
// NUMERIC → 宽泛数字
// BLOB → 二进制
//索引和查询优化器依赖声明类型，所以尽量正确声明。

//ROWID 与 INTEGER PRIMARY KEY
// 每张表都有隐式 ROWID（64位整数，自增）。
// 若定义 INTEGER PRIMARY KEY，该列成为 ROWID 别名，性能最优。
// 插入时若不指定该列，系统自动分配唯一值（比 AUTO_INCREMENT 高效）。
// last_insert_rowid() 返回刚插入行的 ROWID。

//INSERT 语法（重点）
// -- 指定列插入（推荐）
// INSERT INTO table (col1, col2) VALUES (val1, val2);
// -- 插入多行，SQLite 3.7.11+版本才支持
// INSERT INTO table (col1, col2) VALUES (v1a, v2a), (v1b, v2b)...;
// -- 使用 DEFAULT 关键字
// INSERT INTO table (col1, created) VALUES (?, DEFAULT);
// -- 冲突处理：INSERT OR IGNORE / REPLACE / ROLLBACK / ABORT / FAIL
// INSERT OR IGNORE INTO table ...   -- 唯一冲突则忽略此行
// INSERT OR REPLACE INTO table ...  -- 冲突则删除旧行，插入新行

//创建表示例：
// CREATE TABLE users (
//     id INTEGER PRIMARY KEY,
//     name TEXT NOT NULL UNIQUE,
//     age INTEGER CHECK(age >= 0),
//     created_at TEXT DEFAULT (datetime('now'))
// );
//约束：NOT NULL, UNIQUE, CHECK, FOREIGN KEY。
//默认值：DEFAULT 可以跟常量或表达式，如 CURRENT_TIMESTAMP。