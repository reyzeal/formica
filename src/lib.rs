use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;

use async_std::{io, task};
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::stream::StreamExt;
use async_std::sync::{Arc, Mutex};

pub trait HttpHandler: FnMut(httparse::Request, Response) -> Response{}


impl<T> HttpHandler for T where T: FnMut(httparse::Request, Response) -> Response{}

type RouteHandler = Mutex<Box<dyn HttpHandler<Output=Response> + Send>>;
type RouteMaps = BTreeMap<String, Arc<Mutex<HashMap<String, RouteHandler>>>>;

pub struct Formica{
    addr: String,
    routes: BTreeMap<String, Arc<Mutex<HashMap<String, RouteHandler>>>>
    // main_thread: thread::JoinHandle<()>
}
#[derive(Debug)]
pub struct Request{
    pub method: String,
    pub pathname: String,
    pub query: HashMap<String,String>,
    pub content: String,
    pub headers: HashMap<String, String>
}

pub struct Response{
    code: i32,
    headers: HashMap<String,String>,
    content: String
}
impl Response{
    pub fn new() -> Response {
        Response {
            code: 200,
            headers: Default::default(),
            content: String::new()
        }
    }
    pub fn status(&mut self, code: i32) -> &mut Self{
        self.code = code;
        self
    }
    pub fn set_header(&mut self, key: &str, value: &str) -> &mut Self{
        self.headers.insert(key.to_string(), value.to_string());
        self
    }
    pub fn body(&mut self, data: String) -> &mut Self{
        self.content = data;
        self
    }
    pub fn compile(&self) -> String {
        let mut headers = String::new();
        for i in self.headers.iter() {
            headers.push_str(format!("{}: {}\r\n", i.0,i.1).as_str())
        }
        headers.push_str(format!("Content-Length: {}", self.content.len()).as_str());

        format!("HTTP/1.1 {} {}\r\n{}\r\n\r\n{}", self.code, "OK", headers, self.content)
    }
}
async fn on_connection(routes: RouteMaps, mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0u8; 512];
    loop {

        let len = stream.read(&mut buffer).await?;
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);
        let result = req.parse(&buffer[..len]).unwrap();
        if result.is_complete() {
            match req.path {
                Some(path) => {
                    match req.method {
                        None => {}
                        Some(method) => {
                            match routes.get(path) {
                                Some(route) => {
                                    let route = route.clone();
                                    let route = route.lock().await;
                                    match route.get(method) {
                                        None => {
                                            stream.write(b"HTTP/1.1 404 Not Found\r\nContent-Length: 18\r\n\r\nResource Not Found").await?;
                                        }
                                        Some(callback) => {
                                            let mut handler = callback;
                                            let mut callback = handler.lock().await;
                                            let response: Response = callback(req, Response::new());
                                            stream.write(response.compile().as_bytes()).await?;
                                        }
                                    }
                                }
                                _ => {
                                    stream.write(b"HTTP/1.1 404 Not Found\r\nContent-Length: 18\r\n\r\nResource Not Found").await?;
                                }
                            }
                        }
                    }
                    // Some(req) => {
                    //     stream.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK").await?;

                    // },
                    // None => {
                    //     stream.write(b"HTTP/1.1 404 Not Found\r\nContent-Length: 18\r\n\r\nResource Not Found").await?;
                    // }
                    // check router for path.
                    // /404 doesn't exist? we could stop parsing
                },
                None => {
                    // must read more and parse again
                }
            }
        }

    }
}

impl Formica {
    pub fn new (addr: &str) -> Self{
        println!("create new");
        Self {
            addr: addr.to_string(),
            routes: BTreeMap::new()
        }
    }
    pub async fn listen(&mut self) -> io::Result<()> {
        println!("Listening to {}", self.addr);
        let address = &self.addr;
        let tcp = TcpListener::bind(address).await.unwrap();
        let mut incoming = tcp.incoming();

        while let Some(stream) = incoming.next().await {
            println!("incoming");
            let stream = stream?;
            task::spawn(on_connection(self.routes.clone(), stream));
        }
        Ok(())
    }
    pub async fn post(&mut self, path: &str, callback: fn(httparse::Request, Response) -> Response) -> &mut Self{
        let mut routes = &mut self.routes;
        let handler = Box::new(callback);
        match routes.get(&path.to_string()){
            Some(route) => {
                let route = route.clone();
                let mut hashmap = route.lock().await;
                hashmap.insert("POST".to_string(), Mutex::new(handler));
            },
            None => {
                let mut hashmap: HashMap<String, RouteHandler> = HashMap::new();
                hashmap.insert("POST".to_string(), Mutex::new(handler));
                routes.insert(path.to_string(), Arc::new(Mutex::new(hashmap)));
            }
        }
        self
    }
    pub async fn get(&mut self, path: &str, callback: fn(httparse::Request, Response) -> Response) -> &mut Self{
        let mut routes = &mut self.routes;
        let handler = Box::new(callback);
        match routes.get(&path.to_string()){
            Some(route) => {
                let route = route.clone();
                let mut hashmap = route.lock().await;
                hashmap.insert("GET".to_string(), Mutex::new(handler));
            },
            None => {
                let mut hashmap: HashMap<String, RouteHandler> = HashMap::new();
                hashmap.insert("GET".to_string(), Mutex::new(handler));
                routes.insert(path.to_string(), Arc::new(Mutex::new(hashmap)));
            }
        }
        self
    }
}
