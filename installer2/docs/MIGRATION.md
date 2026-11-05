# Go → Rust/Tauri geçiş durumu

## Sonuç ve kapsam

Eski Go uygulamasının Windows, Linux ve macOS için gerçekleştirdiği kurulum davranışları Rust çekirdeğine taşındı. Aşağıdaki işaretli maddeler uygulanmış geliştirme işlerini gösterir. Açık yayın kabul maddeleri ayrı bölümde sıralanır. Bu, üç işletim sisteminde gerçek Steam kurulumunun doğrulandığı anlamına gelmez; yayın kabulü ayrı tutulur.

Önceki incelemeye düzeltme: Go sürümü yalnızca JSON başlatıcısını alt klasörlerde arar. Unix başlatıcısını oyun kökünde `ProjectZomboid64.sh`, `ProjectZomboid64.command`, `start-server.sh` sırasıyla seçer. Yeni sürüm aynı seçimi uygular.

Dağıtım kararı (2026-11-04): Bu araç kendisini sisteme kurmaz. Kullanıcı Windows EXE'sini, Linux çalıştırılabilir dosyasını veya ZIP içindeki macOS APP'ini indirip açar; işi bitince aracı silebilir. Kurulum/kaldırma eylemleri ZombieBuddy moduna aittir. Araç silindiğinde mod, oyun yedekleri ve kalıcı diagnostics geçmişi otomatik silinmez. Setup kurucusu ve sistem paket yöneticisi kurulumu yayın şartı değildir. Önceki NSIS/DEB/AppImage/DMG yönlendirmeleri bu kararla kaldırıldı; geçmiş CI kanıtları aşağıda yalnızca tarihsel kayıt olarak korunur.

Yerel doğrulama (2026-11-04): `npm run tauri build` doğrudan `target/release/ZombieBuddyInstaller-26.11.3.exe` uygulamasını üretti; setup adımı yoktur. Yeni EXE üzerinde `npm run test:native:windows` geçti (açılış, görünüm, klavye, diagnostics yeniden açılışı ve çıkış). 24 UI testi, 8 yayın/link testi, workspace Rust testleri, biçim kontrolü ve Clippy geçti. Gerçek Steam/game dosya yazma testleri çalıştırılmadı. Windows süreç/başlatıcı senaryoları 31–32'de, uzak CI kanıtı 30 ve 34'te izlenir.

## Tamamlanan geliştirme işleri

Her maddede ilk cümle görevi, devamı mevcut uygulamayı açıklar. `[x]` tamamlandı; `[ ]` henüz tamamlanmadı. Bu bölümün tamamlanması yayın kabulünün tamamlandığı anlamına gelmez.

