.PHONY: release-app release-cli

# The desktop bundle includes its own statically linked zsign executable.
release-app:
	@sh gui/build-macos.sh

# Use the same static OpenSSL linkage as the desktop bundle on macOS.
release-cli:
	@ssl_dir="$$(pkg-config --variable=libdir openssl)"; \
	test -f "$$ssl_dir/libssl.a" && test -f "$$ssl_dir/libcrypto.a" || \
	  { echo "OpenSSL static libraries are required (libssl.a and libcrypto.a)." >&2; exit 1; }; \
	$(MAKE) -C build/macos -B -j "$$(sysctl -n hw.ncpu)" \
	  LIBS="$$ssl_dir/libssl.a $$ssl_dir/libcrypto.a"
