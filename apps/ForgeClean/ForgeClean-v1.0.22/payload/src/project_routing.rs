pub struct ProjectRouter { version: String }
impl ProjectRouter {
    pub fn new(v: &str) -> Self { ProjectRouter { version: v.to_string() } }
    pub fn run_migration(&self) -> Result<Vec<String>, Vec<String>> { Ok(vec![]) }
}
