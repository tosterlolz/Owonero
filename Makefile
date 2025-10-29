.PHONY: all clean install build build-win

all: build build-win install

build: clean
	cargo build --release
	@echo "✅ Build (native) complete."
	@echo "To install, run 'make install'."

build-win: clean
ifeq ($(OS),Windows_NT)
	rustup target add x86_64-pc-windows-msvc
	cargo build --release --target x86_64-pc-windows-msvc
	@echo "✅ Windows build (MSVC) complete."
	@echo "Executable: target/x86_64-pc-windows-msvc/release/owonero.exe"
else
	rustup target add x86_64-pc-windows-gnu
	cargo build --release --target x86_64-pc-windows-gnu
	@echo "✅ Windows build (GNU) complete."
	@echo "Executable: target/x86_64-pc-windows-gnu/release/owonero.exe"
endif

install: build
ifeq ($(OS),Windows_NT)
	@echo "⚠️  Installation on Windows is not automated. Please copy the executable manually."
else
	sudo install -Dm755 ./target/release/owonero /usr/local/bin/owonero
	@echo "✅ Installation complete. You can run the application using 'owonero'."
endif

clean:
	cargo clean
	@echo "🧹 Clean complete."