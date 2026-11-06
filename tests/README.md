# Test planı

Komutlar depo kökünden çalıştırılır. [Native UI yönergesindeki](native/unix-ui-acceptance.md) yerel yollar, `--target` kullanılmayan derlemenin `target/release/` çıktısını kullanır; dosya adı `src-tauri/tauri.conf.json` içindeki `mainBinaryName` alanından okunur. CI'da hedef mimariye ait yol derleme adımından test adımlarına aktarılır.

Varsayılan testler gerçek Steam/oyun kurulumuna yazmaz ve Steam'i kapatmaz. Kaynak fixture'lar salt okunur kullanılır; varsayılan test yazmaları geçici dizinlerde yapılır. Gerçek Steam config için aşağıdaki ayrı opt-in güvenlik koşulları geçerlidir. Aşağıdaki opt-in Steam süreç testi yalnızca istemciyi normal kapatıp yeniden açar.

## Installer kabul ölçütleri

Kabul, oyun kapalıyken installer'ın dosya ve yapılandırma sonuçlarını doğrular. LTS/Beta için izole fixture'larda:

| Platform | Kopyalanan payload | Yönetilen başlatma seçeneği |
| --- | --- | --- |
| Windows | `zbNative.dll` ve `ZombieBuddy.jar` | `-agentlib:zbNative --` |
| Linux | `libzbNative.so` ve `ZombieBuddy.jar` | `-javaagent:ZombieBuddy.jar --` |
| macOS | `libzbNative.dylib` ve `ZombieBuddy.jar` | `-javaagent:ZombieBuddy.jar --` |

Steam uses the trailing ` --` shown above. JSON/BAT/shell use only `-agentlib:zbNative` (Windows) or `-javaagent:ZombieBuddy.jar` (Linux/macOS). Reinstall normalizes old separator-bearing patches and duplicate managed agents. Regression tests preserve independent user `--` arguments (including a leading separator through repeat install/uninstall), comments, shebangs and LF/CRLF endings; uninstall accepts both old and current patches. The real `cmd.exe` fixture starts with duplicate old/current agent flags and checks that the resulting `_JAVA_OPTIONS` contains one agent argument and no managed separator.

- Kurulum doğru platform payload'ını ve JSON/BAT/shell/Steam başlatma seçeneklerini uygular; ilgisiz alanları korur.
- Tekrar kurulum idempotenttir: bayraklar çoğalmaz, ilk `.backup` içerikleri değişmez.
- Kaldırma yönetilen payload/bayrakları temizler; sonradan eklenen ilgisiz kullanıcı seçeneklerini ve orijinal backup'ları korur.
- Yazmadan önce bütün yamalar hazırlanır; dosyalar, izinler ve önceden olmayan hedefler rollback/kurtarma kapsamındadır. Başarısız işlem ve yarım journal güvenli geri dönüşle sınanır.
- Oyun açıkken değişiklik reddedilir; Steam normal kapatılır ve son config yazımları sonrasında yeniden keşif/snapshot yapılır.

