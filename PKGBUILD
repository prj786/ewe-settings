# Maintainer: scubba
#
# Part of ewe. Built the same way as komble-arch, and for the same
# hard-won reasons — see the two notes below before changing anything here.

pkgname=ewe-settings
pkgver=0.9.6
pkgrel=1
pkgdesc="Settings for the ewe desktop"
arch=('x86_64' 'aarch64')
url="https://github.com/prj786/ewe-settings"
license=('MIT')

depends=('webkit2gtk-4.1' 'gtk3' 'quickshell')
makedepends=('rust' 'cargo' 'nodejs' 'npm')

# The app was called hypr-shell-settings before the DE became ewe; these three
# make pacman treat this package as its direct successor (installing it removes
# the old one instead of file-conflicting with it).
provides=('hypr-shell-settings')
conflicts=('hypr-shell-settings')
replaces=('hypr-shell-settings')

source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('80198f1bb241bb2c40b575ca30e995b70f39890a3477b73fea927bb938831da6')

# Arch enables LTO in makepkg.conf, which injects -flto into CFLAGS/LDFLAGS.
# Any crate shipping hand-written assembly (ring, via rustls) fails to link with
# a wall of "undefined symbol: ring_core_*". Rust-level LTO stays on via
# [profile.release], so the binary is unaffected.
options=(!lto !debug)

build() {
  cd "$srcdir/$pkgname-$pkgver"
  npm ci
  # Through the Tauri CLI, NOT bare `cargo build`. tauri-build decides
  # dev-vs-production at compile time; a plain cargo build stays in dev mode and
  # bakes devUrl (localhost) into the binary, so the installed app launches and
  # shows "Could not connect to localhost: Connection refused".
  npm run tauri build -- --no-bundle
}

package() {
  cd "$srcdir/$pkgname-$pkgver"
  install -Dm755 src-tauri/target/release/ewe-settings "$pkgdir/usr/bin/ewe-settings"
  # A shell that predates the rename still launches `hypr-settings`.
  ln -s ewe-settings "$pkgdir/usr/bin/hypr-settings"
  install -Dm644 packaging/ewe-settings.desktop \
    "$pkgdir/usr/share/applications/ewe-settings.desktop"
  install -Dm644 src-tauri/icons/128x128.png \
    "$pkgdir/usr/share/icons/hicolor/128x128/apps/ewe-settings.png"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
