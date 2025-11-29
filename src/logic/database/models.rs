#[derive(Debug, Clone)]
pub struct MigrationVersion {
    pub id: i32,
    pub version: u16,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct File {
    pub id: i32,
    pub server_id: i32,
    pub name: String,
    pub added_date: i64,
    pub finish_date: Option<i64>,
}
