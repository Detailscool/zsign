# zsign Desktop

The desktop app is under active development. It uses the existing zsign CLI to
sign and currently requires real Apple account and device validation before it
can be considered production-ready.

On macOS install Rust, Xcode, pkg-config and OpenSSL (including static libraries),
then run `make release-app` from the repository root. The result is
`gui/dist/zsign Desktop.app`. Run `make release-cli` to build only `bin/zsign`
with statically linked OpenSSL. Both targets currently build for macOS.
If the active developer directory is Command Line Tools, select Xcode for the
build by setting `DEVELOPER_DIR` to its `Contents/Developer` directory.
The script links OpenSSL into the bundled CLI; the minimum macOS version of the
result depends on the OpenSSL libraries and compiler used to build it.

On macOS, remembered login sessions and their account/Anisette settings are
stored in the app's user preferences, not the Keychain. Passwords and
verification codes are not stored. Signing private keys still use the Keychain.
Sessions saved by earlier versions in the Keychain are not migrated; sign in
again to save a session in preferences.

Linux and Windows device support uses `idevice_id`, `ideviceinfo`, and
`ideviceinstaller` on PATH. Their packaging and real-device workflows have not
yet been verified.
