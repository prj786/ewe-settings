# Maintainer: scubba
#
# Part of hypr-shell. Built the same way as komble-arch, and for the same
# hard-won reasons — see the two notes below before changing anything here.

pkgname=hypr-shell-settings
pkgver=0.4.0
pkgrel=1
pkgdesc="Settings for the hypr-shell desktop"
arch=('x86_64' 'aarch64')
url="https://github.com/prj786/hypr-shell-settings"
license=('MIT')

depends=('webkit2gtk-4.1' 'gtk3' 'quickshell')
makedepends=('rust' 'cargo' 'nodejs' 'npm')

source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('59f655a6f5063285c88ad21e859f1d4baaae819cb806fb0bd13deea241246ae5')

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
  install -Dm755 src-tauri/target/release/hypr-settings "$pkgdir/usr/bin/hypr-settings"
  install -Dm644 packaging/hypr-settings.desktop \
    "$pkgdir/usr/share/applications/hypr-settings.desktop"
  install -Dm644 src-tauri/icons/128x128.png \
    "$pkgdir/usr/share/icons/hicolor/128x128/apps/hypr-settings.png"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
