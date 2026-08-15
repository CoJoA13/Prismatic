# COPR packaging

Prismatic v1 uses COPR as its binary channel. A release build consumes the signed tag
and `Prismatic-<version>-vendor.tar.xz` asset produced by `just source-tarball`.

Recommended COPR package settings:

- clone URL: `https://github.com/CoJoA13/Prismatic.git`
- spec: `packaging/fedora/prismatic.spec`
- chroots: `fedora-44-x86_64`, `fedora-44-aarch64`
- network during build: disabled
- auto-rebuild: tags only

Do not publish a COPR build until every item in `docs/release-checklist.md` is complete.
