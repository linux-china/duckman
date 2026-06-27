build:
    cargo build --bins

release:
    cargo build --bins --release
    ls -al target/release/duckman
    ls -al target/release/duckman_shim
    cp target/release/duckman ~/.cargo/bin/duckman
    cp target/release/duckman_shim ~/.cargo/bin/duckman_shim

pipe:
    cat family.csv | ./target/debug/duckman run -- -c "select * from read_csv('/dev/stdin')"
