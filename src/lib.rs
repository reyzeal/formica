use std::collections::{BTreeMap, HashMap};

use async_std::{io, task};
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::stream::StreamExt;
use async_std::sync::{Arc, Mutex};

pub trait HttpHandler: FnMut(httparse::Request, Response) -> Response{}


impl<T> HttpHandler for T where T: FnMut(httparse::Request, Response) -> Response{}

type RouteHandler = Arc<Mutex<dyn HttpHandler<Output=Response> + Send>>;
type RouteMaps = BTreeMap<String, RouteHandler>;

pub struct Formica{
    addr: String,
    routes: RouteMaps
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
async fn on_connection(routes: BTreeMap<String, RouteHandler>, mut stream: TcpStream) -> io::Result<()> {
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
                            match routes.get(&(method.to_string()+path)) {
                                Some(handler) => {
                                    let handler = handler.clone();
                                    let mut handler = handler.lock().await;
                                    let response: Response = handler(req, Response::new());
                                    stream.write(response.compile().as_bytes()).await?;
                                }
                                _ => {
                                    stream.write(b"HTTP/1.1 404 Not Found\r\nContent-Length: 18\r\n\r\nResource Not Found").await?;
                                }
                            }
                        }
                    }
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
        Self {
            addr: addr.to_string(),
            routes: BTreeMap::new()
        }
    }
    pub async fn listen(&mut self) -> io::Result<()> {
        let address = &self.addr;
        let tcp = TcpListener::bind(address).await.unwrap();
        let mut incoming = tcp.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            task::spawn(on_connection(self.routes.clone(), stream));
        }
        Ok(())
    }
    pub fn post(&mut self, path: &str, callback: fn(httparse::Request, Response) -> Response) -> &mut Self{
        let handler = Box::new(callback);
        let key = ("POST".to_string()+path);
        match &self.routes.get(&key){
            Some(handler) => {

            },
            None => {
                &self.routes.insert(key, Arc::new(Mutex::new(handler)));
            }
        }
        self
    }
    pub fn get(&mut self, path: &str, callback: fn(httparse::Request, Response) -> Response) -> &mut Self{
        let handler = Box::new(callback);
        let key = ("GET".to_string()+path);
        match &self.routes.get(&key){
            Some(handler) => {

            },
            None => {
                &self.routes.insert(key, Arc::new(Mutex::new(handler)));
            }
        }
        self
    }
}
