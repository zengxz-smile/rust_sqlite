use criterion::Criterion;
use rusqlite::{Connection, params};

#[allow(unused)]
/// 准备数据库连接（每次基准迭代都会重新创建，保证独立干净的环境）
fn setup_conn() -> Connection {
    //Connection::open("filename.db") - 打开或创建磁盘文件
    let conn = Connection::open_in_memory().unwrap(); // 内存库，屏蔽磁盘 I/O 干扰，精确测量 SQLite 逻辑性能
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)",
        [],
    )
    .unwrap();
    conn
}

#[allow(unused)]
/// 基准1：逐条插入（自动提交）
pub fn bench_insert_row_by_row(c: &mut Criterion) {
    c.bench_function("insert_row_by_row", |b| {
        b.iter(|| {
            let conn = setup_conn();
            for i in 0..100 {
                conn.execute(
                    "INSERT INTO users (name, age) VALUES (?1, ?2)",
                    params![format!("user_{}", i), i],
                )
                .unwrap();
            }
        })
    });
}

#[allow(unused)]
/// 基准2：事务内逐条插入
pub fn bench_insert_row_by_row_tx(c: &mut Criterion) {
    c.bench_function("insert_row_by_row_tx", |b| {
        b.iter(|| {
            let mut conn = setup_conn();
            let tx = conn.transaction().unwrap(); //开启事务
            for i in 0..100 {
                tx.execute(
                    "INSERT INTO users (name, age) VALUES (?1, ?2)",
                    params![format!("user_{}", i), i],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        })
    });
} 

#[allow(unused)]
/// 基准3：多值插入（单条 SQL 插入 100 行）
pub fn bench_insert_multi_values(c: &mut Criterion) {
    c.bench_function("insert_multi_values", |b| {
        b.iter(|| {
            let conn = setup_conn();
            // 构建：INSERT INTO users (name, age) VALUES (?1, ?2), (?3, ?4), ...
            let mut sql = String::from("INSERT INTO users (name, age) VALUES ");
            let placeholders: Vec<String> = (0..100)
                .map(|i| format!("(?{}, ?{})", 2 * i + 1, 2 * i + 2))
                .collect();
            sql.push_str(&placeholders.join(", "));

            // 构建参数列表：flat 向量
            let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            for i in 0..100 {
                args.push(Box::new(format!("user_{}", i)));
                args.push(Box::new(i));
            }
            // rusqlite要求&[&dyn ToSql]，需要把引用收集起来
            let param_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();

            conn.execute(&sql, &param_refs[..]).unwrap();
        })
    });
}