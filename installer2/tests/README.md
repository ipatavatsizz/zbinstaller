# Test planı

Varsayılan testler gerçek Steam/oyun kurulumuna yazmaz ve Steam'i kapatmaz. Kaynak fixture'lar salt okunur kullanılır; varsayılan test yazmaları geçici dizinlerde yapılır. Gerçek Steam config için aşağıdaki ayrı opt-in güvenlik kapısı geçerlidir. Aşağıdaki opt-in Steam süreç testi yalnızca istemciyi normal kapatıp yeniden açar.

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

Project Zomboid kapalı, Steam açıkken `installer2` dizininde:

```powershell
cargo test -p installer-core platform::native_tests::native_windows_steam_shutdown_restart --locked -- --ignored --exact --nocapture --test-threads=1
```

[Native test](../crates/installer-core/src/platform.rs), üretimde kullanılan `stop_steam` / `start_steam` adaptörleriyle gerçek istemcinin normal kapanmasını, kapalı istemcide tekrar kapatmanın etkisiz olmasını ve 30 saniye içinde Steam sürecinin yeniden görünmesini doğrular. Test hata verse de Steam'i yeniden açmayı dener; zorla sonlandırmaz. Normal `cargo test` ve CI bu testi atlar.

### Gerçek Steam config testi ve oyun sınırı

Kullanıcı 2026-11-05 tarihinde gerekli Steam değişikliklerini bağımsız geri yüklenebilir yedeklerle onayladı. `platform::native_tests::native_windows_steam_localconfig_backup_roundtrip` artık Steam dışında kalıcı recovery dizinine orijinal config/backup varlık durumu, içerik, SHA-256, readonly ve Windows DACL kaydeder; geri yüklemeyi Steam yeniden başlamadan doğrular. Cleanup hatası test başarısızlığıdır ve Steam kapalı bırakılır. Dört geçici dizin/kapsam regresyon testi ve aşağıdaki tek gerçek config testi 2026-11-05 tarihinde geçti; Tester kabulü ayrıca gereklidir. Normal test/CI bunu atlar; toplu `--ignored` çalıştırılmaz.

```powershell
cargo test -p installer-core platform::native_tests::native_windows_steam_localconfig_backup_roundtrip --locked -- --ignored --exact --nocapture --test-threads=1
```

Kalıcı recovery dizini ve manifest yolu test çıktısında gösterilir; başarılı testten sonra da otomatik silinmez. `manifest.json`, numaralı `.original` dosyalarını tam hedef yolları ve orijinal izinlerle eşler. Hata halinde Steam kapalı tutulur; manifest/kopyaları özel koruyup yalnız listelenen hedefleri geri yükleyin, başlangıçta olmayan dosyaları kaldırın ve içerik SHA-256/readonly/DACL doğrulanana kadar Steam'i açmayın. Ham config veya manifest içeriğini paylaşmayın.

`platform::native_tests::native_windows_game_install_uninstall_roundtrip` gerçek oyun dosyalarına yazar ve **çalıştırılmayacaktır**. Gerçek oyun kurulumu, tekrar kurulum, kaldırma, oyun/mod açılışı ve LTS/Beta gerçek oyun matrisi mevcut görev kapsamı dışındadır. Çalıştırılmamış olmaları başarı değildir ve bu görevin tamamlanmasını engellemez. İzole fixture/başlatıcı testleri gerçek oyun açılışı kanıtı sayılmaz.

## Gerçek kabul çalıştırma prosedürü (30–34)

Güncellenen sıra: (1) kapsam/izin belgeleri → (2) bağımsız yedekli Windows gerçek Steam config roundtrip ve odaklı kontroller → (3) Linux/macOS native Actions'ta production artifact görünür pencere smoke ve mevcut WDIO → (4) aynı kaynak commit'iyle CI, kanıt ve sonuç belgeleri. Her adım Tester tarafından doğrulanmadan sonrakine geçilmez. İlk belge adımında Steam işlemi yapılmadı. İkinci adımda yalnız Windows config testi çalıştırıldı ve aşağıda kaydedildi; Linux/macOS production smoke ve yeni kaynak CI henüz geçti sayılmaz.

