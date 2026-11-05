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

2026-11-05 ilk native kanıt: kaynak `460b969d327411a8024b09345121740c4dfc37cd`, [Actions run](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42057743363). [Linux işi](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42057743363/job/127350652288) ve [Apple Silicon macOS işi](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42057743363/job/127350652999), indirilen production smoke ve mevcut WDIO dahil geçti. İki production ekran görüntüsü indirildi ve ana görünüm görsel olarak doğrulandı; artifact adları yukarıda. Aynı run'ın Windows işi geçici dosya regression'ında DACL normalizasyon farkı nedeniyle başarısız oldu; bu run üç platformun tamamı geçti olarak sunulmaz. Dar düzeltme sonrası yeni aynı kaynak CI doğrulaması ve Tester kabulü ayrıca kaydedilecektir. Tarihsel WDIO sonuçları production smoke yerine kullanılmadı.

Sonraki `a2fc7abb7f5f3ed4a0df81871c4de9e18a862872` [run'ında](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42058808479) Windows DACL regression'ları geçti; karşılaştırma bilgilendirici AUTO_INHERITED marker yerine sıralı binary ACE ve inheritance-protection durumunu kullanır, hak/ACE sırası/inherited/protected farkını reddeder. Ancak macOS smoke'un yeşil sonucuna rağmen indirilen screenshot görsel incelemede boş WebView gösterdi; bu görüntü production UI kabulü sayılmadı. Smoke bu boşluk için bounded painted-content kontrolü ve boş screenshot regression'ıyla düzeltildi. Yerel kontrol ilk Linux/macOS boyanmış screenshot'larını kabul etti, bu boş macOS screenshot'ını reddetti. Yeni kaynak native CI sonucu ayrıca doğrulanacaktır.

`32bf9e9` koşusunda paint-aware macOS smoke geçti; Linux'ta native pencere geç bulununca ortak startup bütçesi ilk screenshot'tan önce tükendi (`paint_attempts=[]`). Bu test bütçesi kusuru, native pencere ve boyanmış içerik için ayrı sınırlı aşamalarla düzeltildi; Linux'ta uygulamanın boş kaldığı iddiası kurulmadı. Başarısız koşunun log/JSON kanıtı korunur.
