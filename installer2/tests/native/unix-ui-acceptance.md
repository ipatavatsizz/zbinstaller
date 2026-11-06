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

## İndirilen production artifact başlangıcı

Native Actions işleri, WebDriver test derlemesinden önce production dosyasını upload edip `actions/download-artifact` ile geri indirir. `cmp` üretilen/indirilen dosyaların aynı olduğunu doğrular. [Production smoke](production-smoke.py), Linux ELF dosyasını veya macOS ZIP'ten `ditto` ile çıkarılan APP executable'ını WebDriver feature/config/argüman enjeksiyonu olmadan çalıştırır. Linux'ta yalnız execute biti geri verilir; dosya içeriği değiştirilmez. Native GitHub Actions kabul edilen ortamdır; ayrı manuel masaüstü veya gerçek Steam/oyun işlemi gerekmez.

Linux'ta Xvfb altında `xdotool` PID, görünürlük, ana pencere başlığı ve boyutunu sorgular; `scrot` ekran görüntüsü alır. macOS'ta sistem Swift/CoreGraphics yardımcısı yalnız başlatılan PID'nin ekrandaki layer-0 ana pencere boyutunu sorgular; `screencapture` pencere görüntüsü alır. İşletim sistemi izin ayarları değiştirilmez; görünür pencere veya screenshot alınamazsa test başarısızdır. Native pencere aşaması 40 saniye, görünür pencere sonrası boyanmış içerik aşaması ayrı 40 saniye, her dış komut 20 saniye, workflow adımı üç dakika ile sınırlıdır. İlk pencere sonrası üç saniye canlı kalma ve yeniden pencere kontrolü gerekir. Ardından ayrı bounded render süresi içinde screenshot'ın başlık/border dışındaki içerik alanında yeterli renk/parlaklık farkı aranır; boş WebView tekrar görüntülenir ve süre dolarsa FAIL olur. Standart Python PNG okuyucusu kullanılır, yeni görüntü paketi gerekmez. Bu kontrol tam UI etkileşim testi değildir; WDIO ayrı kalır. Cleanup yalnız testin başlattığı process group'a TERM, gerekirse KILL gönderir; isimle süreç kapatma yoktur.

`installer2-production-smoke-Linux` / `installer2-production-smoke-macOS` artifact'larında `result.json` (kaynak commit, runner/mimari, indirilen dosya ve executable SHA-256, PID/pencere bilgisi, sonuç), `application.log`, `production-window.png` ve macOS bundle kimliği bulunur. Hata nonzero döner; `always()` kanıt yüklemesi hatayı başarıya çeviremez. Bu smoke production açılışını doğrular; klavye/diagnostics etkileşimleri ayrı WDIO test-build suite'inde kalır. Gerçek Steam/oyun veya imza/Gatekeeper kabulü iddiası değildir.

Yerel yardımcı kontroller (native pencere kabulü sayılmaz):

```sh
python3 -m py_compile tests/native/production-smoke.py
python3 tests/native/production-smoke.py --self-test
```

Kaynak/run/artifact bağlantıları, dosya hash'leri ve hangi platform kontrollerinin gerçekten geçtiği [RELEASE.md](../../docs/RELEASE.md#son-doğrulanan-kaynak-ve-dosyalar) içinde tutulur. Production pencere/çizim kontrolü tam UI etkileşim testi değildir; WDIO ayrı feature build'de çalışır. Steam/oyun işlemleri çağrılmaz. Oyun içi mod açılışı installer kabulü veya public yayın için önkoşul değildir; manuel Gatekeeper/SmartScreen dağıtım akışı bu native testlerin kapsamı dışındadır.
