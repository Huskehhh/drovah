table! {
    builds (build_id) {
        build_id -> Integer,
        project_id -> Integer,
        build_number -> Integer,
        branch -> Text,
        files -> Text,
        status -> Text,
    }
}

table! {
    projects (project_id) {
        project_id -> Integer,
        project_name -> Text,
    }
}

allow_tables_to_appear_in_same_query!(builds, projects,);
