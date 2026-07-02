use criterion::Criterion;
use rusqlite::{Connection, params};

fn setup_data(conn: &Connection) {
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, age INTEGER)", [])
        .unwrap();
    for i in 0..1000 {
        conn.execute("INSERT INTO users (id, age) VALUES (?1, 0)", params![i])
            .unwrap();
    }
}

#[allow(unused)]
pub fn bench_update_auto_commit(c: &mut Criterion) {
    c.bench_function("update_auto_commit", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_data(&conn);
            for i in 0..1000 {
                conn.execute("UPDATE users SET age = age + 1 WHERE id = ?", params![i])
                    .unwrap();
            }
        })
    });
}

#[allow(unused)]
pub fn bench_update_transaction(c: &mut Criterion) {
    c.bench_function("update_transaction", |b| {
        b.iter(|| {
            let mut conn = Connection::open_in_memory().unwrap();
            setup_data(&conn);
            let tx = conn.transaction().unwrap();
            for i in 0..1000 {
                tx.execute("UPDATE users SET age = age + 1 WHERE id = ?", params![i])
                    .unwrap();
            }
            tx.commit().unwrap();
        })
    });
}

#[allow(unused)]
/// 基准1：自动提交，逐条 DELETE
pub fn bench_delete_auto_commit(c: &mut Criterion) {
    c.bench_function("delete_auto_commit", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_data(&conn);
            for i in 0..1000 {
                conn.execute("DELETE FROM users WHERE id = ?", params![i]).unwrap();
            }
        })
    });
}

#[allow(unused)]
/// 基准2：事务内逐条 DELETE
pub fn bench_delete_transaction(c: &mut Criterion) {
    c.bench_function("delete_transaction", |b| {
        b.iter(|| {
            let mut conn = Connection::open_in_memory().unwrap();
            setup_data(&conn);
            let tx = conn.transaction().unwrap();
            for i in 0..1000 {
                tx.execute("DELETE FROM users WHERE id = ?", params![i]).unwrap();
            }
            tx.commit().unwrap();
        })
    });
}

#[allow(unused)]
/// 基准3：ORDER BY LIMIT 删除（删除 age 最小的 100 条）
pub fn bench_delete_order_by_limit(c: &mut Criterion) {
    c.bench_function("delete_order_by_limit", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_data(&conn);
            conn.execute( // sqlite经典错误，不同时支持ORDER BY + LIMIT，要支持话的得加上where条件
                "DELETE FROM users WHERE id IN(SELECT id FROM users WHERE age > 40 ORDER BY age ASC LIMIT 100)",
                [],
            ).unwrap();
        })
    });
}