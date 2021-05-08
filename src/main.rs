use std::{collections::HashMap, error::Error, sync::Mutex};
use harsh::Harsh;
use actix_web::{App, HttpResponse, HttpServer, Responder, get, http::header, web};

#[derive(Clone)]
struct ShortenerService {
    encoder: Harsh,
    entries: HashMap<u64, String>,
    next_index: u64,
}

impl ShortenerService {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            encoder: Harsh::builder().salt("hello world").build().unwrap(),
            entries: HashMap::new(),
            next_index: 0,
        })
    }

    pub fn add(&mut self, url: &str) -> String {
        let index = self.next_index;
        self.next_index += 1;
        
        let encoded = self.encoder.encode(&[index]);
        self.entries.insert(index, format!("https://{}", url));

        println!("{:?}", self.entries);

        encoded
    }

    pub fn lookup(&self, id: &str) -> Option<String> {
        let index_result = self.encoder.decode(id);
        match index_result {
            Ok(index) => {
                self.entries.get(&index[0]).map(|e| e.to_string())
            }
            _ => None
        }
    }
}

struct AppState {
    shortener: Mutex<ShortenerService>,
}


#[get("/encode/{request_url}")]
async fn add(web::Path(request_url): web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let mut shortener = data.shortener.lock().unwrap();
    let encoded = shortener.add(&request_url);

    HttpResponse::Ok().body(format!("{}", encoded))
}

#[get("/{request_hash}")]
async fn find(web::Path(request_hash): web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let shortener = data.shortener.lock().unwrap();
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
    let shortener = ShortenerService::new().unwrap();
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
