use std::{collections::HashMap, error::Error, sync::Mutex};
use harsh::Harsh;
use serde::Deserialize;
use actix_web::{App, HttpResponse, HttpServer, Responder, get, http::header, post, web};
use url_short::mongo_service::MongoService;

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

    pub async fn add(&self, url: &str) -> String {
        let index = self.mongo_service.add_url(&format!("https://{}", url)).await;

        self.encoder.encode(&[index as u64])
    }

    pub async fn lookup(&self, id: &str) -> Option<String> {
        let index_result = self.encoder.decode(id);
        match index_result {
            Ok(index) => {
                let index = index[0] as i64;
                    let lookup = self.mongo_service.lookup_url(index).await;
                    lookup
                // }
            }
            _ => None
        }
    }
}

struct AppState {
    shortener: ShortenerService,
}


#[derive(Deserialize)]
struct AddUrlRequest {
    url: String,
}

#[post("/")]
async fn add(add_url: web::Json<AddUrlRequest>, data: web::Data<AppState>) -> impl Responder {
    let encoded = data.shortener.add(&add_url.url).await;

    HttpResponse::Ok().body(format!("{}", encoded))
}

#[get("/{request_hash}")]
async fn find(web::Path(request_hash): web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let decoded = data.shortener.lookup(&request_hash).await;

    if let Some(url) = decoded {
        HttpResponse::TemporaryRedirect().header(header::LOCATION, url).finish()
    } else {
        HttpResponse::NotFound().body(format!("Could not find URL for: {}", request_hash))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mongo_uri = std::env::args().nth(1).expect("Expected Mongo URI");

    let mongoo_service = MongoService::new(&mongo_uri).await;
    let shortener = ShortenerService::new(mongoo_service).unwrap();
    let app_state = web::Data::new(AppState {
        shortener,
    });

    println!("Starting on port 8080...");

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
