use criterion::{Criterion, BenchmarkId};
use rusqlite::{Connection, params};

fn setup_large_sales(conn: &Connection, n: usize) {
    conn.execute("CREATE TABLE sales (id INTEGER PRIMARY KEY, product TEXT, amount REAL)", []).unwrap();
    let mut stmt = conn.prepare("INSERT INTO sales (product, amount) VALUES (?1, ?2)").unwrap();
    for i in 0..n {
        let product = if i % 2 == 0 { "A" } else { "B" };
        stmt.execute(params![product, i as f64 * 0.5]).unwrap();
    }
}

#[allow(unused)]
pub fn bench_window(c: &mut Criterion) {
    let sizes = vec![10, 100, 1000];
    let mut group = c.benchmark_group("WindowFunction");
    for &size in &sizes {
        group.bench_with_input(BenchmarkId::new("window", size), &size, |b, &size| {
            b.iter(|| {
                let conn = Connection::open_in_memory().unwrap();
                setup_large_sales(&conn, size);
                let mut stmt = conn.prepare(
                    "SELECT product, SUM(amount) OVER (PARTITION BY product ORDER BY id ROWS UNBOUNDED PRECEDING) FROM sales"
                ).unwrap();
                let rows = stmt.query_map([], |row| row.get::<_, f64>(0)).unwrap();
                for _row in rows {}
            })
        });
        group.bench_with_input(BenchmarkId::new("subquery", size), &size, |b, &size| {
            b.iter(|| {
                let conn = Connection::open_in_memory().unwrap();
                setup_large_sales(&conn, size);
                let mut stmt = conn.prepare(
                    "SELECT (SELECT SUM(s2.amount) FROM sales s2 WHERE s2.product = s1.product AND s2.id <= s1.id) FROM sales s1"
                ).unwrap();
                let rows = stmt.query_map([], |row| row.get::<_, f64>(0)).unwrap();
                for _row in rows {}
            })
        });
    }
    group.finish();
}