//基础查询语法
// SELECT [DISTINCT] 列1, 列2, ...
// FROM 表
// WHERE 条件
// ORDER BY 列 [ASC|DESC]
// LIMIT 数量 OFFSET 偏移;

// DISTINCT：去重。
// WHERE：支持 =, <>, >, <, LIKE, IN, BETWEEN, IS NULL 等。
// ORDER BY：默认升序，可多列排序。
// LIMIT：限制返回行数，OFFSET 用于分页（但大数据量时 OFFSET 性能差，后续阶段会讲游标分页）。

//聚合函数
// COUNT(*), COUNT(列)：计数。
// SUM(列), AVG(列), MAX(列), MIN(列)。
// 常与 GROUP BY 配合使用。

//连接（JOIN）- 支持 INNER JOIN, LEFT JOIN, CROSS JOIN。
//示例：
// SELECT users.name, orders.amount
// FROM users
// INNER JOIN orders ON users.id = orders.user_id;

//子查询与 CTE（公共表表达式）
// 子查询可放在 SELECT、FROM、WHERE 中。
// CTE 用 WITH 开头，提高可读性。

//索引与查询性能
// 索引 是 B-Tree 结构，能加速 WHERE、JOIN、ORDER BY。
// 创建索引：CREATE INDEX idx_name ON table (column);
// 组合索引：CREATE INDEX idx_name ON table (col1, col2);，顺序很重要。
// 查询优化器会自动决定是否使用索引，但索引会拖慢 INSERT/UPDATE/DELETE，需权衡。
