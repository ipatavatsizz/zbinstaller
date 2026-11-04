# Test planı

Varsayılan testler gerçek Steam/oyun kurulumuna yazmaz ve Steam'i kapatmaz. Kaynak fixture'lar salt okunur kullanılır; bütün test yazmaları geçici dizinlerde yapılır. Aşağıdaki opt-in Steam süreç testi yalnızca istemciyi normal kapatıp yeniden açar.

## Rust çekirdek testleri

`cargo test -p installer-core` aşağıdakileri doğrular:

- JSON/BAT/shell/VDF yamaları, idempotency, diğer uygulama/ayarların korunması.
- Shebang, satır sonları, mevcut JVM seçenekleri ve eski Unix enjeksiyonunun kaldırılması.
- BOM, quoted/unquoted VDF, escape ve yorumlar; belirsiz hedeflerin reddedilmesi.
- Workshop kaydı ve hazır payload'ın aynı kütüphaneden seçilmesi.
- İndirme beklemesinin iptali ve zaman aşımı; hiçbir süreç işlemi çağrılmaz.
- Yazma hatası sonrası eski dosyaların geri yüklenmesi, yeni dosya/yedeklerin kaldırılması.
- Yarım kalmış journal'ın sonraki çalıştırmada kurtarılması ve izin verilmeyen hedeflerin reddi.
- İkinci dosya tanıtıcısı ve ayrı test sürecinin aynı kurulum kilidini alamaması.

## Windows'ta izole süreç senaryoları

`cargo test -p installer-core install::scenario_tests --locked` üretimdeki kurulum orkestrasyonunu geçici Steam/oyun dizinleri ve ayrı bir test süreciyle çalıştırır. İki branch için kısmi indirme, tamamlanma, iptal/zaman aşımı, kapanışta config flush ve sonradan eklenen kullanıcı, oyun açılma yarışı, yazma hatası/rollback, yeniden başlatma ve kurtarma hataları doğrulanır. Sekiz yazma noktasının her birinde gerçek atomik replacement'tan sonra hata üretilip eski/yeni dosya ve backup durumları karşılaştırılır.

Bu testler normal workspace/CI testlerine dahildir; gerçek Steam adaptörü yerine izole süreç adaptörü kullanır. Gerçek Steam hesabı, Workshop aboneliği veya oyun açılışı kabulü sayılmaz.

## Fixture entegrasyonu

[Windows fixture testleri](tests/integration/installation_fixtures.rs) ve [platform eşlik testleri](tests/integration/platform_parity.rs), `cargo test -p installer2-tests` ile çalışır:

