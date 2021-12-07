use std::collections::HashMap;

use async_trait::async_trait;
use dataload_rs::{BatchFunction, Loader};
use futures::future;

#[derive(Debug, PartialEq, Eq, Clone)]
struct DummyData(String);

struct DummyContext {
    map: HashMap<i64, String>,
}

struct DummyDataLoader;

#[async_trait]
impl BatchFunction<i64, DummyData> for DummyDataLoader {
    type Context = DummyContext;
    async fn load(keys: &[i64], context: &DummyContext) -> Vec<(i64, DummyData)> {
        keys.iter()
            .filter_map(|k| context.map.get(k).cloned().map(|v| (*k, DummyData(v))))
            .collect::<Vec<_>>()
    }
}

#[tokio::test]
async fn basic_load() {
    let mut context = DummyContext { map: HashMap::new() };
    context.map.insert(42, "Foo".to_owned());

    let loader = Loader::new(DummyDataLoader {}, context);
    assert_eq!(loader.load(42).await, Some(DummyData("Foo".to_owned())));
}

#[tokio::test]
async fn repeated_load() {
    let mut context = DummyContext { map: HashMap::new() };
    context.map.insert(42, "Foo".to_owned());

    let loader = Loader::new(DummyDataLoader {}, context);
    assert_eq!(loader.load(42).await, Some(DummyData("Foo".to_owned())));
    assert_eq!(loader.load(42).await, Some(DummyData("Foo".to_owned())));
}

#[tokio::test]
async fn basic_load_many() {
    let mut context = DummyContext { map: HashMap::new() };
    context.map.insert(42, "one fish".to_owned());
    context.map.insert(12, "two fish".to_owned());
    context.map.insert(5, "red fish".to_owned());
    context.map.insert(8, "blue fish".to_owned());

    let loader = Loader::new(DummyDataLoader {}, context);
    assert_eq!(
        loader.load_many(vec![5, 12, 8]).await,
        vec![
            Some(DummyData("red fish".to_owned())),
            Some(DummyData("two fish".to_owned())),
            Some(DummyData("blue fish".to_owned()))
        ]
    );
}

#[tokio::test]
async fn load_async() {
    let mut context = DummyContext { map: HashMap::new() };
    context.map.insert(42, "one fish".to_owned());
    context.map.insert(12, "two fish".to_owned());
    context.map.insert(5, "red fish".to_owned());
    context.map.insert(8, "blue fish".to_owned());

    let loader = Loader::new(DummyDataLoader {}, context);

    let tuple = future::join4(
        loader.load(5),
        loader.load_many(vec![5, 42]),
        loader.load(99),
        loader.load(12),
    );

    assert_eq!(
        tuple.await,
        (
            Some(DummyData("red fish".to_owned())),
            vec![Some(DummyData("red fish".to_owned())), Some(DummyData("one fish".to_owned())),],
            None,
            Some(DummyData("two fish".to_owned()))
        )
    );
}
