//更新
// UPDATE table
// SET col1 = value1, col2 = value2, ...
// WHERE condition
// ORDER BY ... LIMIT ...;   -- SQLite 特有：可限制更新的行数
// 多列更新：用逗号分隔。
// 表达式更新：SET age = age + 1。
// 子查询更新：SET col = (SELECT ...)。
// ORDER BY 和 LIMIT：SQLite 允许在 UPDATE 中使用，非常实用（如只更新最老的 10 条记录）。
// RETURNING 子句（SQLite 3.35.0+，rusqlite 支持）：返回被修改的行数据，省去再次查询。

//删除
// DELETE FROM table
// WHERE condition
// ORDER BY ... LIMIT ...;   -- 同样支持 ORDER BY 和 LIMIT
// 不带 WHERE 会删除所有行（但保留表结构）
// 要清空表并重置自增计数器，可以用 DELETE FROM table。DELETE 不带 WHERE 性能不错，因为不会逐行触发触发器
// 同样支持 RETURNING。

//性能注意事项
// 索引：WHERE 条件中的列如果有索引，会加速定位，但索引也会增加更新/删除的开销（因为要同步维护索引）。
// 事务：批量更新/删除时，显式事务比自动提交快得多（与 INSERT 类似）。
// 大量删除后回收空间：VACUUM 可整理数据库文件，但会锁表，在生产环境要谨慎。