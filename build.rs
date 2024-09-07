use std::fs::read_to_string;

fn main() {
    for line in read_to_string("prepare-sqlx.env").unwrap().lines() {
        println!("cargo:rustc-env={line}")
    }
}
