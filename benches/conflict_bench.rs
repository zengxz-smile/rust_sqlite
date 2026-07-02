use criterion::{Criterion, BenchmarkId};
use rusqlite::{Connection, params};
use std::sync::Arc;
use std::thread;

fn setup_table(conn: &Connection) {
    conn.execute("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT UNIQUE, age INTEGER)", []).unwrap();
    conn.execute("DELETE FROM users", []).unwrap();
}

#[allow(unused)]
pub fn bench_ignore(c: &mut Criterion) {
    c.bench_function("conflict_ignore", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_table(&conn);
            // 生成 5000 个名字，其中 2500 个重复一次
            for i in 0..5000 {
                let name_id = i % 2500; // 0..2499 循环
                let name = format!("user_{}", name_id);
                let age = rand::random_range(18..80);
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO users (name, age) VALUES (?1, ?2)",
                    params![name, age],
                );
            }
        })
    });
}

#[allow(unused)]
pub fn bench_replace(c: &mut Criterion) {
    c.bench_function("conflict_replace", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_table(&conn);
            for i in 0..5000 {
                let name_id = i % 2500;
                let name = format!("user_{}", name_id);
                let age = rand::random_range(18..80);
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO users (name, age) VALUES (?1, ?2)",
                    params![name, age],
                );
            }
        })
    });
}

#[allow(unused)]
pub fn bench_upsert(c: &mut Criterion) {
    c.bench_function("conflict_upsert", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_table(&conn);
            for i in 0..5000 {
                let name_id = i % 2500;
                let name = format!("user_{}", name_id);
                let age = rand::random_range(18..80);
                let _ = conn.execute(
                    "INSERT INTO users (name, age) VALUES (?1, ?2)
                     ON CONFLICT(name) DO UPDATE SET age = excluded.age",
                    params![name, age],
                );
            }
        })
    });
}

/// 执行读取操作（返回行数）
fn read_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0)).unwrap()
}

/// 写入线程：持续插入数据
fn writer(conn: Connection, stop: Arc<std::sync::atomic::AtomicBool>) {
    let mut i = 0;
    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
        conn.execute("INSERT INTO t VALUES (?1)", params![i]).unwrap();
        i += 1;
        thread::sleep(std::time::Duration::from_micros(100)); // 控制写入速度
    }
}

#[allow(unused)]
fn bench_read_isolation(c: &mut Criterion) {
    let modes = vec!["SERIALIZABLE", "READ_UNCOMMITTED"];
    let mut group = c.benchmark_group("ReadIsolation");

    for mode in &modes {
        group.bench_with_input(BenchmarkId::from_parameter(mode), mode, |b, &mode| {
            b.iter(|| {
                // 每个迭代（bench 内部）启动独立的数据库，避免跨迭代干扰
                let db_path = std::env::temp_dir().join("isolation_test.db");
                let _ = std::fs::remove_file(&db_path); // 清理旧文件

                // 主连接（用于建表、读）
                let conn_main = Connection::open(&db_path).unwrap();
                if mode == "READ_UNCOMMITTED" {
                    conn_main.pragma_update(None, "read_uncommitted", true).unwrap();
                }
                conn_main.execute("CREATE TABLE t (id INTEGER)", []).unwrap();

                // 写入连接（共享缓存）
                let conn_writer = Connection::open(&db_path).unwrap(); // 默认共享缓存（对于磁盘文件，默认是开启的）
                // 注意：必须保证所有连接以相同方式打开，才能共享缓存。

                let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let stop_clone = stop.clone();
                let writer_handle = thread::spawn(move || {
                    writer(conn_writer, stop_clone);
                });

                // 主线程执行 100 次读取
                for _ in 0..100 {
                    let _ = read_count(&conn_main);
                }

                stop.store(true, std::sync::atomic::Ordering::Relaxed);
                writer_handle.join().unwrap();

                // 清理文件
                let _ = std::fs::remove_file(&db_path);
            })
        });
    }
    group.finish();
}