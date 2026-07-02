//窗口函数
//窗口函数允许你在不改变行数的情况下，对每一行计算其所在“窗口”（即一组行）的聚合值或排名。这与 GROUP BY 不同，后者会将多行压缩为一行。
//窗口函数非常适合分析型查询，如计算移动平均、累计和、排名等。
//基本语法
// SELECT 
//     列1, 列2,
//     窗口函数() OVER (
//         PARTITION BY 分组列 
//         ORDER BY 排序列 
//         ROWS/RANGE BETWEEN 开始 AND 结束
//     ) AS 别名
// FROM 表;
// PARTITION BY：将数据分组，窗口函数在每个分组内独立计算。
// ORDER BY：定义窗口内的排序顺序。
// 帧（Frame）：定义窗口内包含的行范围，如 ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW（从组内第一行到当前行）。
//常用的窗口函数：
// ROW_NUMBER() - 为窗口内的每一行分配一个唯一的序号（从1开始）
// RANK() - 排名，值相同则并列，但会跳过后续名次（如1,1,3）
// DENSE_RANK() - 排名，值相同则并列，但不跳过名次（如1,1,2）
// LAG(expr, offset, default) - 访问当前行之前第 offset 行的值
// LEAD(expr, offset, default) - 访问当前行之后第 offset 行的值
// SUM(col) OVER(...) - 窗口内的累计和
// AVG(col) OVER(...) - 窗口内的移动平均
//帧（Frame）的定义
// ROWS：基于物理行数（精确控制行数）。
// RANGE：基于值范围（如当前值的 ±10）。
// 常用边界：
// UNBOUNDED PRECEDING：组内第一行
// CURRENT ROW：当前行
// UNBOUNDED FOLLOWING：组内最后一行
// n PRECEDING：前 n 行
// n FOLLOWING：后 n 行
//示例：SUM(amount) OVER (PARTITION BY product_id ORDER BY sale_date ROWS UNBOUNDED PRECEDING) -- 计算从组内第一行到当前行的累计销售额

//Rust代码示例:
use rusqlite::{Connection, params};

#[allow(unused)]
/// 建表与数据准备
pub fn setup_sales(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE sales (
            id INTEGER PRIMARY KEY,
            product TEXT,
            sale_date TEXT,
            amount REAL
        )",
        [],
    )?;

    // 插入示例数据
    let data = vec![
        ("A", "2026-01-01", 100.0),
        ("A", "2026-01-02", 150.0),
        ("A", "2026-01-03", 80.0),
        ("B", "2026-01-01", 200.0),
        ("B", "2026-01-02", 250.0),
        ("B", "2026-01-03", 300.0),
    ];
    for (product, date, amount) in data {
        conn.execute(
            "INSERT INTO sales (product, sale_date, amount) VALUES (?1, ?2, ?3)",
            params![product, date, amount],
        )?;
    }
    Ok(())
}

#[allow(unused)]
/// 查询：每个产品的每日销售额排名
pub fn rank_sales(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT 
            product,
            sale_date,
            amount,
            RANK() OVER (PARTITION BY product ORDER BY amount DESC) AS rank
        FROM sales
        ORDER BY product, rank"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?, row.get::<_, i64>(3)?))
    })?;

    for row in rows {
        let (product, date, amount, rank) = row?;
        println!("产品: {}, 日期: {}, 金额: {}, 排名: {}", product, date, amount, rank);
    }
    Ok(())
}

#[allow(unused)]
///查询：计算每日销售额的累计和（按产品分组）
pub fn cumulative_sum(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT 
            product,
            sale_date,
            amount,
            SUM(amount) OVER (PARTITION BY product ORDER BY sale_date ROWS UNBOUNDED PRECEDING) AS cumulative
        FROM sales
        ORDER BY product, sale_date"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?, row.get::<_, f64>(3)?))
    })?;

    for row in rows {
        let (product, date, amount, cumulative) = row?;
        println!("产品: {}, 日期: {}, 金额: {}, 累计: {}", product, date, amount, cumulative);
    }
    Ok(())
}

#[allow(unused)]
///查询：移动平均（前一行、当前行、后一行）
pub fn moving_avg(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT 
            product,
            sale_date,
            amount,
            AVG(amount) OVER (PARTITION BY product ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS moving_avg
        FROM sales"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?, row.get::<_, f64>(3)?))
    })?;

    for row in rows {
        let (product, date, amount, avg) = row?;
        println!("产品: {}, 日期: {}, 金额: {}, 移动平均: {}", product, date, amount, avg);
    }
    Ok(())
}

#[allow(unused)]
pub fn show(){
    let conn = Connection::open_in_memory().unwrap();
    println!("=========创建数据库============");
    let _ = setup_sales(&conn);
    println!("数据库创建成功！");
    println!();
    println!("---查询每个产品的每日销售额排名---");
    let _ = rank_sales(&conn);
    println!();
    println!("---计算每日销售额的累计和（按产品分组）---");
    println!();
    let _ = cumulative_sum(&conn);
    println!();
    println!("---移动平均（前一行、当前行、后一行）---");
    let _ = moving_avg(&conn);
}