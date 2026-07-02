//1. 三种连接类型
// INNER JOIN：只返回两个表中匹配的行（交集）。这是最常用的。
// LEFT JOIN：返回左表全部行，右表无匹配则补 NULL。
// CROSS JOIN：笛卡尔积。基本不用，除非你确实需要所有组合（极少数情况）。
//2. SQLite 优化器的“自动重排序”（重点）
// 对于 INNER JOIN，SQLite 会自动重排表的连接顺序，以找到最优执行计划。它基于表的大小和索引情况来决定先访问哪张表。
//  写 SQL 时，表在 FROM 中的书写顺序对 INNER JOIN 基本无效，优化器会重新排序。 但对于 LEFT JOIN，顺序是固定的
//    （左表必须首先访问），优化器不会重排左连接。
//3. 连接与索引的强关联
// 如果连接条件 ON t1.id = t2.user_id 中的列没有索引，SQLite 将被迫对右表进行全表扫描（Full Table Scan），性能极差。
// 最佳实践：总是为外键列（如 user_id）建立索引。

//子查询（Subquery）
//子查询是嵌套在其他 SQL 语句中的 SELECT 查询，可以出现在：
// WHERE 子句：用于条件过滤（如 WHERE id IN (SELECT ...)）。
// FROM 子句：将子查询结果作为“派生表”（Derived Table），需要起别名。
// SELECT 子句：作为标量（单值）结果返回。
//示例：
//-- 在 WHERE 中：查找订单总额高于平均值的用户
// SELECT * FROM users 
// WHERE id IN (
//     SELECT user_id FROM orders 
//     GROUP BY user_id 
//     HAVING SUM(amount) > (SELECT AVG(amount) FROM orders)
// );
// -- 在 FROM 中：计算每个用户的总订单额，再与用户表连接
// SELECT u.name, t.total 
// FROM users u
// INNER JOIN (
//     SELECT user_id, SUM(amount) AS total 
//     FROM orders 
//     GROUP BY user_id
// ) t ON u.id = t.user_id;

//CTE（公用表表达式，WITH 子句）
//CTE它将复杂的子查询提取到查询顶部并命名，大幅提升可读性。CTE 的特点是：
// 可以多次引用同一个 CTE（避免重复写子查询）。
// 支持递归（Recursive CTE），这是 SQLite 的杀手级功能。
//示例：
// WITH user_totals AS (
//     SELECT user_id, SUM(amount) AS total
//     FROM orders
//     GROUP BY user_id
// )
// SELECT u.name, ut.total
// FROM users u
// INNER JOIN user_totals ut ON u.id = ut.user_id
// WHERE ut.total > 100;

//递归 CTE 由两部分组成：
// 初始查询（Base Case）：产生第一行数据。
// 递归查询（Recursive Case）：引用 CTE 自身，不断迭代直到返回空集。
// 经典应用：组织架构树、目录层级、图路径搜索。
//语法结构：
// WITH RECURSIVE cte_name(列1, 列2, ...) AS (
//     -- 初始查询
//     SELECT ... FROM ... WHERE 初始条件
//     UNION ALL
//     -- 递归查询
//     SELECT ... FROM cte_name JOIN ... ON 递归条件
// )
// SELECT * FROM cte_name;
//示例：
// WITH RECURSIVE seq(n) AS (
//     SELECT 1          -- 初始：n = 1
//     UNION ALL
//     SELECT n + 1      -- 递归：n + 1
//     FROM seq
//     WHERE n < 10      -- 终止条件
// )
// SELECT * FROM seq;
// SQLite 限制：默认递归深度上限为 1000，可通过 PRAGMA recursive_triggers 调整。