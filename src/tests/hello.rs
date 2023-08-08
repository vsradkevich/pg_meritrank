#[cfg(any(test, feature = "pg_test"))]
#[crate::pg_schema]
mod tests {
    use crate::pg_test;
    use pgx::prelude::*;

    #[pg_test]
    fn hello_world() {
        let result = crate::hello_hello_world();
        assert_eq!("Hello, hello_world", result);
        println!("Test hello_hello_world passed.");
    }

    #[pg_test]
    fn hello_world_spi() {
        let result: String = Spi::get_one("SELECT hello_hello_world();")
            .unwrap()
            .unwrap_or_default();
        assert_eq!("Hello, hello_world", result);
        println!("Test hello_hello_world_spi passed.");
    }
}
