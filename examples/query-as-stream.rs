use chrono::{DateTime, Utc};
use firestore::*;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

pub fn config_env_var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|e| format!("{}: {}", name, e))
}

// Example structure to play with
#[derive(Debug, Clone, Deserialize, Serialize)]
struct MyTestStructure {
    some_id: String,
    some_string: String,
    one_more_string: String,
    some_num: u64,
    created_at: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Logging with debug enabled
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter("firestore=debug")
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Create an instance
    let db = FirestoreDb::new(&config_env_var("PROJECT_ID")?).await?;

    const TEST_COLLECTION_NAME: &'static str = "test";

    println!("Populating a test collection");
    for i in 0..10 {
        let my_struct = MyTestStructure {
            some_id: format!("test-{}", i),
            some_string: "Test".to_string(),
            one_more_string: "Test2".to_string(),
            some_num: 42,
            created_at: Utc::now(),
        };

        // Remove if it already exist
        db.delete_by_id(TEST_COLLECTION_NAME, &my_struct.some_id)
            .await?;

        // Let's insert some data
        db.create_obj(TEST_COLLECTION_NAME, &my_struct.some_id, &my_struct)
            .await?;
    }

    println!("Querying a test collection as a stream");
    // Query as a stream our data
    let mut object_stream: BoxStream<MyTestStructure> = db
        .stream_query_obj(
            FirestoreQueryParams::new(TEST_COLLECTION_NAME.into()).with_filter(
                FirestoreQueryFilter::Compare(Some(FirestoreQueryFilterCompare::Equal(
                    path!(MyTestStructure::some_num),
                    42.into(),
                ))),
            ),
        )
        .await?;

    while let Some(object) = object_stream.next().await {
        println!("Object in stream: {:?}", object);
    }

    // Querying as a stream with errors when needed
    let object_stream_with_errors = Box::pin(
        db.stream_query_obj_with_errors::<MyTestStructure>(
            FirestoreQueryParams::new(TEST_COLLECTION_NAME.into()).with_filter(
                FirestoreQueryFilter::Compare(Some(FirestoreQueryFilterCompare::Equal(
                    path!(MyTestStructure::some_num),
                    42.into(),
                ))),
            ),
        )
        .await?
        .filter_map(|item| {
            std::future::ready(match item {
                Ok(item) => Some(item),
                Err(err) => {
                    println!("Err occurred: {}", err);
                    None
                }
            })
        }),
    );

    let as_vec: Vec<MyTestStructure> = object_stream_with_errors.collect().await;
    println!("{:?}", as_vec);

    Ok(())
}
