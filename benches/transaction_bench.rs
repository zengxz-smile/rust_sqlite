use criterion::{Criterion, BenchmarkId};
use rusqlite::{Connection, params};

#[allow(unused)]
pub fn bench_auto_commit(c: &mut Criterion) {
    c.bench_function("flush_per_row", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap(); // 注意：内存库不会真的 fsync，但为了实测磁盘，改用临时文件
            // 为了精确测量，换成临时文件：
            // let conn = Connection::open("/tmp/test_flush.db").unwrap();
            conn.execute("CREATE TABLE t (id INTEGER)", []).unwrap();
            for i in 0..100 {
                conn.execute("INSERT INTO t VALUES (?1)", params![i]).unwrap();
            }
        })
    });
}

#[allow(unused)]
pub fn bench_explicit_tx(c: &mut Criterion) {
    c.bench_function("flush_per_batch", |b| {
        b.iter(|| {
            let mut conn = Connection::open("test_flush.db").unwrap(); // 磁盘库
            conn.execute("CREATE TABLE IF NOT EXISTS t (id INTEGER)", []).unwrap();
            let tx = conn.transaction().unwrap();
            for i in 0..100 {
                tx.execute("INSERT INTO t VALUES (?1)", params![i]).unwrap();
            }
            tx.commit().unwrap();
        })
    });
}

#[allow(unused)]
pub fn bench_with_savepoint(c: &mut Criterion) {
    c.bench_function("insert_with_savepoint", |b| {
        b.iter(|| {
            let mut conn = Connection::open_in_memory().unwrap();
            conn.execute("CREATE TABLE t (id INTEGER)", []).unwrap();
            let mut tx = conn.transaction().unwrap();
            for i in 0..100 {
                if i % 10 == 0 {
                    // 每 100 行创建保存点，并在插入完成后立即释放
                    let sp = tx.savepoint().unwrap();
                    sp.execute("INSERT INTO t VALUES (?1)", params![i]).unwrap();
                    sp.commit().unwrap();
                } else {
                    tx.execute("INSERT INTO t VALUES (?1)", params![i]).unwrap();
                }
            }
            tx.commit().unwrap();
        })
    });
}

#[allow(unused)]
pub fn bench_insert_without_check(c: &mut Criterion) {
    c.bench_function("insert_without_check", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute("CREATE TABLE t (id INTEGER)", []).unwrap();
            for i in 0..100 {
                conn.execute("INSERT INTO t VALUES (?1)", params![i]).unwrap();
            }
        })
    });
}

#[allow(unused)]
pub fn bench_insert_with_check(c: &mut Criterion) {
    c.bench_function("insert_with_check", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            //CHECK 约束大约会带来 5%~15% 的额外开销，但换来的是数据一致性的硬性保证。
            conn.execute("CREATE TABLE t (id INTEGER CHECK(id > 0))", []).unwrap();
            for i in 1..101 { // 保证插入的值符合约束
                conn.execute("INSERT INTO t VALUES (?1)", params![i]).unwrap();
            }
        })
    });
}

#[allow(unused)]
pub fn bench_journal_mode(c: &mut Criterion) {
    let modes = vec!["DELETE", "TRUNCATE", "PERSIST"];
    let mut group = c.benchmark_group("JournalMode");
    
    // 使用系统临时目录，避免污染项目根目录
    // let db_path: PathBuf = std::env::temp_dir().join("test_journal.db");

    for mode in &modes {
        group.bench_with_input(BenchmarkId::from_parameter(mode), mode, |b, &mode| {
            b.iter(|| {
                // 使用磁盘文件，不能用内存库（内存库无 journal）
                let mut conn = Connection::open("test_journal.db").unwrap();
                conn.pragma_update(None, "journal_mode", mode).unwrap();
                conn.execute("CREATE TABLE IF NOT EXISTS t (id INTEGER)", []).unwrap();
                
                let tx = conn.transaction().unwrap();
                for i in 0..100 {
                    tx.execute("INSERT INTO t VALUES (?1)", params![i]).unwrap();
                }
                tx.commit().unwrap();
            })
        });
    }
    group.finish();
}