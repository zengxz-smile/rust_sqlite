//索引 - 数据量大的表建立索引，提升查询速度
//如果没有索引，SQLite 必须扫描整张表，逐行检查 name 是否匹配。这是 O(N) 操作。
//如果有索引，SQLite 通过 B-tree 快速定位到 "Alice" 的位置，这是 O(log N) 操作。
//B-tree 索引
//B-tree（平衡多路搜索树）是 SQLite 的默认索引结构。
// B-tree 结构示意（简化）：
// ┌──────────────────────────────────────────────────────────
// │                        根节点
// │              [10] [20] [30] [40]
// │              /    |    |    \
// ┌─────────────/─────|────|─────\───────────────────────────
// │  叶子节点1   │  叶子节点2   │  叶子节点3   │  叶子节点4    │
// │  [1][2][3]  │  [11][12][13]│  [21][22][23]│  [31][32][33]
// └─────────────┴─────────────┴─────────────┴───────────────┘
//                          │
//                    数据行指针（ROWID）
//关键特性：
// 有序：所有键值按升序排列
// 平衡：所有叶子节点到根的距离相同
// 多路：每个节点有多个子节点（默认可存储多个键值）
// 指针：叶子节点存储键值 + ROWID（指向数据行的指针）
//B-tree 查找过程：查找 name = "Smile" 的过程：
// 1. 从根节点开始
// 2. 比较 "Smile" 与根节点中的键值
// 3. 根据比较结果，进入对应的子节点
// 4. 重复步骤 2-3，直到到达叶子节点
// 5. 在叶子节点中找到 "Smile" 的条目
// 6. 读取对应的 ROWID
// 7. 使用 ROWID 从表中读取完整数据行
//复杂度：O(log N)，深度通常为 3-5 层（百万级数据）。
//在 SQLite 中，索引是独立的 B-tree，存储在与表数据不同的页中。索引查找需要两次 I/O（索引页 + 数据页），组合索引可避免第二次 I/O。
//索引的代价:
//1.存储空间:
// 表大小：1 GB
// ────────────────────
// 索引 1：100 MB
// 索引 2：80 MB
// 索引 3：120 MB
// ────────────────────
// 总大小：1.3 GB（+30%）
//2.写入性能：每个索引在 INSERT/UPDATE/DELETE 时都需要维护，造成额外的 I/O。
//何时使用索引:
// 查询是否频繁？
//   ├── 是 → 考虑建索引
//   │   ├── 查询选择性 > 10% → 建索引
//   │   ├── 查询选择性 1-10% → 酌情考虑
//   │   └── 查询选择性 < 1% → 不建索引
//   └── 否 → 不建索引
//小表（< 1000 行） - 全表扫描更快；选择性低（如性别） - 无法有效过滤； 写入频繁的表 - 索引维护代价大。这三种不推荐建索引
//Rust创建索引示例：conn.execute("CREATE INDEX idx_data ON t(data)"), []).unwrap();

//索引类型:
// 单列索引	    CREATE INDEX idx ON table(col)	            单列查询	        最简单
// 多列索引	    CREATE INDEX idx ON table(c1, c2)	        多条件查询	        需注意最左前缀
// 唯一索引	    CREATE UNIQUE INDEX idx ON table(col)	    唯一性约束	        兼具约束功能
// 部分索引	    CREATE INDEX idx ON table(col) WHERE cond	只查部分数据	    节省空间
// 表达式索引	CREATE INDEX idx ON table(expr)	            函数查询	        支持函数结果索引

//CREATE INDEX idx_users_name ON users(name); - 最简单的索引：单个列
// SELECT * FROM users WHERE name = 'Smile'; - 会使用单列索引
// SELECT * FROM users WHERE name LIKE 'Smile%'; - 会使用单列索引
// SELECT * FROM users ORDER BY name; - 会使用单列索引
// SELECT * FROM users WHERE LOWER(name) = 'smile'; - 不会使用单列索引，因为函数包裹
// SELECT * FROM users WHERE name LIKE '%Smile'; - 不会使用单列索引，因为通配符在开头

//组合索引：(a, b, c) -  最左前缀原则
//支持的查询：
//   WHERE a = 1                使用索引（第一列）
//   WHERE a = 1 AND b = 2      使用索引（前两列）
//   WHERE a = 1 AND b = 2 AND c = 3     使用索引（全部）
//   WHERE b = 2                不使用索引（没有第一列）
//   WHERE a = 1 AND c = 3      只使用 a，不使用 c（跳过 b）
//   WHERE a > 1 AND b = 2      只使用 a（范围后的列失效）
//关键洞察：
// 索引的顺序决定了哪些查询可以使用它
// 将选择性最高的列放在最前面
// 范围查询（>, <, LIKE）会使后续列失效

//唯一索引兼具两个功能：
// 性能：加速查询
// 约束：保证列值的唯一性
// -- 联合唯一索引：name + age 组合必须唯一
// CREATE UNIQUE INDEX idx_users_name_age_unique ON users(name, age);

