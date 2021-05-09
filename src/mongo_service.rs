// use mongodb::{bson::doc, sync::{Client, Collection}};
use mongodb::{Client, Collection, Database, bson::doc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ShortUrl {
    index: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Status {
    last_index: i64,
}

pub struct MongoService {
    client: Client,
}

impl MongoService {
    pub async fn new(mongo_uri: &str) -> Self {
        let client = Client::with_uri_str(mongo_uri).await.unwrap();

        MongoService {
            client,
        }
    }

    fn get_database(&self) -> Database {
        self.client.database("urls")
    }

    async fn get_next_index(&self) -> i64 {
        let status_collection = self.get_database().collection_with_type::<Status>("status");
        let next_index = status_collection.find_one_and_update(doc!{}, doc!{"$inc": { "last_index": 1 }}, None).await.unwrap();
        if let Some(status) = next_index {
            status.last_index
        } else {
            0
        }
    }

    pub async fn add_url(&self, url: &str) -> i64 {
        let url_collection = self.get_database().collection_with_type::<ShortUrl>("urls");

        let index = self.get_next_index().await;
        url_collection.insert_one(ShortUrl {
            index,
            url: url.to_string(),
        }, None).await.unwrap();

        index
    }

    pub async fn lookup_url(&self, index: i64) -> Option<String> {
        let url_collection = self.get_database().collection_with_type::<ShortUrl>("urls");

        let url = url_collection.find_one(doc! {"index": index}, None).await.unwrap();
        url.map(|u| u.url)
    }
}
