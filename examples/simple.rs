use std::collections::HashMap;

use async_trait::async_trait;
use dataload_rs::{BatchFunction, Loader};

// Empty functor that implements the BatchFunction trait. For this example, it
// trivially loads values from some HashMap.
struct MyBatchFn;

#[async_trait]
impl BatchFunction<i64, String> for MyBatchFn {
    type Context = HashMap<i64, String>;

    async fn load(keys: &[i64], context: &Self::Context) -> Vec<(i64, String)> {
        keys.into_iter().filter_map(|k| context.get(k).cloned().map(|v| (*k, v))).collect()
    }
}

#[tokio::main]
async fn main() {
    let mut context = HashMap::new();
    context.insert(2001, "a space odyssey".to_owned());
    context.insert(7, "samurai".to_owned());
    context.insert(12, "angry men".to_owned());

    let loader = Loader::new(MyBatchFn {}, context);

    assert_eq!(loader.load(7).await.as_deref(), Some("samurai"));
    assert_eq!(loader.load(15).await, None);

    assert_eq!(
        loader
            .load_many(vec![12, 2010, 2001])
            .await
            .iter()
            .map(Option::as_deref)
            .collect::<Vec<_>>(),
        vec![Some("angry men"), None, Some("a space odyssey")]
    );
}
