.PHONY: build build-linux build-windows dist clean

WINDOWS_TARGET := x86_64-pc-windows-gnu
WINDOWS_LINKER := x86_64-w64-mingw32-gcc

# 本机构建 Linux；若已具备 Windows 交叉编译条件，则一并构建 Windows
dist: build-linux
	@if rustup target list --installed | grep -qx '$(WINDOWS_TARGET)' && command -v $(WINDOWS_LINKER) >/dev/null 2>&1; then \
		$(MAKE) build-windows; \
	else \
		echo "Skip Windows build: missing $(WINDOWS_TARGET) target or $(WINDOWS_LINKER)"; \
		echo "  安装 target: rustup target add $(WINDOWS_TARGET)"; \
		echo "  安装 linker: sudo apt install gcc-mingw-w64-x86-64"; \
	fi
	@echo ""
	@echo "━━━━ 构建完成 ━━━━"
	ls -lh target/release/leaderboard target/x86_64-pc-windows-gnu/release/leaderboard.exe 2>/dev/null || true

# Linux
build: build-linux
build-linux:
	cargo build --release -p cli
	@echo "→ target/release/leaderboard"

# Windows（需 mingw: sudo apt install gcc-mingw-w64-x86-64）
build-windows:
	@if ! rustup target list --installed | grep -qx '$(WINDOWS_TARGET)'; then \
		echo "缺少 Rust target: $(WINDOWS_TARGET)"; \
		echo "先执行: rustup target add $(WINDOWS_TARGET)"; \
		exit 1; \
	fi
	@if ! command -v $(WINDOWS_LINKER) >/dev/null 2>&1; then \
		echo "缺少 Windows linker: $(WINDOWS_LINKER)"; \
		echo "先执行: sudo apt install gcc-mingw-w64-x86-64"; \
		exit 1; \
	fi
	CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
	  cargo build --release --target $(WINDOWS_TARGET) -p cli
	@echo "→ target/x86_64-pc-windows-gnu/release/leaderboard.exe"

clean:
	rm -rf dist/
	cargo clean -p cli
