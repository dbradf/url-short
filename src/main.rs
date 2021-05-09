use std::{collections::HashMap, error::Error, sync::Mutex};
use harsh::Harsh;
use mongodb::{bson::doc, sync::{Client, Collection}};
use serde::{Deserialize, Serialize};
use actix_web::{App, HttpResponse, HttpServer, Responder, get, http::header, post, web};


#[derive(Debug, Serialize, Deserialize)]
struct ShortUrl {
    index: i64,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Status {
    last_index: i64,
}

struct MongoService {
    url_collection: Collection<ShortUrl>,
    status_collection: Collection<Status>,
}

impl MongoService {
    pub async fn new(mongo_uri: &str) -> Self {
        let client = Client::with_uri_str(mongo_uri).unwrap();
        let database = client.database("urls");
        let url_collection = database.collection::<ShortUrl>("urls");
        let status_collection = database.collection::<Status>("status");

        MongoService {
            url_collection,
            status_collection,
        }
    }

    fn get_next_index(&self) -> i64 {
        let next_index = self.status_collection.find_one_and_update(doc!{}, doc!{"$inc": { "last_index": 1 }}, None).unwrap();
        if let Some(status) = next_index {
            status.last_index
        } else {
            0
        }
    }

    pub fn add_url(&self, url: &str) -> i64 {
        let index = self.get_next_index();
        self.url_collection.insert_one(ShortUrl {
            index,
            url: url.to_string(),
        }, None).unwrap();

        index
    }

    pub fn lookup_url(&self, index: i64) -> Option<String> {
        let url = self.url_collection.find_one(doc! {"index": index}, None).unwrap();
        url.map(|u| u.url)
    }
}

struct ShortenerService {
    encoder: Harsh,
    mongo_service: MongoService,
    cache: HashMap<i64, String>,
}

impl ShortenerService {
    pub fn new(mongo_service: MongoService) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            encoder: Harsh::builder().salt("hello world").build().unwrap(),
            mongo_service,
            cache: HashMap::new(),
        })
    }

    pub fn add(&mut self, url: &str) -> String {
        let index = self.mongo_service.add_url(&format!("https://{}", url));

        self.encoder.encode(&[index as u64])
    }

    pub fn lookup(&mut self, id: &str) -> Option<String> {
        let index_result = self.encoder.decode(id);
        match index_result {
            Ok(index) => {
                let index = index[0] as i64;
                if self.cache.contains_key(&index) {
                    self.cache.get(&index).map(|u| u.to_string())
                } else {
                    let lookup = self.mongo_service.lookup_url(index);
                    if let Some(url) = &lookup {
                        self.cache.insert(index, url.to_string());
                    }
                    lookup
                }
            }
            _ => None
        }
    }
}

struct AppState {
    shortener: Mutex<ShortenerService>,
}


#[derive(Deserialize)]
struct AddUrlRequest {
    url: String,
}

#[post("/")]
async fn add(add_url: web::Json<AddUrlRequest>, data: web::Data<AppState>) -> impl Responder {
    let mut shortener = data.shortener.lock().unwrap();
    let encoded = shortener.add(&add_url.url);

    HttpResponse::Ok().body(format!("{}", encoded))
}

#[get("/{request_hash}")]
async fn find(web::Path(request_hash): web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let mut shortener = data.shortener.lock().unwrap();
    let decoded = shortener.lookup(&request_hash);

    if let Some(url) = decoded {
        HttpResponse::TemporaryRedirect().header(header::LOCATION, url).finish()
    } else {
        HttpResponse::NotFound().body(format!("Could not find URL for: {}", request_hash))

    }
    // HttpResponse::Ok().body(decoded)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mongo_uri = std::env::args().nth(1).expect("Expected Mongo URI");

    let mongoo_service = MongoService::new(&mongo_uri).await;
    let shortener = ShortenerService::new(mongoo_service).unwrap();
    let app_state = web::Data::new(AppState {
        shortener: Mutex::new(shortener),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(add)
            .service(find)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
