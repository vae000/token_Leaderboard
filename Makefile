.PHONY: build build-linux build-windows dist clean

# 同时构建 Linux + Windows
dist: build-linux build-windows
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
	CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
	  cargo build --release --target x86_64-pc-windows-gnu -p cli
	@echo "→ target/x86_64-pc-windows-gnu/release/leaderboard.exe"

clean:
	rm -rf dist/
	cargo clean -p cli
