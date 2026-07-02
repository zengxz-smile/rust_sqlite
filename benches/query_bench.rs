use criterion::Criterion;
use rusqlite::{Connection, params};

fn setup_data(conn: &Connection) {
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)",
        [],
    ).unwrap();
    let mut stmt = conn.prepare("INSERT INTO users (name, age) VALUES (?1, ?2)").unwrap();
    for i in 0..10000 {
        let age = rand::random_range(18..80);
        stmt.execute(params![format!("user_{}", i), age]).unwrap();
    }
}

#[allow(unused)]
pub fn bench_select_no_index(c: &mut Criterion) {
    c.bench_function("select_age_gt_50_no_index", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_data(&conn);
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM users WHERE age > 50").unwrap();
            //query_row() - 查询单行，期望结果只有一行，否则报错。
            let _count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
            // println!("----bench_select_no_index函数查询大于50岁的用户数量是: {count}");
        })
    });
}

#[allow(unused)]
pub fn bench_select_with_index(c: &mut Criterion) {
    c.bench_function("select_age_gt_50_with_index", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_data(&conn);
            //创建索引
            conn.execute("CREATE INDEX idx_age ON users(age)", []).unwrap();
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM users WHERE age > 50").unwrap();
            let _count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
            //  println!("----bench_select_with_index函数查询大于70岁的用户数量是: {count}");
        })
    });
}

#[allow(unused)]
pub fn bench_query_map(c: &mut Criterion) {
    c.bench_function("select_10k_rows_query_map", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_data(&conn); 
            
            let mut stmt = conn
                .prepare("SELECT id, name, age FROM users ORDER BY id")
                .unwrap();
            
            let rows = stmt.query_map([], |row| {
                // 注意：这里只是借用数据，没有额外拷贝
                Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)?))
            }).unwrap();

            // 必须消费迭代器，否则基准只测试了“创建迭代器”的开销
            let mut count = 0;
            for row in rows {
                let _data = row.unwrap();
                count += 1;
            }
            assert_eq!(count, 10000);
        })
    });
}

#[allow(unused)]
pub fn bench_query_and_then(c: &mut Criterion) {
    c.bench_function("select_10k_rows_query_and_then", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_data(&conn);

            let mut stmt = conn
                .prepare("SELECT id, name, age FROM users where age BETWEEN 20 AND 60 ORDER BY id")
                .unwrap();

            // query_and_then 的闭包返回 Result，适合需要校验数据的场景
            let rows = stmt.query_and_then([], |row| {
                // 假设这里可以做复杂转换，例如将 String 转为自定义类型，可能失败
                Ok::<(i32, String, i32), rusqlite::Error>((row.get(0)?, row.get(1)?, row.get(2)?))
            }).unwrap();

            let mut count = 0;
            for row in rows {
                let _data = row.unwrap();
                count += 1;
            }
            assert_eq!(count, 10000);
        })
    });
}

fn setup_join_data(conn: &Connection) {
    conn.execute("DROP TABLE IF EXISTS orders", []).unwrap();
    conn.execute("DROP TABLE IF EXISTS users", []).unwrap();
    
    // 建 users 表
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        [],
    ).unwrap();
    
    // 建 orders 表，关联 user_id
    conn.execute(
        "CREATE TABLE orders (
            id INTEGER PRIMARY KEY, 
            user_id INTEGER, 
            amount REAL
        )",
        [],
    ).unwrap();

    // 插入 1000 个用户
    for i in 0..1000 {
        conn.execute("INSERT INTO users (name) VALUES (?1)", params![format!("user_{}", i)]).unwrap();
    }

    // 插入 10000 个订单（随机分配给用户 0~999）
    let mut stmt = conn.prepare("INSERT INTO orders (user_id, amount) VALUES (?1, ?2)").unwrap();
    for i in 0..10000 {
        let user_id = i % 1000; // 每个用户平均 10 个订单
        let amount = (i as f64) * 0.5;
        stmt.execute(params![user_id, amount]).unwrap();
    }
}

fn run_join(conn: &Connection) {
    let mut stmt = conn
        .prepare("SELECT users.name, SUM(orders.amount) FROM users INNER JOIN orders ON users.id = orders.user_id GROUP BY users.id")
        .unwrap();
    let rows = stmt.query_map([], |_row| Ok(())).unwrap();
    for row in rows { let _ = row.unwrap(); } // 消费迭代器，触发实际执行
}

#[allow(unused)]
pub fn bench_join_no_index(c: &mut Criterion) {
    c.bench_function("join_no_index", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_join_data(&conn); // 无索引
            run_join(&conn);
        })
    });
}

#[allow(unused)]
pub fn bench_join_with_index(c: &mut Criterion) {
    c.bench_function("join_with_index", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_join_data(&conn);
            conn.execute("CREATE INDEX idx_orders_user_id ON orders(user_id)", []).unwrap();
            run_join(&conn);
        })
    });
}

#[allow(unused)]
pub fn bench_cte(c: &mut Criterion) { //CTE递归
    c.bench_function("cte", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            setup_join_data(&conn);
            let _: i32 = conn.query_row(
                "WITH t AS (SELECT user_id FROM orders GROUP BY user_id HAVING SUM(amount) > 100)
                 SELECT COUNT(*) FROM users WHERE id IN (SELECT user_id FROM t)",
                [],
                |row| row.get(0),
            ).unwrap();
        })
    });
}