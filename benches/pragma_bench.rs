use criterion::{Criterion, BenchmarkId};
use rusqlite::{Connection, params};

fn setup_data(conn: &Connection, n: usize) {
    conn.execute("CREATE TABLE t (id INTEGER, data TEXT)", []).unwrap();
    let mut stmt = conn.prepare("INSERT INTO t VALUES (?1, ?2)").unwrap();
    for i in 0..n {
        stmt.execute(params![i as i64, format!("data_{}", i)]).unwrap();
    }
}

#[allow(unused)]
pub fn bench_cache_sizes(c: &mut Criterion) {
    let cache_sizes = vec![500, 2000, 8000, 32000, 128000];
    let n_rows = 100000;
    
    let mut group = c.benchmark_group("cache_size");
    
    for &size in &cache_sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                // 每个迭代使用独立数据库，避免缓存污染
                let conn = Connection::open_in_memory().unwrap();
                conn.pragma_update(None, "cache_size", size).unwrap();
                setup_data(&conn, n_rows);
                
                // 执行随机查询
                let mut stmt = conn.prepare("SELECT data FROM t WHERE id = ?1").unwrap();
                for _ in 0..1000 {
                    let id :i32= rand::random_range(0..n_rows) as i32;
                    let _: String = stmt.query_row([id], |row| row.get(0)).unwrap();
                }
            })
        });
    }
    group.finish();
}