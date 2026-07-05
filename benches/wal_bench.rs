use criterion::{Criterion, BenchmarkId, Throughput};
use rusqlite::{Connection};
use std::thread;

// ============================================================
// 1. 辅助函数：准备数据
// ============================================================

fn prepare_connection(journal_mode: &str) -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "journal_mode", journal_mode).unwrap();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, data TEXT)", []).unwrap();
    conn
}

fn prepare_file_connection(journal_mode: &str, path: &str) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", journal_mode).unwrap();
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, data TEXT)", []).unwrap();
    conn
}

// ============================================================
// 2. 写入基准：三种模式的吞吐量
// ============================================================
#[allow(unused)]
pub fn bench_write_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_throughput");
    let n_rows = 1000;
    group.throughput(Throughput::Elements(n_rows as u64));
    
    for &mode in &["DELETE", "TRUNCATE", "WAL"] {
        group.bench_with_input(BenchmarkId::new(mode, n_rows), &mode, |b, &mode| {
            b.iter(|| {
                // 使用内存数据库，排除 I/O 干扰
                let conn = prepare_connection(mode);
                for i in 0..n_rows {
                    conn.execute(
                        "INSERT INTO t (data) VALUES (?1)",
                        [format!("data_{}", i)],
                    ).unwrap();
                }
            })
        });
    }
    group.finish();
}

// ============================================================
// 3. 写入基准（磁盘文件，真实 fsync）
// ============================================================
#[allow(unused)]
pub fn bench_write_disk_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_disk_throughput");
    let n_rows = 1000;
    group.throughput(Throughput::Elements(n_rows as u64));
    
    for &mode in &["DELETE", "TRUNCATE", "WAL"] {
        group.bench_with_input(BenchmarkId::new(mode, n_rows), &mode, |b, &mode| {
            b.iter(|| {
                // 使用临时文件，测试真实磁盘 I/O
                let path = format!("/tmp/wal_bench_{}.db", std::process::id());
                let conn = prepare_file_connection(mode, &path);
                for i in 0..n_rows {
                    conn.execute(
                        "INSERT INTO t (data) VALUES (?1)",
                        [format!("data_{}", i)],
                    ).unwrap();
                }
                drop(conn);
                let _ = std::fs::remove_file(&path);
            })
        });
    }
    group.finish();
}

// ============================================================
// 4. 事务提交基准（对比单条 vs 批量）
// ============================================================
#[allow(unused)]
pub fn bench_transaction_commit(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_commit");
    let n_rows = 10;
    
    for &mode in &["DELETE", "WAL"] {
        // 单条提交（自动提交）
        group.bench_with_input(
            BenchmarkId::new(format!("{}_auto", mode), n_rows),
            &mode,
            |b, &mode| {
                b.iter(|| {
                    let conn = prepare_connection(mode);
                    for i in 0..n_rows {
                        conn.execute(
                            "INSERT INTO t (data) VALUES (?1)",
                            [format!("data_{}", i)],
                        ).unwrap();
                    }
                })
            }
        );
        
        // 批量提交（显式事务）
        group.bench_with_input(
            BenchmarkId::new(format!("{}_batch", mode), n_rows),
            &mode,
            |b, &mode| {
                b.iter(|| {
                    let mut conn = prepare_connection(mode);
                    let tx = conn.transaction().unwrap();
                    for i in 0..n_rows {
                        tx.execute(
                            "INSERT INTO t (data) VALUES (?1)",
                            [format!("data_{}", i)],
                        ).unwrap();
                    }
                    tx.commit().unwrap();
                })
            }
        );
    }
    group.finish();
}

// ============================================================
// 5. 读取并发基准（模拟多读者）
// ============================================================
#[allow(unused)]
pub fn bench_concurrent_readers(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_readers");
    let n_readers = 10;
    let n_queries_per_reader = 100;
    
    for &mode in &["DELETE", "WAL"] {
        group.bench_with_input(
            BenchmarkId::new(mode, n_readers),
            &mode,
            |b, &mode| {
                b.iter(|| {
                    // 创建主连接并准备数据
                    let main_conn = prepare_connection(mode);
                    // 插入一些数据
                    for i in 0..1000 {
                        main_conn.execute(
                            "INSERT INTO t (data) VALUES (?1)",
                            [format!("data_{}", i)],
                        ).unwrap();
                    }
                    
                    // 如果是 WAL 模式，共享内存文件需要持久化
                    // 使用磁盘文件以便多连接共享
                    let path = format!("/tmp/wal_concurrent_{}.db", std::process::id());
                    let main_conn = prepare_file_connection(mode, &path);
                    for i in 0..1000 {
                        main_conn.execute(
                            "INSERT INTO t (data) VALUES (?1)",
                            [format!("data_{}", i)],
                        ).unwrap();
                    }
                    
                    // 启动多个读线程
                    let handles: Vec<_> = (0..n_readers).map(|_| {
                        let path_clone = path.clone();
                        thread::spawn(move || {
                            let conn = Connection::open(&path_clone).unwrap();
                            // 如果是 WAL，连接自动继承 WAL 模式
                            if mode == "WAL" {
                                // 不需要额外设置，连接自动支持
                            }
                            for _ in 0..n_queries_per_reader {
                                let count: i64 = conn.query_row(
                                    "SELECT COUNT(*) FROM t WHERE data LIKE 'data_%'",
                                    [],
                                    |row| row.get(0),
                                ).unwrap();
                                // 防止编译器优化掉查询
                                if count == 0 { panic!("查询返回 0") }
                            }
                        })
                    }).collect();
                    
                    for handle in handles {
                        handle.join().unwrap();
                    }
                    
                    drop(main_conn);
                    let _ = std::fs::remove_file(&path);
                    let _ = std::fs::remove_file(format!("{}-wal", path));
                    let _ = std::fs::remove_file(format!("{}-shm", path));
                })
            }
        );
    }
    group.finish();
}

// ============================================================
// 6. Checkpoint 开销基准
// ============================================================
#[allow(unused)]
pub fn bench_checkpoint_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoint_overhead");
    
    group.bench_function("checkpoint_auto", |b| {
        b.iter(|| {
            let conn = prepare_connection("WAL");
            // 插入数据，触发自动 checkpoint
            for i in 0..5000 {
                conn.execute(
                    "INSERT INTO t (data) VALUES (?1)",
                    [format!("data_{}", i)],
                ).unwrap();
            }
        })
    });
    
    group.bench_function("checkpoint_manual", |b| {
        b.iter(|| {
            let conn = prepare_connection("WAL");
            // 禁用自动 checkpoint
            conn.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
            for i in 0..5000 {
                conn.execute(
                    "INSERT INTO t (data) VALUES (?1)",
                    [format!("data_{}", i)],
                ).unwrap();
            }
            // 手动 checkpoint
            conn.execute_batch("PRAGMA wal_checkpoint(FULL);").unwrap();
        })
    });
    
    group.finish();
}