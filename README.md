# Postgres Merit Rank

Postgres Merit Rank is an extension for PostgreSQL that provides functionality for calculating and ranking merits. This README provides instructions for testing the extension using `cargo pgx test` and installing it in a PostgreSQL database.

## Testing (old)

To run the tests for Postgres Merit Rank, follow these steps:

1. Make sure you have Rust and Cargo installed on your system. You can install them from the official Rust website: [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).

2. Clone the repository for Postgres Merit Rank:

   ```bash
   git clone https://github.com/vsradkevich/pg_meritrank.git
   ```

3. Navigate to the project directory:

   ```bash
   cd pg_meritrank
   ```

4. Run the tests using `cargo pgx test`:

   ```bash
   cargo pgx test
   ```

   This command will compile the extension, create a test PostgreSQL database, and run the test suite against the database.

## Testing (new)

### Preparation

Before running tests, make sure you have the required version of Rust and all dependencies installed:

```bash
rustup default nightly
```

If you have specific features such as `fetch_and_generate`, make sure you enable them before running tests.

### Run tests

To run the tests, run the following command:

```bash
cargo +nightly pgx test
```

If you want to run tests with a specific feature enabled, such as `fetch_and_generate`, use the following command:

```bash
cargo +nightly pgx test --features fetch_and_generate
```

### Interpreting results

After testing is complete, you will see the results in the console:

- `ok` - the test passed successfully.
- `failed` - test failed.
- `ignored` - the test was ignored.
- `measured` - the test execution time was measured (if applicable).
- `filtered out` - the test was excluded from execution.

Successful completion of all tests ensures that your code works as expected. If any tests fail, it is recommended that you review the test output and correct any issues before proceeding.

## Troubleshooting (new)

If you encounter problems while working with our project, please follow these steps:

### 1. Rust upgrade to nightly version

Make sure you have the latest version of Rust (nightly). To do this, run the following command:

```bash
rustup default nightly
```

### 2. Installing the global module cargo-pgx

Install the `cargo-pgx` module for Rust (nightly):

```bash
cargo install cargo-pgx --force
```

### 3. Initialization

Before working with `pgx`, initialize:

```bash
cargo +nightly pgx init
```

### 4. Run tests

To make sure everything is set up correctly, run the tests:

```bash
cargo +nightly pgx test
```

### 5. Plugin installation and testing

After installing the plugin in Postgres, you can test its functionality through SQL queries. Sample queries and their expected results can be found in the plugin documentation.

### 6. Feedback and problem reporting

If you have any problems or questions, please create a [new issue](https://github.com/vsradkevich/pg_meritrank/issues) on Github. We will try to answer your request as quickly as possible.

## Installation (new)

To install Postgres Merit Rank in a PostgreSQL database, follow these steps:

1. Make sure you have Rust and Cargo installed on your system. You can install them from the official Rust website: [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).

2. Clone the repository for Postgres Merit Rank:

   ```bash
   git clone https://github.com/vsradkevich/pg_meritrank.git
   ```

3. Navigate to the project directory:

   ```bash
   cd pg_meritrank
   ```

4. Build the extension using `cargo pgx build`:

   ```bash
   cargo +nightly pgx build
   ```

   This command will compile the extension and generate the necessary files for installation.

5. Install the extension in your PostgreSQL database using `cargo pgx install`:

   ```bash
   cargo +nightly pgx install
   ```

   This command will install the extension in the default PostgreSQL extension directory (`$PG_CONFIG/share/extension`) or the directory specified by the `PGX_DESTDIR` environment variable.

6. Connect to your PostgreSQL database using an SQL client.

7. Enable the Postgres Merit Rank extension in the database:

   ```sql
   CREATE EXTENSION pg_meritrank;
   ```

   This command will enable the extension, making its functions and features available in the database.

8. You can now use the Postgres Merit Rank functions in your PostgreSQL queries.

Please refer to the documentation or source code for further details on how to use the Postgres Merit Rank extension and its available functions.

Documentation
-------------

For detailed usage instructions and API reference, please refer to the [documentation](https://docs.rs/pg_meritrank).

Contributing
------------

Contributions are welcome! If you have any bug reports, feature requests, or suggestions, please open an issue on the [GitHub repository](https://github.com/vsradkevich/pg_meritrank). Pull requests are also encouraged.

License
-------

`pg_meritrank` is licensed under the MIT License. See the [LICENSE](https://github.com/vsradkevich/pg_meritrank/blob/main/LICENSE) file for more information.

Maintainer
----------

`pg_meritrank` is actively maintained by [Vladimir Radkevich](https://github.com/vsradkevich). Feel free to reach out if you have any questions or need assistance.

Enjoy using `pg_meritrank` for calculating and ranking merits in your PostgreSQL database!