- Üç platform × iki branch için kurulum, yeniden kurulum ve kaldırma.
- Platforma uygun payload/flags, iç içe JSON yolu, `.sh`/`.command`/`start-server.sh` seçimi.
- Native Unix çalıştırma izninin korunması (Unix runner'larında).
- Eski Go yedeklerinin korunması ve ilk yedeğin yeniden kurulumda değişmemesi.
- Kaldırmadan önce yapılan kullanıcı ayarlarının korunması; nested Windows VM argümanlarının temizlenmesi.
- İkinci Steam kullanıcı girdisi bozuksa ilk kullanıcıya veya yedeğe hiçbir yazma yapılmaması.
- Standart/Flatpak/Snap/macOS yol adayları; oyun yokken kısmi diagnostics raporu.

Fixture testlerinde native adaptörler gerçek Steam'e yönlendirilmez. [Tauri log testi](../src-tauri/src/session_log.rs) yeniden açılışta log okuma ve diskten temizlemeyi doğrular.

## UI sözleşmeleri

Diagnostics testleri ayrıca eşzamanlı yenilemelerin tekilleştirilmesini, bekleyen eski yanıtın temizlenen logu geri getirmemesini, yerel işlem/temizleme hatalarının sonraki yenilemede görünür kalmasını ve başarılı temizlemenin hem yerel hem backend geçmişini kaldırmasını doğrular.

Dosya raporu veya durum isteğinden biri başarısız olduğunda diğerinin gösterilmesi, sonraki yenilemede toparlanma, log okuma konumunun korunması, en altta yeni kayıtların takip edilmesi ve değişmeyen metnin yeniden oluşturulmaması da sınanır.

`npm run test:ui`, Tauri API'sini mock'layarak branch seçimi, Workshop, indirme beklemesi/iptali, iki branch'in birlikte kilitlenmesi, ilerleme yüzdesi, bitiş/hata mesajları, F5/Escape, menü/kaldırma, diagnostics ve log temizleme davranışlarını sınar. DOM testlerinin gerçek native pencere davranışını doğruladığı iddia edilmez.

## Tam kontrol

```powershell
npm run build
npm run test:ui
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run tauri build
```

## Windows native Steam süreç kabulü (opt-in)

Project Zomboid kapalı, Steam açıkken `installer2` dizininde:

```powershell
cargo test -p installer-core platform::native_tests::native_windows_steam_shutdown_restart --locked -- --ignored --exact --nocapture --test-threads=1
```

[Native test](../crates/installer-core/src/platform.rs), üretimde kullanılan `stop_steam` / `start_steam` adaptörleriyle gerçek istemcinin normal kapanmasını, kapalı istemcide tekrar kapatmanın etkisiz olmasını ve 30 saniye içinde Steam sürecinin yeniden görünmesini doğrular. Test hata verse de Steam'i yeniden açmayı dener; zorla sonlandırmaz. Normal `cargo test` ve CI bu testi atlar.

Gerçek Steam `localconfig.vdf` backup/yazma roundtrip testi ayrı olarak şu komutla çalıştırılır:

```powershell
cargo test -p installer-core platform::native_tests::native_windows_steam_localconfig_backup_roundtrip --locked -- --ignored --exact --nocapture --test-threads=1
```

Bu test Steam'i normal kapatır, gerçek kullanıcı `localconfig.vdf` dosyalarında Windows launch option patch'ini ve ilk `.backup` içeriğini doğrular, ardından test öncesi config/backup dosyalarını geri koyup Steam'i yeniden açar. Oyun dosyalarına kurulum yapmaz; hata veya panic sırasında geri yüklemeyi `Drop` koruması dener. Normal `cargo test` ve CI bu testi de atlar.

Gerçek oyun klasöründe seçili ve hazır Workshop branch'iyle tam install/reinstall/uninstall roundtrip testi:

```powershell
cargo test -p installer-core platform::native_tests::native_windows_game_install_uninstall_roundtrip --locked -- --ignored --exact --nocapture --test-threads=1
```

Bu test gerçek Steam keşfi, subscription/payload hazır olma kontrolü, DLL/JAR kopyası, JSON/BAT/`localconfig.vdf` patch ve backup'ları, tekrar kurulumun idempotency'si ve uninstall'ı sınar. Test öncesi oyun/Steam dosyalarını ve backup'ları snapshot'lar, test bitince aynen geri koyar ve Steam'i açık bırakır. Project Zomboid kapalı olmalı; oyun başlatılmaz. Normal test/CI bu testi çalıştırmaz.

İlk süreç testi config veya oyun dosyalarına dokunmaz; Steam kendi normal açılış/kapanış verilerini yazabilir. İkinci test Steam config dosyalarını geçici olarak değiştirir. Üçüncü test ayrıca gerçek oyun hedeflerini geçici olarak değiştirir ve tüm snapshot'ları geri koyar. İki dosya testinde de Workshop indirmesi veya oyunu başlatma yapılmaz.

## Linux/macOS native UI testi

Deneysel native UI suite, Linux ve macOS'ta gerçek Tauri penceresini embedded WebDriver ile açar. Test eklentileri yalnızca `wdio-tests` feature derlemesinde bulunur; release uygulamasına eklenmez. [Kurulum, komutlar ve kapsam](native/unix-ui-acceptance.md). GitHub Actions her iki platformda testleri çalıştırıp ekran görüntülerini artifact olarak saklar.

## Windows native UI kabulü

Kabul doğrudan release EXE'si üzerinde yapılır; aracın kendisini kuran bir setup adımı yoktur. CI indirmesini sınamak için runner'a `-Executable <indirilen-exe-yolu>` verilebilir. Linux'ta doğrudan çalıştırılabilir dosya, macOS'ta ZIP'ten çıkarılan APP native olarak sınanır.

Windows üzerinde Node.js 22+ ve WebView2 ile, `installer2` dizininde:

```powershell
npm run tauri build
npm run test:native:windows
```

[PowerShell runner](native/windows-ui.ps1), üretilen release uygulamasını ayrı bir geçici WebView2 profiliyle açar. [WebView testi](native/windows-ui.mjs), native pencere boyutu/odak/stilleri, logo ve yerleşim, klavyeyle menü kullanımı, F5 ile diagnostics açma/yeniden açma ve Escape ile çıkışı doğrular. Kurulum, kaldırma, Workshop ve yama komutları test sırasında engellenir; Steam kapatılmaz. Odak kontrolü için testi etkileşimli Windows masaüstünde çalıştırın ve bitene kadar başka pencereye geçmeyin.

Bu kontrol gerçek Steam/oyun kabulünün yerine geçmez. Platform ve branch bazında kalan manuel adımlar ve sonuç kaydı [geçiş listesinde](../docs/MIGRATION.md#kalan-işler-sırayla) bulunur.

[Üç platform workflow](../../.github/workflows/installer2.yml) aynı kontrolleri native runner'larda çalıştırır. Native UI/Steam/oyun kabulü ve imza gereksinimleri [migration checklist'te](../docs/MIGRATION.md) ayrı tutulur. Windows'ta üç hedefin dosya düzenini test etmek Linux/macOS native testi sayılmaz.
