use poem::handler;



#[handler]
pub fn up() -> String {
    format!("{{ status: \"UP\" }}\n")
}