Project Zomboid'i veya modu oyun içinde başlatmak installer kabulü ya da public yayın için önkoşul değildir. Geliştiricinin gerçek oyununda kurulum testi yapılmaz; dosya/kurulum/rollback testleri geçici dizinlerde kalır. Fixture/probe süreçleri oyun içi çalışmayı kanıtlamaz. Kaynak, dosya hash'leri ve platform kanıtları [RELEASE.md](../docs/RELEASE.md#son-doğrulanan-kaynak-ve-dosyalar) içinde tutulur.

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

Bu testler normal workspace/CI testlerine dahildir; gerçek Steam adaptörü yerine izole süreç adaptörü kullanır. Gerçek Steam hesabı veya Workshop aboneliği testi değildir. Linux CI'da `download_completion_flush_and_new_user_are_seen_before_installation` testinin 100 ms fixture indirme bütçesi aralıklı zaman aşımı üretebilir; yeniden koşu sonucu kaydedilmeli, ilk hata gizlenmemelidir.

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

### Windows başlatıcı süreç kabulü

`cargo test -p installer2-tests --test windows-launcher --locked` LTS ve Beta için kurulum, tekrar kurulum, iki yönde branch değişimi ve kaldırmadan sonra yamalanan BAT'ı gerçek `cmd.exe` ile çalıştırır. Alt test süreci gerçek ortam değişkenlerini ve payload dosyalarını okur. Boşluk/`&` içeren yollar, kullanıcı düzenlemeleri, ilk backup ve yönetilen bayrakların temizlenmesi sınanır. Bütün dosyalar geçici dizinlerdedir. Bu başlatıcı süreç testi gerçek JVM/oyun/mod açılışı değildir. Windows dışındaki runner'larda bu hedef sıfır test içerir.

## UI sözleşmeleri

Diagnostics testleri ayrıca eşzamanlı yenilemelerin tekilleştirilmesini, bekleyen eski yanıtın temizlenen logu geri getirmemesini, yerel işlem/temizleme hatalarının sonraki yenilemede görünür kalmasını ve başarılı temizlemenin hem yerel hem backend geçmişini kaldırmasını doğrular.

Dosya raporu veya durum isteğinden biri başarısız olduğunda diğerinin gösterilmesi, sonraki yenilemede toparlanma, log okuma konumunun korunması, en altta yeni kayıtların takip edilmesi ve değişmeyen metnin yeniden oluşturulmaması da sınanır.

`npm run test:ui`, Tauri API'sini mock'layarak branch seçimi, Workshop, indirme beklemesi/iptali, iki branch'in birlikte kilitlenmesi, ilerleme yüzdesi, bitiş/hata mesajları, F5/Escape, menü/kaldırma, diagnostics ve log temizleme davranışlarını sınar. DOM testlerinin gerçek native pencere davranışını doğruladığı iddia edilmez.

## Tam kontrol

```powershell
npm run build
npm run test:ui
npm run test:release
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run tauri build
```

## Windows native Steam süreç kabulü (opt-in)

Project Zomboid kapalı, Steam açıkken proje kökünde:

```powershell
cargo test -p installer-core platform::native_tests::native_windows_steam_shutdown_restart --locked -- --ignored --exact --nocapture --test-threads=1
```

[Native test](../crates/installer-core/src/platform.rs), üretimde kullanılan `stop_steam` / `start_steam` adaptörleriyle gerçek istemcinin normal kapanmasını, kapalı istemcide tekrar kapatmanın etkisiz olmasını ve 30 saniye içinde Steam sürecinin yeniden görünmesini doğrular. Test hata verse de Steam'i yeniden açmayı dener; zorla sonlandırmaz. Normal `cargo test` ve CI bu testi atlar.

### Gerçek Steam config roundtrip (opt-in)

```powershell
cargo test -p installer-core platform::native_tests::native_windows_steam_localconfig_backup_roundtrip --locked -- --ignored --exact --nocapture --test-threads=1
```

Yalnız tam adlı testi seçin; toplu `--ignored` çalıştırmayın. `platform::native_tests::native_windows_game_install_uninstall_roundtrip` gerçek oyun dosyalarına yazar ve çalıştırılmaz. [Proje kuralları](../AGENTS.md) hedefli Steam config testine bağımsız geri yüklenebilir yedekle izin verir; oyun dosyaları ile kapsam dışı hesap/abonelik/indirme değişiklikleri yasaktır.

1. Oyun kapalı olmalı. Keşfedilen `localconfig.vdf` ve orijinal `.backup` hedeflerini önceden belirleyin; beklenmeyen hedefte yazmadan durun. Steam'in ilk açık/kapalı durumunu kaydedip normal kapanmasını bekleyin; zorla sonlandırmayın.
2. Steam kapandıktan sonra ilk yazmadan önce hedefler dışında kalıcı, bağımsız disk yedeği alın. Her config/backup'ın varlık-yokluk, içerik/SHA-256, readonly ve DACL durumunu kaydedip yedeği doğrulayın. Bellek snapshot'ı veya uygulamanın `.backup` dosyası tek başına yeterli değildir.
3. Yalnız Project Zomboid launch seçeneklerinde gerçek patch, ilk backup ve idempotency doğrulanır. Başlangıçta yönetilen flag varsa geçici scoped baseline kullanılır; ilgisiz config byte'ları/seçenekleri korunmalıdır. Oyun kurulumu veya Workshop işlemi çağrılmaz.
4. Başarı ve hatada geçici config değişikliklerini ve orijinal backup içerik/izinlerini veya başlangıçtaki yokluklarını geri yükleyip doğrulayın. Cleanup hatası testi başarısız kılar; belirsiz config üzerinde Steam yeniden açılmaz. DACL karşılaştırması yalnız bilgilendirici `AI` bitini dışlar, `AR` ve diğer kontrol bayrakları ile sıralı ACE'leri korur.
5. Geri yükleme doğrulandıktan sonra Steam'in başlangıçtaki açık/kapalı durumunu normal işlemlerle geri sağlayın. Recovery dizini ve `manifest.json` yolu çıktıda gösterilir; başarılı testten sonra da korunur. Hata halinde yalnız manifest'te listelenen hedefleri geri yükleyin, başlangıçta olmayan dosyaları kaldırın ve hash/izin doğrulanana kadar Steam'i açmayın. Config, manifest ve ham yedekleri paylaşmayın.

İzole kurtarma ve izin regresyonlarını gerçek Steam'e dokunmadan çalıştırmak için:

```powershell
cargo test -p installer-core platform::steam_config_test_support --locked
```

Tekrar kullanılabilir oturum kaydı: kaynak commit'i, OS/mimari, test komutu, UTC, beklenen/gözlenen sonuç ve log/ekran bağlantısı; config testinde Steam sürümü, tam hedef listesi, özel recovery yolu ve doğrulanan son durum; native uygulama testinde indirilen artifact adı/URL, SHA-256 ve pencere/PID kanıtı tutulur. Çalıştırılmamış kontroller başarılı sayılmaz. [Doğrulanmış sonuçlar](../docs/RELEASE.md#son-doğrulanan-kaynak-ve-dosyalar), komutlardan ayrı kaydedilir.

## Linux/macOS native UI testi

Deneysel native UI suite, Linux ve macOS'ta gerçek Tauri penceresini embedded WebDriver ile açar. Test eklentileri yalnızca `wdio-tests` feature derlemesinde bulunur; release uygulamasına eklenmez. [Kurulum, komutlar ve kapsam](native/unix-ui-acceptance.md). GitHub Actions her iki platformda testleri çalıştırıp ekran görüntülerini artifact olarak saklar.

## Windows native UI kabulü

Kabul doğrudan release EXE'si üzerinde yapılır; aracın kendisini kuran bir setup adımı yoktur. CI indirmesini sınamak için runner'a `-Executable <indirilen-exe-yolu>` verilebilir. Linux'ta doğrudan çalıştırılabilir dosya, macOS'ta ZIP'ten çıkarılan APP native olarak sınanır.

Windows üzerinde Node.js 22+ ve WebView2 ile, proje kökünde:

```powershell
npm run tauri build
npm run test:native:windows
```

[PowerShell runner](native/windows-ui.ps1), üretilen release uygulamasını ayrı bir geçici WebView2 profiliyle açar. [WebView testi](native/windows-ui.mjs), native pencere boyutu/odak/stilleri, logo ve yerleşim, klavyeyle menü kullanımı, F5 ile diagnostics açma/yeniden açma ve Escape ile çıkışı doğrular. Kurulum, kaldırma, Workshop ve yama komutları test sırasında engellenir; Steam kapatılmaz. Odak kontrolü için testi etkileşimli Windows masaüstünde çalıştırın ve bitene kadar başka pencereye geçmeyin.

Bu UI kontrolü Steam config roundtrip sonucundan ayrıdır. [Üç platform workflow](../.github/workflows/installer2.yml) native runner'larda build/test/lint ve Linux/macOS pencere kontrollerini çalıştırır. Fixture'da üç platform dosya düzenini sınamak native UI testi değildir. [Kabul ölçütleri](#installer-kabul-ölçütleri) ve [kaynak/platform kanıtı](../docs/RELEASE.md#son-doğrulanan-kaynak-ve-dosyalar) ayrı tutulur.
