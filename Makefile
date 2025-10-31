.PHONY: all clean install build build-win check

# Główne zadanie — zbuduj i zainstaluj
all: build build-win install

check:
	cargo fmt -- --check
	@echo "✅ All tasks complete."

# 🔧 Build natywny
build:
	cargo build --release
	@echo "✅ Build (native) complete."
	@echo "To install, run 'make install'."

# 🪟 Build pod Windows (MSVC lub GNU)
build-win:
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

# 🧩 Instalacja binarki + completions
install: build
ifeq ($(OS),Windows_NT)
	@echo "⚠️  Installation on Windows is not automated. Please copy the executable manually."
else
	# Install binary
	sudo install -Dm755 target/release/owonero /usr/local/bin/owonero
	@echo "✅ Binary installed to /usr/local/bin/owonero"

	# Install completions for common shells
	@if [ -n "$$SUDO_USER" ]; then \
		USER_TO_RUN="$$SUDO_USER"; \
	else \
		USER_TO_RUN="$$USER"; \
	fi; \
	echo "→ Installing completions for user: $$USER_TO_RUN"; \
	sudo -u $$USER_TO_RUN /usr/local/bin/owonero --install-completions bash || true; \
	sudo -u $$USER_TO_RUN /usr/local/bin/owonero --install-completions zsh || true; \
	sudo -u $$USER_TO_RUN /usr/local/bin/owonero --install-completions fish || true; \
	echo "✅ Completions installation attempted (check ~/.config/... or ~/.local/... for files)"
endif

# 🧹 Czyszczenie
clean:
	cargo clean
	@echo "🧹 Clean complete."
