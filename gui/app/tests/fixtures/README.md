The Mach-O fixtures are minimal iOS ARM64 executables built from
`int main(void) { return 0; }`, or the repository's `test/dylib/bin/demo1.dylib`.
The signed fixtures are ad-hoc signed with the adjacent entitlement plist;
they contain no developer credentials. `unsigned-executable.macho` has its
signature removed. These verify parsing behavior, not device installation.