- [x] **1. Mevcut Git değişikliklerini korumak.** `migration/complete-go-parity` dalı; başlangıç değişiklikleri ayrı commit'te korundu.
- [x] **2. Windows Steam Registry keşfi.** HKCU/HKLM, 32/64 bit görünümler, `SteamPath`/`InstallPath` korunur.
- [x] **3. Linux Steam keşfi.** `.steam/steam`, `.local/share/Steam`, küçük harfli alternatif, Flatpak ve Snap yolları taşındı.
- [x] **4. macOS Steam keşfi.** Kullanıcı Steam verisi ve sistem/kullanıcı Steam.app çalıştırıcıları taşındı.
- [x] **5. Ek Steam kütüphaneleri ve oyun manifestleri.** Yeni/eski `libraryfolders.vdf` biçimleri, özel oyun klasör adı ve varsayılan klasör desteği; bozuk oyun manifestinde diğer kütüphanelere devam edilir.
- [x] **6. LTS/Beta Workshop kimlikleri ve sürümleri.** Aynı kimlikler, `VERSION` okuma, eksik sürüm ile abonelik ayrımı korunur.
- [x] **7. Workshop sayfası açma.** Sabit Workshop kimlikleri Steam istemcisine iletilir; aynı anda kurulumla çalışamaz.
- [x] **8. İndirmenin tamamlanmasını bekleme.** 60 saniye zaman aşımı, düzenli yeniden keşif ve iptal; hazır dosyalar ile varsa ACF kaydı aynı kütüphaneden doğrulanır. Eski sürümdeki ACF bulunmayan yerel payload desteği korunur.
- [x] **9. Platforma uygun payload kopyalama.** Windows DLL, Linux SO, macOS DYLIB ve ortak JAR dosyası; boş payload reddedilir.
- [x] **10. Platforma uygun başlatma bayrakları.** Windows `-agentlib:zbNative --`; Unix `-javaagent:ZombieBuddy.jar --`.
- [x] **11. JSON başlatıcısının alt klasörlerde bulunması.** Kök öncelikli ve büyük/küçük harfe duyarsız arama; directory symlink'leri izlenmez.
- [x] **12. JSON yaması.** Yalnızca root `vmArgs` kurulum sırasında değiştirilir; diğer ayarlar korunur; tekrar kurulum bayrağı çoğaltmaz.
- [x] **13. Windows BAT yaması.** Mevcut `_JAVA_OPTIONS` ve satır sonları korunur; mevcut çıplak native bayrağı normalize edilir.
- [x] **14. Linux/macOS betik yaması.** Üç eski başlatıcı adı desteklenir; `_JAVA_OPTIONS` korunur; shebang ilk satırda ve dosya izinleri yerinde kalır.
- [x] **15. Steam kullanıcı yapılandırmalarının yaması.** Kullanıcılar taranır; yalnızca Project Zomboid launch seçenekleri değiştirilir; eksik/çift/anlaşılmaz hedef reddedilir.
- [x] **16. İlk orijinal `.backup` dosyaları.** JSON, betik/BAT ve yamalanan Steam yapılandırmaları için oluşturulur; mevcut Go yedekleri değiştirilmez. Yedekler de rollback kapsamındadır.
- [x] **17. Doğrulama ve atomik yazma.** Bütün değişiklikler ilk yazmadan önce hazırlanır; aynı dizinde geçici dosya, sync ve atomik replacement kullanılır.
- [x] **18. Hata durumunda geri alma.** Değiştirilen/eski/silinen dosyalar ve sonradan oluşan yedekler geri alınır; bütün geri alma hataları raporlanır.
- [x] **19. Çökme kurtarma.** Oyun dizininde `.zombiebuddy-transaction.json`; işlem başında yazılır, başarılı commit sonrası silinir. Sonraki kurulum/kaldırma/diagnostics işlemi, Steam kapandıktan sonra yarım kalan yazmaları geri alır.
- [x] **20. Süreçler arası kurulum kilidi.** Steam ve oyun dizinlerinde OS dosya kilitleri; ikinci süreç testi mevcut. Kilit dosyası silinmez, kilit OS tarafından bırakılır.
- [x] **21. Steam kapatma ve yeniden başlatma.** Windows/Linux/macOS adaptörleri; kapatma sonrası yeniden keşif ve snapshot; başarıdan veya tamamlanmış rollback'ten sonra başlatma.
- [x] **22. Kaldırma hatası.** Payload silinirken root ve iç içe JSON `vmArgs` içinde kalan mod bayrakları da temizlenir; BAT yorumları ve kullanıcının diğer seçenekleri korunur.
- [x] **23. Diagnostics manuel yamaları.** Aynı platform seçimi, doğrulama, yedek, kilit ve işlem/kurtarma mekanizmasına bağlandı. Birden fazla kullanıcıdan sonraki geçersiz yapılandırma önceki kullanıcıya yazma yapılmasını önler.
- [x] **24. Diagnostics görünümü ve geçmişi.** Steam/oyun/Library/config/dosya/branch durumu; oyun yokken kısmi rapor; uygulama yeniden başlatıldığında okunabilen log (2.000 kayıt ve 2 MiB sınırı); kalıcı log temizleme.
- [x] **25. Eski etkileşimler.** F5 ve görünür diagnostics düğmesi; Escape; sağ tık ve klavye/dokunma ile branch eylemleri; yazar bağlantıları.
- [x] **26. UI işlem durumu.** İki branch birlikte kilitlenir; indirme beklemesi iptal edilebilir; 1–8 adımları yüzdeye çevrilir; bitiş/hatada eski ilerleme mesajı ekranda kalmaz.
- [x] **27. Üretim varlıkları ve izinleri.** Logo Vite import'u ile paketlenir; platform ikonları eklendi; native URL izni yalnızca üç sabit proje sayfasına daraltıldı.
- [x] **28. Üç platform CI / uygulama dağıtımı yapılandırması.** [Workflow](../../.github/workflows/installer2.yml): native testler ve lint sonrasında doğrudan Windows EXE, Linux çalıştırılabilir dosyası ve macOS APP ZIP'i toplar; isteğe bağlı taslak prerelease destekler. Windows/Linux bundling kapalı, macOS hedefi yalnızca APP. Önceki üç başarılı CI işi yeni dağıtım ayarlarının doğrulaması değildir; güncel commit'in CI ve native açılış kabulü 30. maddede izlenir.
- [x] **29. Regresyon testleri.** Saf patcher, fixture, üç platform × iki branch, tekrar kurulum/kaldırma, bozuk girdi, yedek koruma, yarım işlem kurtarma, ikinci süreç kilidi ve UI testleri.

## Bilinçli davranış farkları

- Steam zorla sonlandırılmak yerine 30 saniyeye kadar normal kapatılır; kapanmıyorsa hiçbir yama yazılmadan hata verilir.
- Snapshot Steam'in son yazmaları bittikten sonra alınır. Başarılı kurulumdan sonraki Steam başlatma hatası kurulumu geri almaz.
- Bütün yamalar yazmadan önce doğrulanır. Eksik `LaunchOptions` için Go'daki hata davranışı sürer.
- Shell shebang'i ilk satırda kalır; mevcut çalıştırma izinleri korunur. Kaldırma, eski yedekleri körlemesine geri yükleyip yeni kullanıcı ayarlarını silmek yerine yalnızca modun seçeneklerini temizler.
- Yarım rollback veya bozuk kurtarma kaydı varsa Steam kapalı bırakılır ve hata açıkça raporlanır. Journal, orijinal içeriği ve izinleri korur; yalnızca keşfedilen yönetilen hedeflere geri yükleme kabul edilir.
- Go uygulaması yeni kilidi kullanmaz; eski ve yeni sürümü aynı kurulum üzerinde eşzamanlı çalıştırmayın.
- VDF quoted/unquoted token, BOM, satır/blok yorumları ve escape'ler desteklenir. `#include`/`#base` ve koşullu VDF direktifleri anlaşılmaz girdi olarak reddedilir.