//部分索引（条件索引）
// -- 只为活跃用户创建索引
// CREATE INDEX idx_users_active_name ON users(name) WHERE status = 'active';
// SELECT * FROM users WHERE status = 'active' AND name = 'Smile'; - 会使用部分索引（status = 'active'）
// SELECT * FROM users WHERE status = 'inactive' AND name = 'Smile'; - 不会使用部分索引（status 不匹配）
// SELECT * FROM users WHERE name = 'Smile';  -- 不会使用部分索引，未指定 status
//部分索引更小、更快、更高效

//表达式索引
// -- 索引 UPPER(name) 的结果
// CREATE INDEX idx_users_name_upper ON users(UPPER(name));
// SELECT * FROM users WHERE UPPER(name) = 'ALICE'; - 会使用表达式索引
// SELECT * FROM users WHERE UPPER(name) LIKE 'ALI%'; - 会使用表达式索引
// SELECT * FROM users WHERE name = 'Alice';  不会使用表达式索引，表达式不匹配

//索引类型选择决策树
// 查询模式分析 - EXPLAIN QUERY PLAN / EXPLAIN
//     │
//     ├── 单列等值查询 → 单列索引
//     │
//     ├── 多列等值查询 → 组合索引（按选择性排序）
//     │
//     ├── 范围查询 → 组合索引（范围列放最后）
//     │
//     ├── 唯一性要求 → 唯一索引
//     │
//     ├── 查询部分数据 → 部分索引
//     │
//     └── 函数查询 → 表达式索引
//EXPLAIN QUERY PLAN - 高层执行计划，用于日常查询优化
//EXPLAIN - 虚拟字节码，用于深入调试

//查询计划分析
//EXPLAIN QUERY PLAN
// SELECT users.name, orders.amount
// FROM users
// INNER JOIN orders ON users.id = orders.user_id
// WHERE users.name = 'Smile';
//输出结构示例：
// id|parent|detail
// --|------|------
// 0|0|SEARCH users USING INDEX idx_users_name (name=?)
// 1|0|SEARCH orders USING INDEX idx_orders_user_id (user_id=?)
//字段说明：
// id：操作序号（0 是顶层，1+ 是子操作）
// parent：父操作的 id（0 表示顶层）
// detail：具体执行方式
//执行计划关键术语
// SCAN TABLE t	                全表扫描	 差	        没有可用索引或索引无效
// SEARCH t USING INDEX idx	    索引搜索	 好	        索引被正确使用
// SEARCH t USING PRIMARY KEY	主键搜索	 最好	    主键索引使用
//索引相关术语
// USING INDEX idx	            使用普通索引	    好
// USING PRIMARY KEY	        使用主键索引	    最好
// USING COVERING INDEX idx	    使用覆盖索引	    最好（无需回表）
// WITHOUT ROWID	            无 rowid 表	       特殊优化
// WITH INTEGER PRIMARY KEY	    整数主键	       高效（主键即 rowid）
//排序相关术语 - 无索引排序的情况
// USE TEMP B-TREE FOR ORDER BY	    使用临时 B-tree 排序	需要额外排序操作
// USE TEMP B-TREE FOR GROUP BY	    使用临时 B-tree 分组	需要额外分组操作
// USE TEMP B-TREE FOR DISTINCT	    使用临时 B-tree 去重	需要额外去重操作
//连接（JOIN）分析
//SQLite 对 INNER JOIN 会自动重排连接顺序，以找到最优执行计划。
//SQLite 只支持嵌套循环连接（Nested Loop Join）
// 对于每个来自左表的行：
//     在右表中查找匹配的行（使用索引）
//     如果匹配，输出结果
//关键要求：右表必须有索引，否则会退化为全表扫描。
//Rust伪代码示例：
// fn explain_query(conn: &Connection, sql: &str) -> Result<()> {
//     let explain_sql = format!("EXPLAIN QUERY PLAN {}", sql);
//     let mut stmt = conn.prepare(&explain_sql)?;
//     let rows = stmt.query_map([], |row| {
//         Ok((
//             row.get::<_, i32>(0)?,
//             row.get::<_, i32>(1)?,
//             row.get::<_, String>(2)?,
//         ))
//     })?;
//     println!("id|parent|detail");
//     println!("--|------|------");
//     for row in rows {
//         let (id, parent, detail) = row?;
//         let indent = if parent == 0 { "" } else { "  " };
//         println!("{}|{}|{}{}", id, parent, indent, detail);
//     }
//     Ok(())
// }

//索引设计策略 - 核心原则
// 索引设计五步法：
// 1. 分析查询模式 → 2. 评估选择性 → 3. 确定列顺序 → 4. 选择索引类型 → 5. 验证与监控
//原则一：为查询建索引，不是为表建索引
//原则二：选择性决定索引价值，选择性 = 不同值的数量 / 总行数
// 选择性	        索引价值	        示例列
// > 10%	        高价值	        身份证号、邮箱
// 1-10%	        中等价值	    年龄、城市
// < 1%	            低价值	        性别、是否删除
// = 100%	        最高价值	    主键、唯一标识
//原则三：权衡写入性能
// 每个索引都会影响写入性能：
// - INSERT：每个索引需要插入一条记录
// - UPDATE：索引列更新时需要维护索引
// - DELETE：每个索引需要删除一条记录

