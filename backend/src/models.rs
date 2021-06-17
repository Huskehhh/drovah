#[derive(Queryable)]
pub struct Build {
    pub build_id: i32,
    pub project_id: i32,
    pub build_number: i32,
    pub branch: String,
    pub files: String,
    pub status: String,
}
#[derive(Queryable)]
pub struct Project {
    pub project_id: i32,
    pub project_name: String,
}
