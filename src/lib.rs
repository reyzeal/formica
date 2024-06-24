use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use async_std::{io, task};
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::stream::StreamExt;
use async_std::sync::{Arc, Mutex};

pub trait HttpHandler: FnMut(Request, Response) -> Response{}
impl<T> HttpHandler for T where T: FnMut(Request, Response) -> Response{}
type RouteHandler = Arc<Mutex<dyn HttpHandler<Output=Response> + Send>>;
type RouteMaps = BTreeMap<String, RouteHandler>;
pub struct Formica{
    addr: String,
    routes: RouteMaps
}

#[derive(Debug)]
pub struct Request{
    pub method: String,
    pub pathname:  String,
    pub query: HashMap<String,String>,
    pub content: Vec<u8>,
    pub headers: HashMap<String, String>
}

impl From<&httparse::Request<'_, '_>> for Request {
    fn from(value: &httparse::Request) -> Request {
        let headers_vec = value.headers.to_vec();
        let mut headers: HashMap<String, String> = HashMap::new();
        for i in headers_vec.iter() {
            headers.insert(i.name.to_string(), String::from_utf8_lossy(i.value).to_string());
        }
        Request {
            method: value.method.unwrap().to_string(),
            pathname: value.path.unwrap().to_string(),
            query: Default::default(),
            content: vec![],
            headers,
        }
    }
}

pub struct Response{
    code: i32,
    headers: HashMap<String,String>,
    content: String
}
impl<'server> Response{
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
async fn on_connection(routes: Arc<Mutex<BTreeMap<String, RouteHandler>>>, mut stream: TcpStream) -> io::Result<()> {
    // let mut reader = BufReader::new(&stream);
    const MAX_BUFFER: usize = 1024;
    let mut buf = [0u8; MAX_BUFFER];


    let mut key = String::new();
    loop {
        let size = stream.read(&mut buf).await.or_else(|e| {
            Err(false)
        });
        match size {
            Ok(size) => {
                let mut headers = [httparse::EMPTY_HEADER; 16];
                let mut req = httparse::Request::new(&mut headers);
                let result = req.parse(&buf[..size]).unwrap().is_complete();
                if result {
                    match req.path {
                        Some(path) => {
                            match req.method {
                                None => {
                                    stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await?;
                                    stream.flush().await?;
                                }
                                Some(method) => {
                                    let mut request = Request::from(&req);
                                    key = format!("{}", method.to_string()+path).to_owned();
                                    let routes = routes.clone();
                                    let routes = routes.lock().await;
                                    match routes.get(&key) {
                                        Some(handler) => {
                                            let handler = handler.clone();
                                            let mut handler = handler.lock().await;
                                            if method != "GET" {
                                                let mut content_length = 0;
                                                for header in req.headers.iter() {
                                                    if "Content-Length" == header.name {
                                                        match String::from_utf8_lossy(header.value).to_string().parse::<usize>() {
                                                            Ok(len) => {
                                                                content_length = len;
                                                                Some(1);
                                                            },
                                                            Err(_) => {}
                                                        }
                                                        break;
                                                    }
                                                }
                                                if size == MAX_BUFFER {
                                                    let mut raw : Vec<u8> = Vec::from(&buf[..size]);
                                                    let mut buf = [0u8; MAX_BUFFER];
                                                    loop {
                                                        println!("black hole");
                                                        let Ok(size) = stream.read(&mut buf).await else { break };
                                                        if size < MAX_BUFFER {
                                                            raw.extend(&buf[0..size]);
                                                            break;
                                                        }
                                                        raw.extend(&buf);
                                                        println!("kesekip");
                                                    }
                                                    println!("weh");

                                                    let slice = raw.len() - content_length;

                                                    if slice < raw.len() && slice > 0 {
                                                        request.content = raw[raw.len() - content_length..].to_vec();
                                                    }
                                                }

                                            }
                                            let response: Response = handler(request, Response::new());
                                            stream.write_all(response.compile().as_bytes()).await?;
                                            stream.flush().await?;
                                            drop(routes);
                                        }
                                        _ => {
                                            stream.write(b"HTTP/1.1 404 Not Found\r\nContent-Length: 18\r\n\r\nResource Not Found").await?;
                                        }
                                    }
                                    // break;
                                }
                            }
                        },
                        None => {
                            // must read more and parse again
                        }
                    }
                }
            }
            Err(_) => {
                break
            }
        }


    };



    Ok(())
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
        let mut cloned: Vec<Arc<Mutex<BTreeMap<String, RouteHandler>>>>= vec![];
        for i in 0..10 {
            cloned.push(Arc::new(Mutex::new(self.routes.clone())));
        }
        let mut c = 0;
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            task::spawn(on_connection(Arc::clone(&cloned[c]), stream));
            c = (c+1) % 10;
        }
        Ok(())
    }
    pub fn post(&mut self, path: &str, callback: fn(Request, Response) -> Response) -> &mut Self{
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
    pub fn get(&mut self, path: &str, callback: fn(Request, Response) -> Response) -> &mut Self{
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
