use std::collections::{BTreeMap, HashMap};

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
pub struct Request<'header, 'buf>{
    pub method: &'buf str,
    pub pathname:  &'buf str,
    pub query: HashMap<String,String>,
    pub content: &'buf [u8],
    pub headers: HashMap<&'header str, &'buf [u8]>
}

impl<'header, 'buf> Drop for Request<'header, 'buf> {
    fn drop(&mut self) {

    }
}

impl<'headers, 'buf> From<&httparse::Request<'headers, 'buf>> for Request<'headers, 'buf>  {
    fn from(value: &httparse::Request<'headers, 'buf>) -> Request<'headers, 'buf> {
        let headers_vec = value.headers.to_vec();
        let mut headers: HashMap<&str, &[u8]> = HashMap::new();
        for i in headers_vec.iter() {
            headers.insert(i.name, i.value);
        }
        Request {
            method: value.method.unwrap(),
            pathname: value.path.unwrap(),
            query: Default::default(),
            content: &[],
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
    const MAX_BUFFER: usize = 20480;
    let mut raw = vec![0u8; 0];
    let mut buf = [0u8; MAX_BUFFER];
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    while let Ok(len) = stream.read(&mut buf).await {
        if len < MAX_BUFFER {
            raw.append(&mut buf[..len].to_vec());
            break
        }else{
            raw.append(&mut buf.to_vec())
        }
    }

    let result = req.parse(raw.as_ref()).unwrap();
    if result.is_complete(){
        match req.path {
            Some(path) => {
                match req.method {
                    None => {}
                    Some(method) => {
                        let routes = routes.clone();
                        let routes = routes.lock().await;
                        match routes.get(&(method.to_string()+path)) {
                            Some(handler) => {
                                let handler = handler.clone();
                                let mut handler = handler.lock().await;
                                let mut request = Request::from(&req);
                                // if method != "GET" {
                                //     let mut content_length = 0;
                                //     for header in req.headers.iter() {
                                //         if "Content-Length" == header.name {
                                //             match String::from_utf8_lossy(header.value).to_string().parse::<usize>() {
                                //                 Ok(len) => {
                                //                     content_length = len;
                                //                     Some(1);
                                //                 },
                                //                 Err(_) => {}
                                //             }
                                //             break;
                                //         }
                                //     }
                                //     let slice = raw.len() - content_length;
                                //     if slice < raw.len() && slice > 0 {
                                //         request.content = raw[raw.len() - content_length..].as_ref();
                                //     }
                                //
                                // }

                                let response: Response = handler(request, Response::new());
                                stream.write_all(response.compile().as_bytes()).await?;
                                stream.flush().await?;
                                drop(req);
                                drop(raw);
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
    drop(routes);
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
            println!("worker {}", c);
            task::spawn(on_connection(Arc::clone(&cloned[c]), stream));
            c = c+1;
            if c >= 10 {
                c = c%10;
            }
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