//计算选择性:
// -- 计算表的总行数
// SELECT COUNT(*) FROM users;  -- 假设 1,000,000

// -- 计算各列的选择性
// SELECT
//     COUNT(DISTINCT id) * 1.0 / 1000000 AS id_selectivity,        -- 1.0
//     COUNT(DISTINCT name) * 1.0 / 1000000 AS name_selectivity,    -- 0.85
//     COUNT(DISTINCT email) * 1.0 / 1000000 AS email_selectivity,  -- 0.99
//     COUNT(DISTINCT age) * 1.0 / 1000000 AS age_selectivity,      -- 0.05
//     COUNT(DISTINCT gender) * 1.0 / 1000000 AS gender_selectivity -- 0.000001
// FROM users;

use rusqlite::{Connection, Result};

#[allow(unused)]
#[derive(Debug)]
struct QueryPattern {
    sql: String,
    frequency: i32,              // 每秒查询数
    columns: Vec<String>,        // WHERE 中使用的列
    select_columns: Vec<String>, // SELECT 的列
}

#[allow(unused)]
struct IndexSuggestion {
    table: String,
    columns: Vec<String>,
    index_type: IndexType,
    predicted_benefit: f64,
    predicted_cost: f64,
}

#[allow(unused)]
enum IndexType {
    Single,
    Composite,
    Unique,
    Partial,
    Expression,
}

struct IndexDesigner {
    conn: Connection,
    table: String,
    query_patterns: Vec<QueryPattern>,
}

#[allow(unused)]
impl IndexDesigner {
    fn new(conn: Connection, table: &str) -> Self {
        Self {
            conn,
            table: table.to_string(),
            query_patterns: Vec::new(),
        }
    }

    fn add_query_pattern(
        &mut self,
        sql: &str,
        frequency: i32,
        where_cols: &[&str],
        select_cols: &[&str],
    ) {
        self.query_patterns.push(QueryPattern {
            sql: sql.to_string(),
            frequency,
            columns: where_cols.iter().map(|s| s.to_string()).collect(),
            select_columns: select_cols.iter().map(|s| s.to_string()).collect(),
        });
    }

    fn analyze(&self) -> Result<Vec<IndexSuggestion>> {
        let mut suggestions = Vec::new();

        for pattern in &self.query_patterns {
            // 1. 计算每列的选择性
            let selectivities = self.calculate_selectivities(&pattern.columns)?;

            // 2. 评估索引价值
            let (benefit, cost) = self.evaluate_index(&pattern, &selectivities);

            // 3. 如果值得建索引，生成建议
            if benefit > cost {
                let mut index_cols = Vec::new();
                // WHERE 列按选择性排序
                let mut sorted_cols: Vec<_> = pattern
                    .columns
                    .iter()
                    .map(|col| (col, selectivities.get(col).unwrap_or(&0.0)))
                    .collect();
                sorted_cols.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
                for (col, _) in sorted_cols {
                    index_cols.push(col.clone());
                }

                // 如果是组合索引，添加 SELECT 列
                let select_cols: Vec<_> = pattern
                    .select_columns
                    .iter()
                    .filter(|col| !index_cols.contains(col))
                    .cloned()
                    .collect();
                for col in select_cols {
                    if index_cols.len() < 5 {
                        // 限制索引列数
                        index_cols.push(col);
                    }
                }

                suggestions.push(IndexSuggestion {
                    table: self.table.clone(),
                    columns: index_cols,
                    index_type: IndexType::Composite,
                    predicted_benefit: benefit,
                    predicted_cost: cost,
                });
            }
        }
        Ok(suggestions)
    }

    fn calculate_selectivities(
        &self,
        columns: &[String],
    ) -> Result<std::collections::HashMap<String, f64>> {
        let mut map = std::collections::HashMap::new();
        let total: i64 =
            self.conn
                .query_row(&format!("SELECT COUNT(*) FROM {}", self.table), [], |row| {
                    row.get(0)
                })?;
        if total == 0 {
            return Ok(map);
        }

        for col in columns {
            let distinct: i64 = self.conn.query_row(
                &format!("SELECT COUNT(DISTINCT {}) FROM {}", col, self.table),
                [],
                |row| row.get(0),
            )?;
            map.insert(col.clone(), distinct as f64 / total as f64);
        }
        Ok(map)
    }

    fn evaluate_index(
        &self,
        pattern: &QueryPattern,
        selectivities: &std::collections::HashMap<String, f64>,
    ) -> (f64, f64) {
        let avg_selectivity = selectivities.values().sum::<f64>() / selectivities.len() as f64;
        let benefit = pattern.frequency as f64 * avg_selectivity * 100.0;
        let cost = 1.0 + selectivities.len() as f64 * 0.2; // 每个索引增加约 20% 写入开销
        (benefit, cost)
    }
}

// fn show() {

// }
