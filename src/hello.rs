use poem::{ handler, web::Path};



#[handler]
pub fn hello(Path(name): Path<String>) -> String {
    format!("hello: {}", name)
}

#[handler]
pub fn hello2(Path(name): Path<String>) -> String {
    format!("hello2: {}", name)
}