## Son doğrulama (2026-11-04, Windows x64)

Diagnostics takip düzeltmeleri (2026-11-04):

- [x] Yenileme isteklerini tekilleştirmek ve log temizlemeden önce başlayan yanıtların silinen kayıtları geri getirmesini önlemek. Bekleyen backend yanıtıyla regresyon testi eklendi.
- [x] Yerel işlem/log temizleme hatalarını otomatik yenilemede korumak. Yerel pencere kayıtları backend geçmişiyle birlikte gösterilir; başarılı log temizleme ikisini de temizler. Hata ve sonraki yenileme senaryoları regresyon testleriyle doğrulandı.

- [x] Diagnostics dosya raporu ve durum/log isteklerinin hatalarını ayrı ele almak. Dosya raporu alınamasa da geçmiş yüklenir; durum isteği başarısızsa dosya raporu hata mesajıyla birlikte gösterilir. Sonraki başarılı yenilemede görünüm toparlanır.
- [x] Otomatik log yenilemesinde okuma konumunu korumak. Kullanıcı en alttaysa yeni kayıtlar takip edilir; eski kayıtları okuyorsa kaydırma konumu korunur. Değişmeyen içerik yeniden oluşturulmaz; metin seçimi korunur.

Bu düzeltmeler sonrasında `npm run test:ui`: 24 test geçti; `npm run build`: geçti. Aşağıdaki önceki native kabul kanıtları bu değişikliklerin native doğrulaması değildir.

- `npm run build`: geçti; logo üretim varlıklarına dahil.
- `npm run test:ui`: 15 test geçti.
- `cargo fmt --all -- --check`: geçti.
- `cargo test --workspace --locked`: 33 Rust testi geçti (15 çekirdek, 2 native log, 16 fixture/entegrasyon).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: geçti.
- Önceki `npm run tauri build`: Windows release uygulaması üretildi; aynı çalışmadaki setup çıktısı artık dağıtım hedefi değildir.
- `npm run test:native:windows`: geçti; iki branch, logo, yerleşim, pencere boyutu/odak/stilleri, menü klavyesi, F5/Escape ve diagnostics yeniden açılışı doğrulandı. Hiçbir install/uninstall/patch komutu çağrılmadı; Steam kapatılmadı.
- Opt-in `native_windows_steam_shutdown_restart`: geçti; gerçek Steam üretim adaptörüyle yaklaşık 2,74 saniyede normal kapandı, kapalıyken tekrar kapatma etkisiz kaldı, yeniden başlatıldıktan yaklaşık 1,08 saniye sonra Steam süreci görüldü. Test Steam'i açık bıraktı; kurulum/yama/Workshop/indirme/ayar komutu çağrılmadı. Bu sonuç branch'ten bağımsız süreç kabulüdür; oyun veya config flush kabulü değildir.
- Linux/macOS native testleri bu Windows ortamında çalıştırılmadı; aşağıdaki 30. maddede kaydedilen uzak CI çalışmasında kendi native runner'larında geçti. Önceki 2026-11-03 denemesinde Rust standard library indirmesi ilerlemediği için çapraz hedef kontrolü çalıştırılamamıştı. CI testleri ve derleme, dağıtılan uygulamanın native açılışı veya gerçek Steam/oyun kabulünün yerine geçmez; açık doğrulamalar 30–32. maddelerdedir.

## Tamamlanan Windows native kabul işleri