### Windows Steam config: hedef, yedek ve kurtarma kapısı

1. [Proje kurallarındaki](../AGENTS.md) sınırlar geçerlidir. Oyun kapalı olmalı; test oyun dosyalarına yazmamalı veya oyunu açmamalıdır. Steam keşfiyle bulunan, testin gerçekten değiştireceği `localconfig.vdf` ve orijinal `.backup` yollarını işlemden önce belirleyip kaydedin. Beklenmeyen kullanıcı/hedef eklenirse yazmadan durun. Hesap, abonelik veya indirme değişikliği kapsamda değildir.
2. Steam'in ilk açık/kapalı durumunu kaydedin ve normal kapanmasını bekleyin; zorla sonlandırmayın. Kapanıştan sonra her config/orijinal backup için varlık-yokluk, ham içerik hash'i ve ilgili izinleri kaydedin. İlk yazmadan önce Steam/oyun hedefleri dışında kalıcı bağımsız disk yedeği oluşturun; içeriğinin geri yüklenebilir olduğunu doğrulayın. Sadece bellek snapshot'ı veya uygulamanın `.backup` dosyası yeterli değildir.
3. Config roundtrip yalnız belirlenen hedeflerde patch/ilk backup/idempotency beklentilerini doğrular; ilgisiz seçeneklerin korunması ve başlangıç backup'ının aynı kalması odaklı kanıta yazılır. Gerçek oyun kurulumu ve Workshop indirmesi bu testin parçası olamaz.
4. Başarıda ve hatada bütün geçici config değişikliklerini ve orijinal `.backup` dosyalarını (başlangıçta yoksa yokluklarını) geri yükleyip hash/izinleri doğrulayın. Cleanup hatası testi başarısız kılmalı; başarılı patch sonucu hatayı gizlememelidir. Kalıcı kurtarma yedeği ve hata raporu korunmalıdır.
5. Geri yükleme doğrulandıktan sonra Steam'i başlangıçtaki açık/kapalı durumuna normal başlatma/kapatmayla döndürün. Geri yükleme belirsizse Steam'i otomatik açmayın; hedef ve bağımsız kurtarma yollarını raporlayın. Steam'in kendi normal açılış/kapanış yazmalarını test patch'iyle karıştırmayın.

Bu kapı test desteğinde uygulandı ve 2026-11-05 config roundtrip çalışmasında geçti: iki config/backup hedefi, mevcut managed flag durumundan geçici scoped baseline ve zorunlu gerçek patch yazımı, normal kapanış 2,667 sn, scoped patch/ilk backup/idempotency ve restart öncesi doğrulanmış cleanup; normal restart 1,061 sn. Steam sonunda açık. Yerel kanıt: `target/acceptance/windows-steam-config-step2-final.log`; özel recovery yolu logda bulunur. Steam EXE FileVersion `11.15.70.27`, ProductVersion `01.00.00.02`. Kaynak kimliği HEAD `aed9b22` + commit edilmemiş ikinci adım değişiklikleridir; tarihsel `c7662e9` binary kabulü değildir. Mevcut aç/kapat testinin tarihsel başarısı bu yeni config kanıtından ayrıdır.

### Aday ve Linux/macOS native Actions kabulü (30, 33)

Linux x64 ve Apple Silicon macOS arm64 için native GitHub Actions runner'ları kabul edilen doğrulama ortamıdır; ayrı manuel masaüstü makinesi gerekmez. Mevcut `wdio-tests` feature/config ile açılan test uygulamasının native UI başarısı korunur. Buna ek olarak aynı kaynak commit'inden, WebDriver feature/config olmadan üretilen Linux çalıştırılabilir dosyası ve ZIP'ten çıkarılan macOS APP native runner'da görünür pencere açmalı; süreç/pencere kimliği, sürüm, çıktı hash'i, log ve ekran görüntüsü kanıtı toplanmalıdır. Production startup smoke henüz uygulanmadı veya çalıştırılmadı; mevcut WDIO sonucu bu açığı kapatmaz.

