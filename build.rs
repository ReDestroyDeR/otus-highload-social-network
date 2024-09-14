use std::fs::read_to_string;

fn main() {
    if let Ok(env_file) = read_to_string("prepare-sqlx.env") {
        for line in env_file.lines() {
            println!("cargo:rustc-env={line}")
        }
    }
}
