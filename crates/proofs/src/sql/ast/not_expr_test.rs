use super::{ColumnRef, EqualsExpr, FilterExpr, FilterResultExpr, NotExpr, TableExpr};
use crate::base::database::ColumnType;
use crate::base::database::{
    make_random_test_accessor, RandomTestAccessorDescriptor, TestAccessor,
};
use crate::base::scalar::IntoScalar;
use crate::sql::proof::QueryExpr;
use crate::sql::proof::{exercise_verification, VerifiableQueryResult};
use arrow::array::Int64Array;
use arrow::record_batch::RecordBatch;
use curve25519_dalek::scalar::Scalar;
use indexmap::IndexMap;
use polars::prelude::*;
use rand::{
    distributions::{Distribution, Uniform},
    rngs::StdRng,
};
use rand_core::SeedableRng;
use std::sync::Arc;

#[test]
fn we_can_prove_a_not_equals_query_with_a_single_selected_row() {
    let expr = FilterExpr::new(
        vec![FilterResultExpr::new(
            ColumnRef {
                column_name: "a".to_string(),
                table_name: "t".to_string(),
                namespace: None,
                column_type: ColumnType::BigInt,
            },
            "a".to_string(),
        )],
        TableExpr {
            name: "t".to_string(),
        },
        Box::new(NotExpr::new(Box::new(EqualsExpr::new(
            ColumnRef {
                column_name: "b".to_string(),
                table_name: "t".to_string(),
                namespace: None,
                column_type: ColumnType::BigInt,
            },
            Scalar::from(1u64),
        )))),
    );
    let mut accessor = TestAccessor::new();
    accessor.add_table(
        "t",
        &IndexMap::from([
            ("a".to_string(), vec![123, 456]),
            ("b".to_string(), vec![0, 1]),
        ]),
    );
    let res = VerifiableQueryResult::new(&expr, &accessor);

    exercise_verification(&res, &expr, &accessor);

    let res = res.verify(&expr, &accessor).unwrap().unwrap();
    let res_col: Vec<i64> = vec![123];
    let expected_res = RecordBatch::try_new(
        expr.get_result_schema(),
        vec![Arc::new(Int64Array::from(res_col))],
    )
    .unwrap();
    assert_eq!(res, expected_res);
}

#[test]
fn we_can_query_random_tables() {
    let descr = RandomTestAccessorDescriptor {
        min_rows: 1,
        max_rows: 20,
        min_value: -3,
        max_value: 3,
    };
    let mut rng = StdRng::from_seed([0u8; 32]);
    let cols = ["a", "b"];
    for _ in 0..10 {
        let accessor = make_random_test_accessor(&mut rng, "t", &cols, &descr);
        let val = Uniform::new(descr.min_value, descr.max_value + 1).sample(&mut rng);
        let expr = FilterExpr::new(
            vec![FilterResultExpr::new(
                ColumnRef {
                    column_name: "a".to_string(),
                    table_name: "t".to_string(),
                    namespace: None,
                    column_type: ColumnType::BigInt,
                },
                "a".to_string(),
            )],
            TableExpr {
                name: "t".to_string(),
            },
            Box::new(NotExpr::new(Box::new(EqualsExpr::new(
                ColumnRef {
                    column_name: "b".to_string(),
                    table_name: "t".to_string(),
                    namespace: None,
                    column_type: ColumnType::BigInt,
                },
                val.into_scalar(),
            )))),
        );
        let res = VerifiableQueryResult::new(&expr, &accessor);
        exercise_verification(&res, &expr, &accessor);
        let res = res.verify(&expr, &accessor).unwrap().unwrap();
        let expected = accessor.query_table("t", |df| {
            df.clone()
                .lazy()
                .filter(col("b").neq(val))
                .select([col("a")])
                .collect()
                .unwrap()
        });
        assert_eq!(res, expected);
    }
}