Aracın kendisi kurulmaz; setup/DMG/paket yöneticisi adımı eklenmez. Windows production EXE UI kontrolü için mevcut runner `powershell -NoProfile -File tests/native/windows-ui.ps1 -Executable '<indirilen-windows-exe>'` ile kullanılabilir. SmartScreen/Gatekeeper gözlemleri varsa kaydedilir; imzasız deneysel dağıtım politikası değişmez. Ayrı manuel masaüstünde uyarı/açılış denemesi mevcut kapsam dışındadır.

Tarihsel aday `c7662e9e9489a669381ed9315b2f39f0f2fe5612`, belge adımı başlangıç HEAD'i `aed9b2284a73d6008c1ee2a7ab284dbba615ab38`'den ayrıdır. Yeni değişiklikler aynı kaynak commit'iyle CI'da doğrulanmalıdır; eski adayın binary/hash/UI sonuçları yeni commit'e aktarılmaz. Artifact adı/URL, kaynak commit ve beklenen/ölçülen SHA-256 ayrı kaydedilir. Dosya hash'i için:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath '<windows-exe>'
```

```sh
# Linux executable
sha256sum '<linux-dosyasi>'
# macOS ZIP
shasum -a 256 '<macos-zip>'
```

### Oturum kanıt şablonu

Kanıtları Steam/oyun dışında `acceptance/<tam-kaynak-commit>/<platform>/<UTC-oturum>/record.md` düzeninde tutun. Bu yol şablondur; mevcut kanıt iddiası değildir. Her kontrol için `Geçti / Başarısız / Engelli / Çalıştırılmadı / Kapsam dışı`, UTC, beklenen-gözlenen sonuç ve kanıt bağlantısı yazın. Ham config/yedekler özel tutulur; paylaşılan kanıtta hesap belirteçlerini çıkarın.

```text
Oturum / UTC / çalıştıran / Tester:
Platform / mimari / OS-runner-image / WebView sürümü:
Kaynak tam commit / yerel HEAD / tarihsel aday kimliği:
Workflow-run-job URL / artifact adı-URL / beklenen ve ölçülen SHA-256:
Windows Steam sürümü-build / ilk açık-kapalı durumu:
Önceden belirlenen config ve orijinal backup hedefleri:
Bağımsız disk yedeği / varlık-yokluk-hash-izin / kurtarma doğrulaması:
31 Windows config patch-backup-idempotency / cleanup / Steam son durumu / kanıt:
30 Linux native WDIO test-build / production artifact süreç-pencere / kanıt:
30 macOS native WDIO test-build / ZIP-APP production süreç-pencere / kanıt:
33 imza politikası / mevcut uyarı gözlemi / kanıt:
34 aynı kaynak CI / üç artifact-hash listesi / kalan kontrol / kanıt:
Gerçek oyun ve ayrı manuel masaüstü: Kapsam dışı, çalıştırılmadı:
Son durum / hatalar / kurtarma yolu / Tester kararı:
```

Final kontrol (34) bu onaylı kapsamı esas alır: güvenli Windows Steam config sonucu, Linux/macOS native WDIO ve production pencere smoke'u, aynı kaynak CI ve dosya/hash kanıtları. Gerçek oyun/manuel masaüstü kabulü başarı sayılmaz ve görev engeli değildir. Public yayın ayrı karardır; bu çalışma otomatik public yayın yapmaz.

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

Bu UI kontrolü gerçek Steam config roundtrip sonucu değildir. Gerçek oyun ve ayrı manuel masaüstü kabulü mevcut kapsam dışındadır; güncel kapsam ve sonuç kaydı [geçiş listesinde](../docs/MIGRATION.md#kalan-işler-sırayla) bulunur.

[Üç platform workflow](../../.github/workflows/installer2.yml) aynı kontrolleri native runner'larda çalıştırır. Native UI/Steam/oyun kabulü ve imza gereksinimleri [migration checklist'te](../docs/MIGRATION.md) ayrı tutulur. Windows'ta üç hedefin dosya düzenini test etmek Linux/macOS native testi sayılmaz.
