.PHONY: all clean install

all: build install
build:
	cargo build --release
	@echo "Build complete."
	@echo "To install, run 'make install'."
install:
	install -Dm755 ./target/release/owonero-rs /usr/local/bin/owonero
	@echo "Installation complete. You can run the application using 'owonero'."
clean:
	cargo clean
	@echo "Clean complete."