- [x] Windows WebView2: release uygulamasının açılışı, ana görünüm/varlıklar, diagnostics yükleme ve kapatıp yeniden açma için otomatik smoke kontrolü.
- [x] Windows: release uygulamasında native WebView2 ile odak, 680 × 540 sabit pencere boyutu, boyutlandırma/büyütme/küçültme engeli, menü tuşları (Tab/yön/Home/End/Enter/Space/Escape), F5 diagnostics açma-yeniden açma ve uygulamadan çıkış doğrulandı. Yazar bağlantısı alt durum panelinin hemen üstünde ortalı ve görünür; düzen taşmıyor.
- [x] Native Windows kabul testini `npm run test:native:windows` ile tekrar çalıştırılabilir hale getirmek ve kullanımını [test planına](../tests/README.md#windows-native-ui-kabulü) eklemek. 2026-11-04 tarihinde yeni release build üzerinde geçti.

## Kalan işler (sırayla)

**2026-11-05 güncel kapsam:** gerekli Steam config değişiklikleri belirli hedefler ve bağımsız geri yüklenebilir yedeklerle izinlidir. Gerçek oyun dosyaları/kurulum/açılış ve ayrı manuel masaüstü kabulü bu görev kapsamı dışındadır; çalıştırılmadı, başarılı sayılmaz ve görev engeli değildir. Linux/macOS için native GitHub Actions kabul edilen ortamdır. Aşağıdaki tarihsel sonuçlar ait oldukları commit için korunur; yeni kontrolün yokluğu başarı sayılmaz. Public yayın ayrı karardır.

2026-11-04 kapsamı: Windows'ta yapılabilen geliştirme ve izole süreç/başlatıcı kabul işleri sırayla tamamlanır; Linux/macOS deneysel destek olarak kalır. Linux/macOS manuel masaüstü/oyun kabulleri Windows işlerini engellemez ve geçilmiş sayılmaz. 1–29'daki Go işlev eşliği uygulanmıştır; kalan kutular yeni bir özellik listesi değil, kabul ve yayın işidir.

### 30. Üç platformda CI ve doğrudan uygulama doğrulaması

- [x] [Installer 2 workflow'unu](../../.github/workflows/installer2.yml) içeren commit'i GitHub'a göndermek ve aynı commit için Windows x64, Linux x64 ve Apple Silicon macOS CI işlerinin başarıyla bittiğini kaydetmek. 2026-11-04: `migration/complete-go-parity` dalındaki `b649174f625f5e830c34f955c418e00a0933a2c0` için [CI çalışması](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41798650750) başarılı. Bu kanıt kayıtlı commit'e aittir; çalışma dizinindeki commit edilmemiş değişiklikleri kapsamaz.
- [x] Güncel doğrudan uygulama dağıtımı ayarlarıyla aynı commit için üç CI işini doğrulamak; Windows EXE, Linux çalıştırılabilir dosyası ve APP ZIP artifact'larını kaydetmek. 2026-11-04: `1a46881bceb44d005c3583bb3021f62b14c0ac25`, [CI çalışması](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41804893095) başarılı; iş kanıtları aşağıda. Setup paketi üretilmedi.
- [x] Windows x64'te doğrudan çalıştırılabilir EXE'yi setup olmadan açmak; ana görünüm, diagnostics, klavye ve çıkışı doğrulamak. 2026-11-04: yerel release EXE ve CI'dan indirilen `ZombieBuddyInstaller-windows-amd64.exe` üzerinde [native UI kabulü](../tests/README.md#windows-native-ui-kabulü) geçti. Pencere 680 × 540; layout, tuşlar, diagnostics yeniden açma ve uygulama çıkışı doğrulandı. Windows WebView2 runtime gereklidir.
- [ ] Linux x64 native Actions runner'ında WebDriver içermeyen production executable ile görünür pencere startup smoke ve mevcut WDIO suite'ini aynı kaynak commit'inde doğrulamak. Production smoke henüz uygulanmadı/çalıştırılmadı. Ayrı manuel masaüstü kabulü kapsam dışıdır.
- [ ] Apple Silicon macOS native Actions runner'ında ZIP'ten çıkarılan production APP ile görünür pencere startup smoke ve mevcut WDIO suite'ini aynı kaynak commit'inde doğrulamak. Production smoke henüz uygulanmadı/çalıştırılmadı. Ayrı manuel masaüstü kabulü kapsam dışıdır.
- [x] **Deneysel cross-platform kontrolü:** Linux/macOS native runner'larında test-only WDIO feature ile Tauri/WebKit uygulamasını açmak; ana layout ve branch menülerini, diagnostics ikinci penceresini, kapanışta ana pencere odağını ve diagnostics tekrar açılışını doğrulamak. Test-only yetkiler release build'e alınmaz; workflow logları ve screenshot artifact'ları yükler. Windows'ta aynı suite yerel test uygulamasında geçti. 2026-11-04, commit `93501839b3e0f6e77d40586786ac1c4e74f056ef`: [üç platform Actions çalışması](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41897827574) başarılı; Linux [native UI işi](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41897827574/job/126811984090) ve Apple Silicon macOS [native UI işi](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41897827574/job/126811984055) geçti, screenshot artifact'ları yüklendi.

30. görevde Linux ve Apple Silicon macOS native UI testi ile Actions adımları eklendi ve `93501839b3e0f6e77d40586786ac1c4e74f056ef` commit'inde başarıyla doğrulandı. Test uygulaması indirilebilir release binary ile aynı uygulama kodudur; yalnızca embedded WebDriver capability/eklenti için ayrı feature/config ile derlenir. Yerel Windows WebView2'de suite de geçti.

Deneysel tamamlanma ölçütü: aynı commit'in üç başarılı CI işi; Linux ve macOS native UI suite sonucu başarılı ve screenshot artifact'ları yüklü. Bu, test konfigürasyonlu uygulama derlemesini native açar; indirilebilir release dosyasının birebir açılışı veya Go release akışını değiştirme onayı değildir. Güncel görevde bu production açılış farkı native Actions runner'ında görünür pencere smoke ile kapatılacaktır; ayrı manuel masaüstü gerekmez. Aracın kendisinin sisteme kurulması veya setup testi gerekmez. Windows fixture testleri Linux/macOS native UI testi yerine geçmez.

Güncel doğrudan uygulama CI kanıtları (`1a46881bceb44d005c3583bb3021f62b14c0ac25`, 2026-11-04; deneysel UI suite'in sonraki `93501839b3e0f6e77d40586786ac1c4e74f056ef` doğrulaması aşağıda kayıtlıdır):

| Hedef | Native CI sonucu | Üretilen indirilebilir dosya |
| --- | --- | --- |
| Windows x64 | [Başarılı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41804893095/job/126489361684) | `ZombieBuddyInstaller-windows-amd64.exe`; indirilen dosya native UI testinden geçti |
| Linux x64 | [Başarılı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41804893095/job/126489362305) | `ZombieBuddyInstaller-linux-amd64` |
| macOS arm64 | [Başarılı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41804893095/job/126489362202) | `ZombieBuddyInstaller-macos-arm64.zip`; ZIP içinde `ZombieBuddy Installer.app` doğrulandı |

Üç native işte build, UI testleri, Rust biçim kontrolü, workspace testleri ve Clippy geçti. `9350183` workflow run'ında Linux/macOS native UI suite'leri ve screenshot artifact yüklemeleri de başarılıdır. Windows artifact'ı indirildi ve gerçek WebView2 penceresinde açıldı; Linux/macOS production artifact görünür pencere smoke'u güncel görevde native Actions üzerinde henüz bekliyor; ayrı manuel masaüstü kapsam dışıdır. Taslak release adımı çalıştırılmadı; bir sürüm yayımlanmadı.

Önceki dağıtım ayarlarının tarihsel CI kanıtları (`b649174f625f5e830c34f955c418e00a0933a2c0`, 2026-11-04; güncel indirme biçimlerinin kabulü değildir):

| Hedef | Native CI sonucu | Üretilen paket / yüklenen artifact |
| --- | --- | --- |
| Windows x64 | [Başarılı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41798650750/job/126468731937) | NSIS / `installer2-windows-amd64` |
| Linux x64 | [Başarılı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41798650750/job/126468732265) | DEB + AppImage / `installer2-linux-amd64` |
| macOS arm64 | [Başarılı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41798650750/job/126468732291) | APP + DMG üretildi; DMG / `installer2-macos-arm64` |

Bu eski çalışmada frontend build, UI testleri, Rust biçim kontrolü, workspace testleri, Clippy ve o tarihteki dağıtım çıktılarının üretimi geçti; artifact yüklemeleri doğrulandı. İlk CI denemesi Linux/macOS'ta BAT fixture testinin sabit CRLF varsayımını ortaya çıkardı. Test, Git checkout ayarından bağımsız olarak LF ve CRLF girdilerinde satır sonlarının korunmasını ve tekrar yamanın etkisizliğini doğrulayacak şekilde düzeltildi. Taslak release adımı koşulu gereği atlandı; release yayımlanmadı. Güncel EXE/Linux dosyası/APP ZIP dağıtımının CI ve native açılış kabulü ayrıca yapılacaktır; eski setup çıktıları artık istenmez.

### 31. Gerçek Steam indirme ve süreç davranışı

- [x] **Windows'ta yapılabilen izole süreç senaryoları:** üretimdeki kurulum akışını `InstallRuntime` sınırında izole etmek; LTS ve Beta için eksik abonelik, kısmi indirme/tamamlanma, iptal ve indirme zaman aşımında sıfır yazma; kapanış sırasında yazılan son config ve yeni kullanıcıyı yeniden keşfetme; kapanmayan istemci ve oyun açılma yarışında sıfır yazma; sekiz yazma noktasında rollback ve yeniden başlatma; rollback'te son config'i koruma; restart hatasını raporlama; bozuk journal'da kapalı kalma. [Senaryo testleri](../crates/installer-core/src/install_scenarios.rs) Windows'ta geçti. Config flush/restart gerçek, ayrı bir test sürecinde yapılır; Steam taklidi gerçek Steam istemcisinin veya oyunun kabulü değildir. Tüm dosyalar geçici dizindedir. `cargo test --workspace --locked`, biçim kontrolü, Clippy, frontend build ve 24 UI testi de geçti.

Branch'ten bağımsız süreç kontrolü:

- [x] **Windows'ta gerçek Steam'i üretim adaptörüyle normal kapatıp yeniden açan opt-in testi eklemek ve çalıştırmak.** 2026-11-04 tarihinde geçti; [native test](../crates/installer-core/src/platform.rs) normal test/CI koşusunda atlanır, hata durumunda yeniden açmayı dener. [Çalıştırma komutu ve kapsam](../tests/README.md#windows-native-steam-süreç-kabulü-opt-in). O tarihteki kullanıcı açma/kapatma izni ve test dışı verilere dokunmama sınırı [proje kurallarına](../AGENTS.md) işlendi. Steam/oyun dosyalarına doğrudan yazılmadı.

Güncel Windows tekrar kabulü (2026-11-04): gerçek Steam 3,16 saniyede normal kapandı, 1,06 saniyede yeniden başladı; test sonunda açık bırakıldı. Kurulum/yama/Workshop/indirme/ayar komutu çağrılmadı. Gerçek config flush ve gerçek oyun kabulü bu süreç testinin kapsamında değildir.

Güncel görevde sonraki adım: Windows gerçek Steam config roundtrip'ini [bağımsız disk yedeği ve kurtarma kapısına](../tests/README.md#windows-steam-config-hedef-yedek-ve-kurtarma-kapısı) uyarlamak; yalnız tam adlı opt-in testi çalıştırıp patch/backup/idempotency, ilgisiz seçeneklerin korunması, cleanup ve Steam son durumunu kanıtlamak. Kod güvenliği uygulandı; 2026-11-05 tam adlı opt-in config testi geçti. İki config/backup hedefi, mevcut managed flag durumundan geçici scoped baseline ve zorunlu gerçek patch yazımı, normal kapanış 2,667 sn, scoped patch/ilk backup/idempotency, ilgisiz config byte'larının korunması ve orijinal varlık/içerik/SHA-256/readonly/DACL cleanup'ı restart öncesi doğrulandı; Steam 1,061 sn'de yeniden açıldı ve açık bırakıldı. Log: `installer2/target/acceptance/windows-steam-config-step2-final.log` (repo köküne göre); özel recovery manifest/kopyaları Steam dışındaki kalıcı dizinde tutulur, tam yol logdadır. Steam EXE FileVersion `11.15.70.27`, ProductVersion `01.00.00.02`. Kaynak HEAD `aed9b22` + henüz commit edilmemiş ikinci adım değişiklikleridir; `c7662e9` artifact'ı değildir. Dört regression testi, `cargo test -p installer-core --locked` (26 geçti, 3 opt-in atlandı), fmt ve workspace all-targets Clippy geçti. Frontend/production uygulama kodu değişmediğinden bu adımda frontend rebuild yapılmadı; Tester kabulü beklenir.

Gerçek Workshop abonelik/indirme branch matrisi, gerçek istemcide kapanmama zaman aşımı ve gerçek kurulum rollback denemeleri mevcut kapsam dışıdır, çalıştırılmadı. İzole süreç senaryolarının sonuçları bu gerçek davranışlara aktarılmaz; bunlar bu görev için zorunlu kabul hücresi değildir.

### 32. İki branch için gerçek oyun kabulü

- [x] **Windows'ta yapılabilen başlatıcı süreç kabulü:** LTS ve Beta'yı izole oyun/Steam dizininde kurmak, yamalanan BAT'ı gerçek `cmd.exe` ile çalıştırmak, alt sürece iletilen JVM seçeneklerini ve payload dosyalarını doğrulamak; kullanıcı düzenlemeleri sonrasında tekrar kurulum, iki yönde branch değişimi ve kaldırma ardından yeniden başlatıcı çalıştırmak. Boşluk ve `&` içeren yollar, ilk backup'lar, ilgisiz oyun/ayarların korunması sınandı. [Windows başlatıcı testi](../tests/tests/integration/windows_launcher.rs) ve tüm workspace testleri/Clippy Windows'ta geçti. Başlatılan alt süreç test probe'udur; gerçek JVM, Project Zomboid veya ZombieBuddy oyun içi kodu çalıştırılmaz; gerçek oyun kabulü bu kanıtla başarılı sayılmaz.

Gerçek oyun kurulum/tekrar kurulum/kaldırma ve oyun/mod açılışının üç platform × iki branch matrisi **kapsam dışı, çalıştırılmadı** olarak korunur. Bu görevde oyun dosyalarına dokunulmaz ve oyun açılmaz. Bu sonuçların yokluğu görevin engeli değildir; izole başlatıcı/fixture başarısı gerçek oyun başarısı sayılmaz.

### 33. İmza politikası (yayın kararı)

- [x] Deneysel Rust adaylarında mevcut Go yayınlarıyla aynı imzasız dağıtımı sürdürmek ve Windows SmartScreen/macOS Gatekeeper uyarılarını kaydetmek. [İmza ve aday politikası](RELEASE.md): Windows Authenticode ve macOS notarization bu deneysel kapsamda uygulanmaz; üç dosya için SHA-256 listesi üretilir. Mevcut Windows EXE imza durumu `NotSigned`; önceki Go workflow'unda da imza adımı yoktur. Bu kayıt imzalı yayın kabulü veya herkese açık yayın kararı değildir.

### 34. Ana yayın akışını değiştirme

- [x] Windows izole kabulü ve Linux/macOS deneysel native UI suite'leri başarılı aday commit'ini, workflow'u ve doğrudan uygulama dosyalarını kaydetmek; indirilen Windows EXE'sini native UI suite'inden geçirmek. `c7662e9e9489a669381ed9315b2f39f0f2fe5612`, [Rust etiket workflow'u](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41903125865) başarılı. Windows x64/Linux x64/macOS arm64 test/build/lint ve Linux/macOS native UI işleri geçti. Aynı run'dan indirilen Windows EXE'si yerel native UI kabulünden geçti (pencere/odak, 24 DOM testinden ayrı gerçek klavye kontrolleri, diagnostics yeniden açma, çıkış).
- [x] [Ana `v*` etiket/release workflow'unu](../../.github/workflows/release.yml) Rust [native build workflow'unu](../../.github/workflows/installer2.yml) yeniden kullanacak şekilde değiştirmek. Go derlemesi kaldırıldı; çıktı Windows EXE/Linux dosyası/APP ZIP ve `SHA256SUMS` içeren taslak prerelease'dir. Actionlint geçti. README artık Rust geliştirme/dağıtımını esas alır. macOS site indirmesi APP ZIP'ini seçer ve önceki Go dosya adıyla uyumludur; sekiz paket/link regresyon testi geçti.
- [x] Taslak release'in üç uygulama dosyası, SHA-256 listesi, aday commit'i ve sürüm adlarını doğrulamak. [Rust aday taslağı](https://github.com/ipatavatsizz/zbinstaller/releases/tag/untagged-75932294f8141d3d5a54): `v26.11.3-rust-candidate.20261104.2`; draft/prerelease, hedef commit `c7662e9e9489a669381ed9315b2f39f0f2fe5612`. Dört varlık (üç uygulama + `SHA256SUMS`) doğrulandı; üç dosya indirilip hash'leri karşılaştırıldı. Windows dosyası native UI testinde kullanılan dosyayla birebir aynı; macOS ZIP içinde APP executable ve `Info.plist` var. Windows imzası `NotSigned`.
- [ ] Installer 2 taslağını herkese açık yayımlamak — mevcut görev kapsamı dışında, ayrı yayın kararı gerekir. Bu çalışma otomatik public yayın yapmaz.

Windows'ta tamamlanan kapsam: 31'in izole süreç senaryoları ve gerçek Steam aç/kapat kabulü, 32'nin izole kurulum/başlatıcı döngüsü, 33'ün deneysel imza politikası ve 34'ün Rust derleme/taslak kabulü. Yerelde 42 workspace Rust testi, 24 UI testi, 8 yayın/link testi, fmt ve Clippy geçti; normal workspace çalışmasında üç gerçek Steam/game opt-in testi atlanır. Bunlardan yalnızca dosya yazmayan Steam süreç testi ayrıca çalıştırıldı ve geçti. Gerçek geliştirici oyununa kurulum yapılmadı. Gerçek oyun/Workshop/config sonuçları bu izole kanıtlarla başarılı sayılmaz.

Doğrulanan aday dosyaları:

| Dosya | SHA-256 |
| --- | --- |
| `ZombieBuddyInstaller-windows-amd64.exe` | `ada99c3558b19c26eee3850e136a6b1a53a42d0d4bd6f436f25882574d50349b` |
| `ZombieBuddyInstaller-linux-amd64` | `b0c310f280fd9d3d46e54effaf075014b4978a5357061a9c50b1b81d59fab0d2` |
| `ZombieBuddyInstaller-macos-arm64.zip` | `9bcc926e7fc75574471727b5bed50c4d032c2682b14e1362fff66a942eb5c8cd` |

## Kabul ortamı ve engeller (2026-11-05)

Kullanıcının güncel onayıyla [kabul prosedürü ve kanıt şablonu](../tests/README.md#gerçek-kabul-çalıştırma-prosedürü-3034) revize edildi. [Proje kuralları](../AGENTS.md), gerekli Steam config işlemlerine belirli hedefler, ilk yazmadan önce kalıcı bağımsız yedek ve doğrulanmış geri yüklemeyle izin verir. İlk belge adımında Steam işlemi yapılmadı; ikinci adımda güvenli hale getirilen config testi yalnız tam adıyla opt-in çalıştırıldı ve geçti. Oyun roundtrip testi ve toplu `--ignored` çalıştırılmadı.

Tarihsel aday: `c7662e9e9489a669381ed9315b2f39f0f2fe5612`, etiket `v26.11.3-rust-candidate.20261104.2`; hash'leri 34. maddede kayıtlıdır. Revize belge adımı başında salt okunur doğrulanan HEAD `aed9b2284a73d6008c1ee2a7ab284dbba615ab38` idi. HEAD ve yeni değişiklikler aday commit'i değildir; eski binary/hash/UI kabulü yeni kaynak commit'ine aktarılmaz.

| Kontrol / kabul ortamı | Güvenlik ve kanıt kapısı | Güncel durum |
| --- | --- | --- |
| Windows x64 gerçek Steam config roundtrip | Tam hedef listesi; normal kapanış sonrası bağımsız disk yedeği; config/orijinal backup varlık-hash-izin geri yükleme doğrulaması; Steam sürümü/son durumu | 2026-11-05 geçti; 31. maddede yerel kanıt, Tester kabulü beklenir |
| Linux x64 native Actions | Aynı kaynak commit; WDIO test-build sonucu ayrı; production executable hash/süreç/görünür pencere/log/screenshot ayrı | Tarihsel WDIO geçti; yeni production smoke uygulanmadı/çalıştırılmadı |
| macOS Apple Silicon native Actions | Aynı kaynak commit; WDIO test-build sonucu ayrı; ZIP/APP hash/süreç/görünür pencere/log/screenshot ayrı | Tarihsel WDIO geçti; yeni production smoke uygulanmadı/çalıştırılmadı |
| Aynı kaynak commit CI ve artifact doğrulaması | Run/job URL; üç dosya ve SHA-256; kaynak/HEAD/tarihsel aday ayrımı; Tester sonucu | Revize değişiklikler için henüz çalıştırılmadı |
| Gerçek oyun ve ayrı manuel masaüstü | Oyun dosyalarına yazılmaz/oyun açılmaz; ek masaüstü makinesi gerekmez | Kapsam dışı, çalıştırılmadı; görev engeli değil |

Sıralı plan: güvenli Windows Steam config testi (yerel geçti, Tester bekleniyor) → Linux/macOS production smoke ve WDIO → aynı kaynak CI/kanıt/sonuç kapatılması. Her adım Tester kabulüne bağlıdır. Ayrılmış oyun sistemi yokluğu bu kapsamda engel değildir. 33'ün imzasız deneysel dağıtım politikası korunur; public yayın ayrı kalır.

## Kabul sonuç kaydı

Her sonuç ilgili kaynak commit'i, UTC ve kanıt bağlantısıyla yazılır. `Çalıştırılmadı` başarı değildir; `Kapsam dışı` bu görevin gerektirmediği ve yapılmamış kontrolü belirtir. İkinci adımda Windows config sonucu eklendi; sonraki Linux/macOS ve CI kontrolleri hâlâ çalıştırılmadı.

| Kontrol | Sonuç | Kanıt / yapılacak kayıt |
| --- | --- | --- |
| Windows gerçek Steam normal aç/kapat | Tarihsel geçti, branch'ten bağımsız | 31. madde; 2026-11-04; config/game roundtrip kanıtı değil |
| Windows bağımsız yedekli config roundtrip | 2026-11-05 geçti; Tester kabulü beklenir | 31. madde; yerel log ve bağımsız recovery manifest; HEAD + ikinci adım değişiklikleri |
| Linux x64 WDIO test-build | Tarihsel geçti | `c7662e9` CI; yeni kaynak commit'inde tekrar kayıt gerekir |
| Linux x64 production artifact pencere smoke | Çalıştırılmadı | Sonraki workflow adımı; süreç/pencere ve artifact hash kanıtı |
| macOS arm64 WDIO test-build | Tarihsel geçti | `c7662e9` CI; yeni kaynak commit'inde tekrar kayıt gerekir |
| macOS arm64 production APP pencere smoke | Çalıştırılmadı | Sonraki workflow adımı; ZIP/APP ve süreç/pencere kanıtı |
| Yeni kaynak commit'i üç platform CI/artifact/hash | Çalıştırılmadı | Final adımda run/job/commit/hash kayıtları |
| Windows x64 LTS/Beta gerçek oyun matrisi | Kapsam dışı, çalıştırılmadı | Gerçek oyun dosyası ve açılış işlemi yapılmaz |
| Linux x64 LTS/Beta gerçek oyun matrisi | Kapsam dışı, çalıştırılmadı | Görev engeli değil; fixture başarıları aktarılmaz |
| macOS arm64 LTS/Beta gerçek oyun matrisi | Kapsam dışı, çalıştırılmadı | Görev engeli değil; fixture başarıları aktarılmaz |
| Ayrı Linux/macOS manuel masaüstü | Kapsam dışı, çalıştırılmadı | Native Actions kabul edilen ortam |
| Public yayın | Kapsam dışı, yapılmadı | Ayrı yayın kararı |

CI kanıtları: `c7662e9e9489a669381ed9315b2f39f0f2fe5612` üç platform test/build/lint ve Linux/macOS deneysel native UI suite'leri için tarihsel başarılıdır; Windows indirilen EXE native kabulü ve taslak hash kontrolü de geçti (34. madde). Bu sonuçlar revize değişikliklerin doğrulaması değildir. Kanıt yolları `acceptance/<tam-kaynak-commit>/<platform>/<UTC>/record.md` şablonuyla, Steam/oyun dışında tutulur; ham config/yedekler özel kalır. İmza politikası [RELEASE.md](RELEASE.md) içindedir. Görev tamamlandığında gerçek oyun veya public yayın tamamlandı iddiası kurulmaz.

## İlgili dosyalar ve kaynaklar

- [Çekirdek](../crates/installer-core/src/lib.rs), [platform seçimi](../crates/installer-core/src/target.rs), [işlem/kurtarma](../crates/installer-core/src/transaction.rs)
- [Üç platform CI](../../.github/workflows/installer2.yml), [mimari](ARCHITECTURE.md), [testler](../tests/README.md)
- Platform birleştirme ve CI yapılandırması için [Tauri configuration](https://v2.tauri.app/develop/configuration-files/#platform-specific-configuration), [Tauri GitHub pipelines](https://v2.tauri.app/distribute/pipelines/github/) ve [native bağımlılıklar](https://v2.tauri.app/start/prerequisites/) esas alındı.
- Süreçler arası kilit [Rust File::try_lock](https://doc.rust-lang.org/stable/std/fs/struct.File.html#method.try_lock) kullanır (Rust 1.89+).
