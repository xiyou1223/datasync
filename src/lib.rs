pub mod args;
pub mod db;
pub mod demo;
pub mod handle;
pub mod model;
pub mod util;

#[cfg(test)]
mod test_hello {
    #[test]
    fn say_hello() {
        println!("Hello, world!");
    }
}
