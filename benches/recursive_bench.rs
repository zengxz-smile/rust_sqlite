use criterion::Criterion;
use rusqlite::{Connection, params};
use std::collections::VecDeque;

/// 准备数据：生成一棵随机树，每个节点最多 5 个子节点，总节点数约 10000
fn setup_tree(conn: &Connection, n_nodes: usize) {
    conn.execute("CREATE TABLE employees (id INTEGER PRIMARY KEY, manager_id INTEGER)", [])
        .unwrap();
    // 根节点 id = 1，manager_id = NULL
    conn.execute("INSERT INTO employees (id, manager_id) VALUES (1, NULL)", [])
        .unwrap();
    // 使用队列生成 BFS 树
    let mut queue = VecDeque::new();
    queue.push_back(1);
    let mut next_id = 2;
    while let Some(parent) = queue.pop_front() {
        if next_id > n_nodes {
            break;
        }
        // 随机子节点数量：0~5
        let children = rand::random_range(0..=5);
        for _ in 0..children {
            if next_id > n_nodes {
                break;
            }
            conn.execute(
                "INSERT INTO employees (id, manager_id) VALUES (?1, ?2)",
                params![next_id as i32, parent as i32],
            ).unwrap();
            queue.push_back(next_id);
            next_id += 1;
        }
    }
}

/// 方式 A：递归 CTE 查询所有下属（单个 SQL）
fn query_subordinates_cte(conn: &Connection, root_id: i32) -> Vec<i32> {
    let mut stmt = conn
        .prepare(
            "WITH RECURSIVE sub AS (
                SELECT id FROM employees WHERE manager_id = ?1
                UNION ALL
                SELECT e.id FROM employees e JOIN sub ON e.manager_id = sub.id
            )
            SELECT id FROM sub",
        )
        .unwrap();
    let rows = stmt.query_map(params![root_id], |row| row.get(0)).unwrap();
    rows.map(|r| r.unwrap()).collect()
}

/// 方式 B：Rust 循环查询（多次 SQL）
fn query_subordinates_rust_loop(conn: &Connection, root_id: i32) -> Vec<i32> {
    let mut result = Vec::new();
    let mut frontier = vec![root_id];
    while let Some(parent) = frontier.pop() {
        // 查询直接下属
        let mut stmt = conn
            .prepare("SELECT id FROM employees WHERE manager_id = ?1")
            .unwrap();
        let rows = stmt.query_map(params![parent], |row| row.get(0)).unwrap();
        for id in rows {
            let id = id.unwrap();
            result.push(id);
            frontier.push(id);
        }
    }
    result
}

#[allow(unused)]
pub fn bench_recursive(c: &mut Criterion) {
    let n_nodes = 100;
    let root_id = 1;

    let mut group = c.benchmark_group("RecursiveCTE");

    // 基准 CTE
    group.bench_function("cte", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_tree(&conn, n_nodes);
            let _ = query_subordinates_cte(&conn, root_id);
        })
    });

    // 基准 Rust 循环
    group.bench_function("rust_loop", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_tree(&conn, n_nodes);
            let _ = query_subordinates_rust_loop(&conn, root_id);
        })
    });

    group.finish();
}