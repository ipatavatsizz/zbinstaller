# Linux ve macOS native UI testi

Bu, Windows dışındaki native UI davranışını deneysel CI seviyesinde doğrular. GitHub Actions her değişiklikte Linux x64 ve Apple Silicon macOS runner'ında gerçek Tauri uygulamasını embedded WebDriver ile açar. Test yalnızca test derlemesinde etkinleştirilen WebDriver eklentilerini kullanır; indirmeye sunulan release ikilisine bu eklentiler girmez.

Testte yerel WebView gerçekten açılır; ana pencere içeriği, sabit boyut, logo, taşmayan düzen ve iki branch eylem menüsünün görünür etkileşimi sınanır. Diagnostics gerçek Tauri komutuyla açılır, ikinci native pencerede yüklenmesi beklenir, kapatılır ve native pencere durumundan odağın ana pencereye döndüğü doğrulanır; ardından diagnostics tekrar açılır. Ekran görüntüleri workflow artifact'ında saklanır. Eylemleri yürütecek Steam/oyun, Workshop, install, uninstall, patch ve log-clear çağrıları kabul edilmez. Diagnostics discovery yalnızca okuma yapar. Klavye menü/F5/Escape sözleşmeleri ayrıca [UI testlerinde](../README.md#ui-sözleşmeleri) sınanır.

## Yerel çalıştırma

Node.js 22+, Rust stable ve Linux'ta WebKitGTK 4.1/WebKitWebDriver/Xvfb gereklidir. Proje kökünden `installer2` dizininde:

```sh
npm ci
npm run tauri -- build --no-bundle --features wdio-tests --config tests/native/tauri.wdio.conf.json
```

Linux'ta:

```sh
TAURI_TEST_APP=target/x86_64-unknown-linux-gnu/release/ZombieBuddyInstaller-26.11.3 \
  xvfb-run --auto-servernum npm run test:native:unix
```

Apple Silicon macOS'ta:

```sh
TAURI_TEST_APP=target/aarch64-apple-darwin/release/ZombieBuddyInstaller-26.11.3 \
  npm run test:native:unix
```

Test derlemesinden önce `VITE_WDIO_TEST=true` ortam değişkeni verilmelidir; Tauri CLI `beforeBuildCommand` ile Vite build'ini kendisi çalıştırır. Gerçek dağıtım build'i aynı özellik bayrağı/config olmadan üretilir. Suite bir bütün olarak geçmeli ve her iki platform işinden de `main.png` ile `diagnostics.png` çıktıları alınmalıdır.

Bu test native UI/webview başlangıç ve etkileşimini doğrular; gerçek Steam/oyun davranışı, Apple Gatekeeper imzası veya farklı masaüstü dağıtımlarının kabulü değildir. Hatalar bulunduğu platformun native CI logunda raporlanır.
