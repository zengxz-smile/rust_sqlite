//GROUP BY 、UNION ALL 、聚合函数和自定义聚合函数
//SELECT product, SUM(amount) AS total
// FROM sales
// GROUP BY product;

//自定义聚合 MEDIAN，计算分组的中位数
//1.定义状态结构体
// 状态：存储所有数据（为简单起见，收集到 Vec，然后排序计算中位数）
// 实际应用中，可用更高效的两堆法，但此处为演示。
// struct MedianState {
//     values: Vec<f64>,
// }
//2.实现聚合函数
// rusqlite 要求为聚合函数提供三个函数：
//  step：处理每一行，更新状态。
//  finalize：输出最终结果（返回 Option<T> 或 T）。
//  注册时指定状态类型和上述函数。
//示例：
// step 函数：将新值加入状态
// fn step_median(state: &mut MedianState, value: ValueRef<'_>) -> SqliteResult<()> {
//     if let Ok(val) = value.as_f64() {
//         state.values.push(val);
//     } // 忽略 NULL
//     Ok(())
// }
// finalize 函数：计算中位数
// fn finalize_median(state: MedianState) -> SqliteResult<Option<f64>> {
//     let mut vals = state.values;
//     if vals.is_empty() {
//         return Ok(None);
//     }
//     vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
//     let len = vals.len();
//     if len % 2 == 0 {
//         let mid = len / 2;
//         Ok(Some((vals[mid - 1] + vals[mid]) / 2.0))
//     } else {
//         Ok(Some(vals[len / 2]))
//     }
// }
// 注册聚合函数到连接
// fn register_median(conn: &Connection) -> Result<()> {
//     conn.create_aggregate(
//         "MEDIAN",           // SQL 函数名
//         1,                  // 参数个数（这里只接受一个参数）
//         || MedianState { values: Vec::new() },  // 初始状态
//         step_median,        // step 函数
//         finalize_median,    // finalize 函数
//     )
// }

//Rust代码部分：
use rusqlite::functions::{Aggregate, Context, FunctionFlags};
use rusqlite::{Connection, Result as SqliteResult, params};

#[allow(unused)]
fn setup_sales(conn: &Connection, n: usize) {
    conn.execute(
        "CREATE TABLE sales (product TEXT, region TEXT, amount REAL)",
        [],
    )
    .unwrap();
    let mut stmt = conn
        .prepare("INSERT INTO sales VALUES (?1, ?2, ?3)")
        .unwrap();
    let products = ["A", "B", "C"];
    let regions = ["North", "South", "East", "West"];
    for i in 0..n {
        let product = products[i % 3];
        let region = regions[i % 4];
        stmt.execute(params![product, region, i as f64]).unwrap();
    }
}

#[allow(unused)]
// 基础 GROUP BY + GROUP_CONCAT
fn group_concat_example(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT product, GROUP_CONCAT(amount) AS amounts
         FROM sales
         GROUP BY product",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (product, amounts) = row?;
        println!("Product: {}, Amounts: {}", product, amounts);
    }
    Ok(())
}

#[allow(unused)]
// 使用 UNION ALL（含小计）
fn rollup_example(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT product, region, SUM(amount) FROM sales GROUP BY product, region
             UNION ALL
             SELECT product, NULL, SUM(amount) FROM sales GROUP BY product
             UNION ALL
             SELECT NULL, NULL, SUM(amount) FROM sales",
    )?;

    let rows = stmt.query_and_then(
        [],
        |row| -> Result<(Option<String>, Option<String>, f64), rusqlite::Error> {
            let product: Option<String> = row.get(0)?;
            let region: Option<String> = row.get(1)?;
            let total: f64 = row.get(2)?;
            Ok((product, region, total))
        },
    )?;

    for row in rows {
        let (product, region, total) = row?;
        println!(
            "Product: {:?}, Region: {:?}, Total: {}",
            product, region, total
        );
    }
    Ok(())
}

#[allow(unused)]
// 使用 JSON_GROUP_ARRAY（将分组数据打包成 JSON）
fn json_group_array_example(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT product, JSON_GROUP_ARRAY(amount) AS amounts_json
         FROM sales
         GROUP BY product",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (product, json) = row?;
        println!("Product: {}, JSON: {}", product, json);
    }
    Ok(())
}

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
    fn init(&self, _ctx: &mut Context<'_>) -> SqliteResult<MedianState> {
        Ok(MedianState::default())
    }

    // 处理每一行数据，更新聚合状态
    fn step(&self, ctx: &mut Context<'_>, acc: &mut MedianState) -> SqliteResult<()> {
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
    ) -> SqliteResult<Option<f64>> {
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

#[allow(unused)]
fn use_median(conn: &Connection) -> SqliteResult<()> {
    // 注册聚合函数
    conn.create_aggregate_function(
        "MEDIAN",   // 1. SQL函数名
        1,  // 2. 参数个数
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC, // 3. 函数标志
        Median, // 4. 实现了 Aggregate trait 的类型
    )?;

    // 准备示例数据
    conn.execute("CREATE TABLE test (x REAL)", [])?;
    for i in 1..=10 {
        conn.execute("INSERT INTO test VALUES (?1)", params![i as f64])?;
    }

    // 查询中位数
    let median: Option<f64> = conn.query_row("SELECT MEDIAN(x) FROM test", [], |row| row.get(0))?;
    println!("中位数: {:?}", median); // 应为 5.5
    Ok(())
}

#[allow(unused)]
pub fn show() {
    let conn = Connection::open_in_memory().unwrap();
    setup_sales(&conn, 10);
    let version: String = conn
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .unwrap();
    println!("SQLite version: {}", version);

    let _ = group_concat_example(&conn);
    let _ = json_group_array_example(&conn);
    let _ = rollup_example(&conn);

    use_median(&conn);
}
