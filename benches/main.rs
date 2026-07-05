use criterion::{criterion_group, criterion_main};

// mod test_bench;
// mod query_bench;
// mod update_bench;
// mod type_mapping_bench;
// mod transaction_bench;
// mod conflict_bench;
// mod window_bench;
// mod recursive_bench;
mod udaf_bench;
mod wal_bench;
mod pragma_bench;

criterion_group! {
    name = benches;
    config = criterion::Criterion::default()
        .sample_size(1000)
        .measurement_time(std::time::Duration::from_secs(10));
    targets =
        // test_bench::bench_insert_row_by_row,
        // test_bench::bench_insert_row_by_row_tx,
        // test_bench::bench_insert_multi_values,
        // query_bench::bench_select_no_index,
        // query_bench::bench_select_with_index,
        // query_bench::bench_query_map,
        // query_bench::bench_query_and_then,
        // update_bench::bench_update_auto_commit,
        // update_bench::bench_update_transaction,
        // update_bench::bench_delete_auto_commit,
        // update_bench::bench_delete_transaction,
        // update_bench::bench_delete_order_by_limit,
        // query_bench::bench_join_no_index,
        // query_bench::bench_join_with_index,
        // query_bench::bench_cte,
        // type_mapping_bench::bench_raw_i32,
        // type_mapping_bench::bench_enum_mapping,
        // type_mapping_bench::bench_json_mapping,
        // transaction_bench::bench_auto_commit,
        // transaction_bench::bench_explicit_tx,
        // transaction_bench::bench_with_savepoint,
        // transaction_bench::bench_insert_with_check,
        // transaction_bench::bench_insert_without_check,
        // transaction_bench::bench_journal_mode,
        // conflict_bench::bench_ignore,
        // conflict_bench::bench_upsert,
        // conflict_bench::bench_replace,
        // window_bench::bench_window,
        // recursive_bench::bench_recursive,
        // udaf_bench::bench_avg_native,
        // udaf_bench::bench_median_udaf,
        // wal_bench::bench_write_throughput,
        // wal_bench::bench_write_disk_throughput,
        // wal_bench::bench_transaction_commit,
        // wal_bench::bench_concurrent_readers,
        wal_bench::bench_checkpoint_overhead,
        pragma_bench::bench_cache_sizes,
}

criterion_main!(benches);