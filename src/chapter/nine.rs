//递归CTE深入
// WITH RECURSIVE cte_name(列1, 列2, ...) AS (
//     -- 初始查询（锚点成员）
//     SELECT ... FROM ... WHERE 初始条件
//     UNION ALL   -- 或 UNION（去重）
//     -- 递归查询（递归成员）
//     SELECT ... FROM cte_name JOIN ... ON 递归条件
// )
// SELECT * FROM cte_name;
//执行过程：
// 1.执行锚点查询，结果作为第一轮结果集。
// 2.将上一轮结果集作为输入，执行递归查询，产生新一轮结果。
// 3.重复步骤2，直到递归查询返回空集。
// 4.用 UNION ALL 合并所有轮次的结果（若用 UNION，会额外去重）。
//终止条件：
//递归查询中必须包含终止条件（通常在WHERE子句中），否则会无限递归。SQLite默认限制递归深度为1000，可通过PRAGMA recursive_triggers
// 调整（实际上深度限制由 sqlite3_limit(SQLITE_LIMIT_TRIGGER_DEPTH) 控制，默认 1000）。
//典型应用场景
// 1.树形结构：组织架构、分类目录、评论回复。
// 2.路径枚举：图中两点间的所有路径。
// 3.生成序列：产生连续数字、日期序列。
//递归 CTE 的性能考量
// 深度限制：默认 1000，可通过 PRAGMA recursive_triggers = ON 调整到更大值（或使用 sqlite3_limit()）。
// 索引：在递归连接条件列（如 manager_id、from_node）上建立索引，能大幅提升递归查询速度。
// 去重：如果业务允许，使用 UNION ALL 比 UNION 快（避免额外去重开销）。
// 无限循环防护：务必在递归部分使用 WHERE 条件避免循环（如 INSTR 检查）。

//Rust代码示例:
use rusqlite::{Connection, params};

#[allow(unused)]
fn setup_org(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE employees (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            manager_id INTEGER REFERENCES employees(id)
        )",
        [],
    )?;
    // 插入示例数据
    let data = vec![
        (1, "Alice", None),
        (2, "Bob", Some(1)),
        (3, "Charlie", Some(1)),
        (4, "David", Some(2)),
        (5, "Eve", Some(2)),
        (6, "Frank", Some(3)),
    ];
    for (id, name, mgr) in data {
        conn.execute(
            "INSERT INTO employees (id, name, manager_id) VALUES (?1, ?2, ?3)",
            params![id, name, mgr],
        )?;
    }
    Ok(())
}

///获取 Alice（id=1）的所有下属（包括直接和间接）
#[allow(unused)]
fn get_subordinates(conn: &Connection, manager_id: i32) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "WITH RECURSIVE subordinates AS (
            SELECT id, name, 1 AS level
            FROM employees
            WHERE manager_id = ?1
            UNION ALL
            SELECT e.id, e.name, s.level + 1
            FROM employees e
            INNER JOIN subordinates s ON e.manager_id = s.id
        )
        SELECT id, name, level FROM subordinates ORDER BY level, name",
    )?;
    let rows = stmt.query_map(params![manager_id], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
        ))
    })?;
    for row in rows {
        let (id, name, level) = row?;
        println!("ID: {}, Name: {}, Level: {}", id, name, level);
    }
    Ok(())
}

#[allow(unused)]
fn setup_edges(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute("CREATE TABLE edges (from_node TEXT, to_node TEXT)", [])?;
    let edges = vec![("A", "B"), ("A", "C"), ("B", "D"), ("C", "D"), ("B", "C")];
    for (f, t) in edges {
        conn.execute("INSERT INTO edges VALUES (?1, ?2)", params![f, t])?;
    }
    Ok(())
}

///查找两点间所有路径
#[allow(unused)]
fn find_paths(conn: &Connection, start: &str, end: &str) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "WITH RECURSIVE paths(path, last_node) AS (
            SELECT ?1 || '->' || to_node, to_node
            FROM edges
            WHERE from_node = ?1
            UNION ALL
            SELECT p.path || '->' || e.to_node, e.to_node
            FROM paths p
            JOIN edges e ON p.last_node = e.from_node
            WHERE INSTR(p.path, e.to_node) = 0
        )
        SELECT path FROM paths WHERE last_node = ?2",
    )?;
    let rows = stmt.query_map(params![start, end], |row| row.get::<_, String>(0))?;
    for path in rows {
        println!("路径: {}", path?);
    }
    Ok(())
}

#[allow(unused)]
pub fn show() {
    let conn = Connection::open_in_memory().unwrap();
    setup_org(&conn);
    get_subordinates(&conn,1);

    setup_edges(&conn);
    find_paths(&conn,"A","D");
}
