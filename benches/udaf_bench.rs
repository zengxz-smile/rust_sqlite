use criterion:: Criterion;
use rusqlite::{Connection, Result};
use rusqlite::functions::{Aggregate, Context, FunctionFlags};

// 聚合状态：存储所有数值
#[derive(Default)]
#[allow(unused)]
struct MedianState {
    values: Vec<f64>,
}

// 聚合函数主体，不需要额外状态
#[allow(unused)]
struct Median;

impl Aggregate<MedianState, Option<f64>> for Median {
    // 初始化聚合状态
    fn init(&self, _ctx: &mut Context<'_>) -> Result<MedianState> {
        Ok(MedianState::default())
    }

    // 处理每一行数据，更新聚合状态
    fn step(&self, ctx: &mut Context<'_>, acc: &mut MedianState) -> Result<()> {
        // 尝试读取第一个参数（索引0），若为 NULL 则忽略
        if let Ok(val) = ctx.get::<f64>(0) {
            acc.values.push(val);
        }
        Ok(())
    }

    // 所有行处理完成后，计算并返回最终结果
    fn finalize(
        &self,
        _ctx: &mut Context<'_>,
        acc: Option<MedianState>,
    ) -> Result<Option<f64>> {
        match acc {
            Some(mut state) => {
                if state.values.is_empty() {
                    return Ok(None);
                }
                state.values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let len = state.values.len();
                let median = if len % 2 == 0 {
                    (state.values[len / 2 - 1] + state.values[len / 2]) / 2.0
                } else {
                    state.values[len / 2]
                };
                Ok(Some(median))
            }
            None => Ok(None),
        }
    }
}

// ---------- 辅助函数：准备测试数据 ----------
fn setup_data(conn: &Connection, n: usize) {
    conn.execute("CREATE TABLE t (x REAL)", []).unwrap();
    
    let mut stmt = conn.prepare("INSERT INTO t VALUES (?1)").unwrap();
    for _ in 0..n {
        let val = rand::random_range(0.0..100.0);
        stmt.execute([val]).unwrap();
    }
}

#[allow(unused)]
// ---------- 基准函数 ----------
pub fn bench_avg_native(c: &mut Criterion) {
    c.bench_function("avg_native", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_data(&conn, 100);
            let _: f64 = conn
                .query_row("SELECT AVG(x) FROM t", [], |row| row.get(0))
                .unwrap();
        })
    });
}

#[allow(unused)]
pub fn bench_median_udaf(c: &mut Criterion) {
    c.bench_function("median_udaf", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            // 注册自定义聚合函数
            conn.create_aggregate_function(
                "MEDIAN",
                1,
                FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
                Median,
            )
            .unwrap();
            setup_data(&conn, 100);
            let _: Option<f64> = conn
                .query_row("SELECT MEDIAN(x) FROM t", [], |row| row.get(0))
                .unwrap();
        })
    });
}