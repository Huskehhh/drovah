use mysql::prelude::Queryable;
use mysql::{Pool, PooledConn};

use crate::{BuildData, ProjectData};
pub struct MySQLConnection {
    pub pool: Pool,
}

impl MySQLConnection {
    /// Creates a new MySQLConnection, given a connection url
    pub fn new(url: &str) -> MySQLConnection {
        let pool = Pool::new(url).unwrap();
        MySQLConnection { pool }
    }

    /// Executes a query on db, doesn't return result
    pub async fn execute_update(&self, statement: &str) {
        let conn: &mut PooledConn = &mut self.pool.get_conn().unwrap();
        if let Err(why) = conn.query_drop(statement) {
            println!("{}", why);
        }
    }

    /// Queries and returns the first result as an i32
    async fn query_first_i32(&self, query: &str) -> i32 {
        let conn: &mut PooledConn = &mut self.pool.get_conn().unwrap();

        match conn.query_first(query) {
            Ok(result) => {
                return result.unwrap_or(1);
            }
            Err(why) => {
                eprintln!("Error in query_first! {}", why);
                return 1;
            }
        }
    }

    /// Queries and returns the first result as a String
    async fn query_first_str(&self, query: &str) -> String {
        let conn: &mut PooledConn = &mut self.pool.get_conn().unwrap();

        match conn.query_first(query) {
            Ok(result) => {
                return result.unwrap_or(String::from("failing"));
            }
            Err(why) => {
                eprintln!("Error in query_first! {}", why);
                return String::from("failing");
            }
        }
    }

    /// Gets the project id of given project
    pub async fn get_project_id(&self, project: &str) -> i32 {
        self.query_first_i32(&format!(
            "SELECT `project_id` FROM `projects` WHERE `project` = '{}';",
            project
        ))
        .await
    }

    /// Gets the project name of a given project id
    pub async fn get_project_name(&self, project_id: i32) -> String {
        self.query_first_str(&format!(
            "SELECT `project` FROM `projects` WHERE `project_id` = '{}';",
            project_id
        ))
        .await
    }

    /// Gets the latest build number of a given project
    pub async fn get_build_number(&self, project_id: i32) -> i32 {
        self.query_first_i32(&format!("SELECT * FROM `builds` WHERE `project_id` = '{}' ORDER BY `project_id` DESC LIMIT 0, 1;", project_id)).await
    }

    /// Retrieves the latest build status for a given project
    pub async fn get_latest_build_status(&self, project_id: i32) -> String {
        self.query_first_str(&format!(
            "SELECT `status` FROM `builds` WHERE project_id = '{}' AND `branch` = 'master' ORDER BY id DESC LIMIT 0, 1;",
            project_id
        ))
    .await
    }

    pub async fn get_status_for_build(&self, project_id: i32, build_number: i32) -> String {
        let query = format!(
            "SELECT `status` FROM `builds` WHERE `project_id` = '{}' AND `build_number` = '{}';",
            project_id, build_number
        );

        self.query_first_str(&query).await
    }

    pub async fn get_project_data(&self, project_id: i32) -> ProjectData {
        let build_query = format!(
            "SELECT `status`, `files`, `build_number` from `builds` WHERE `project_id` = '{}';",
            project_id
        );

        let project_name = self.get_project_name(project_id).await;

        let conn: &mut PooledConn = &mut self.pool.get_conn().unwrap();

        let result = conn
            .query_map(&build_query, |(status, files, build_number)| {
                let files: String = files;
                let split_files = files
                    .split_terminator(", ")
                    .map(|s| s.to_owned())
                    .collect::<Vec<String>>();
                BuildData {
                    build_number,
                    build_status: status,
                    archived_files: Some(split_files),
                }
            })
            .unwrap();

        ProjectData {
            project: project_name,
            builds: result,
        }
    }
}
