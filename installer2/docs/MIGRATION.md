# Go → Rust/Tauri geçiş durumu

## Sonuç ve kapsam

Eski Go uygulamasının Windows, Linux ve macOS için gerçekleştirdiği kurulum davranışları Rust çekirdeğine taşındı. Aşağıdaki tablo kod ve otomatik test durumunu gösterir. Bu, üç işletim sisteminde gerçek Steam kurulumunun doğrulandığı anlamına gelmez; yayın kabulü ayrı tutulur.

Önceki incelemeye düzeltme: Go sürümü yalnızca JSON başlatıcısını alt klasörlerde arar. Unix başlatıcısını oyun kökünde `ProjectZomboid64.sh`, `ProjectZomboid64.command`, `start-server.sh` sırasıyla seçer. Yeni sürüm aynı seçimi uygular.

## Tek tek iş listesi

| # | Özellik / yapılacak iş | Durum ve uygulama |
| --- | --- | --- |
| 1 | Mevcut Git değişikliklerini korumak | `migration/complete-go-parity` dalı; başlangıç değişiklikleri ayrı commit'te korundu. |
| 2 | Windows Steam Registry keşfi | HKCU/HKLM, 32/64 bit görünümler, `SteamPath`/`InstallPath` korunur. |
| 3 | Linux Steam keşfi | `.steam/steam`, `.local/share/Steam`, küçük harfli alternatif, Flatpak ve Snap yolları taşındı. |
| 4 | macOS Steam keşfi | Kullanıcı Steam verisi ve sistem/kullanıcı Steam.app çalıştırıcıları taşındı. |
| 5 | Ek Steam kütüphaneleri ve oyun manifestleri | Yeni/eski `libraryfolders.vdf` biçimleri, özel oyun klasör adı ve varsayılan klasör desteği; bozuk oyun manifestinde diğer kütüphanelere devam edilir. |
| 6 | LTS/Beta Workshop kimlikleri ve sürümleri | Aynı kimlikler, `VERSION` okuma, eksik sürüm ile abonelik ayrımı korunur. |
| 7 | Workshop sayfası açma | Sabit Workshop kimlikleri Steam istemcisine iletilir; aynı anda kurulumla çalışamaz. |
| 8 | İndirmenin tamamlanmasını bekleme | 60 saniye zaman aşımı, düzenli yeniden keşif ve iptal; hazır dosyalar ile varsa ACF kaydı aynı kütüphaneden doğrulanır. Eski sürümdeki ACF bulunmayan yerel payload desteği korunur. |
| 9 | Platforma uygun payload kopyalama | Windows DLL, Linux SO, macOS DYLIB ve ortak JAR dosyası; boş payload reddedilir. |
| 10 | Platforma uygun başlatma bayrakları | Windows `-agentlib:zbNative --`; Unix `-javaagent:ZombieBuddy.jar --`. |
| 11 | JSON başlatıcısının alt klasörlerde bulunması | Kök öncelikli ve büyük/küçük harfe duyarsız arama; directory symlink'leri izlenmez. |
| 12 | JSON yaması | Yalnızca root `vmArgs` kurulum sırasında değiştirilir; diğer ayarlar korunur; tekrar kurulum bayrağı çoğaltmaz. |
| 13 | Windows BAT yaması | Mevcut `_JAVA_OPTIONS` ve satır sonları korunur; mevcut çıplak native bayrağı normalize edilir. |
| 14 | Linux/macOS betik yaması | Üç eski başlatıcı adı desteklenir; `_JAVA_OPTIONS` korunur; shebang ilk satırda ve dosya izinleri yerinde kalır. |
| 15 | Steam kullanıcı yapılandırmalarının yaması | Kullanıcılar taranır; yalnızca Project Zomboid launch seçenekleri değiştirilir; eksik/çift/anlaşılmaz hedef reddedilir. |
| 16 | İlk orijinal `.backup` dosyaları | JSON, betik/BAT ve yamalanan Steam yapılandırmaları için oluşturulur; mevcut Go yedekleri değiştirilmez. Yedekler de rollback kapsamındadır. |
| 17 | Doğrulama ve atomik yazma | Bütün değişiklikler ilk yazmadan önce hazırlanır; aynı dizinde geçici dosya, sync ve atomik replacement kullanılır. |
| 18 | Hata durumunda geri alma | Değiştirilen/eski/silinen dosyalar ve sonradan oluşan yedekler geri alınır; bütün geri alma hataları raporlanır. |
| 19 | Çökme kurtarma | Oyun dizininde `.zombiebuddy-transaction.json`; işlem başında yazılır, başarılı commit sonrası silinir. Sonraki kurulum/kaldırma/diagnostics işlemi, Steam kapandıktan sonra yarım kalan yazmaları geri alır. |
| 20 | Süreçler arası kurulum kilidi | Steam ve oyun dizinlerinde OS dosya kilitleri; ikinci süreç testi mevcut. Kilit dosyası silinmez, kilit OS tarafından bırakılır. |
| 21 | Steam kapatma ve yeniden başlatma | Windows/Linux/macOS adaptörleri; kapatma sonrası yeniden keşif ve snapshot; başarıdan veya tamamlanmış rollback'ten sonra başlatma. |
| 22 | Kaldırma hatası | Payload silinirken root ve iç içe JSON `vmArgs` içinde kalan mod bayrakları da temizlenir; BAT yorumları ve kullanıcının diğer seçenekleri korunur. |
| 23 | Diagnostics manuel yamaları | Aynı platform seçimi, doğrulama, yedek, kilit ve işlem/kurtarma mekanizmasına bağlandı. Birden fazla kullanıcıdan sonraki geçersiz yapılandırma önceki kullanıcıya yazma yapılmasını önler. |
| 24 | Diagnostics görünümü ve geçmişi | Steam/oyun/Library/config/dosya/branch durumu; oyun yokken kısmi rapor; uygulama yeniden başlatıldığında okunabilen, sınırlı boyutlu log; kalıcı log temizleme. |
| 25 | Eski etkileşimler | F5 ve görünür diagnostics düğmesi; Escape; sağ tık ve klavye/dokunma ile branch eylemleri; yazar bağlantıları. |
| 26 | UI işlem durumu | İki branch birlikte kilitlenir; indirme beklemesi iptal edilebilir; 1–8 adımları yüzdeye çevrilir; bitiş/hatada eski ilerleme mesajı ekranda kalmaz. |
| 27 | Üretim varlıkları ve izinleri | Logo Vite import'u ile paketlenir; platform ikonları eklendi; native URL izni yalnızca üç sabit proje sayfasına daraltıldı. |
| 28 | Üç platform CI / paketleme | `installer2.yml`: native Windows, Linux ve macOS testleri, lint ve paketleme; NSIS, DEB/AppImage, APP/DMG ayarları; isteğe bağlı taslak prerelease. |
| 29 | Regresyon testleri | Saf patcher, fixture, üç platform × iki branch, tekrar kurulum/kaldırma, bozuk girdi, yedek koruma, yarım işlem kurtarma, ikinci süreç kilidi ve UI testleri. |
| 30 | Release geçişi kabulü | Aşağıdaki gerçek ortam kontrolleri tamamlanana kadar Go etiket/release akışı korunur. |

## Bilinçli davranış farkları

- Steam zorla sonlandırılmak yerine 30 saniyeye kadar normal kapatılır; kapanmıyorsa hiçbir yama yazılmadan hata verilir.
- Snapshot Steam'in son yazmaları bittikten sonra alınır. Başarılı kurulumdan sonraki Steam başlatma hatası kurulumu geri almaz.
- Bütün yamalar yazmadan önce doğrulanır. Eksik `LaunchOptions` için Go'daki hata davranışı sürer.
- Shell shebang'i ilk satırda kalır; mevcut çalıştırma izinleri korunur. Kaldırma, eski yedekleri körlemesine geri yükleyip yeni kullanıcı ayarlarını silmek yerine yalnızca modun seçeneklerini temizler.
- Yarım rollback veya bozuk kurtarma kaydı varsa Steam kapalı bırakılır ve hata açıkça raporlanır. Journal, orijinal içeriği ve izinleri korur; yalnızca keşfedilen yönetilen hedeflere geri yükleme kabul edilir.
- Go uygulaması yeni kilidi kullanmaz; eski ve yeni sürümü aynı kurulum üzerinde eşzamanlı çalıştırmayın.
- VDF quoted/unquoted token, BOM, satır/blok yorumları ve escape'ler desteklenir. `#include`/`#base` ve koşullu VDF direktifleri anlaşılmaz girdi olarak reddedilir.

## Doğrulama (2026-11-03, Windows x64)

- `npm run build`: geçti; logo üretim varlıklarına dahil.
- `npm run test:ui`: 12 test geçti.
- `cargo fmt --all -- --check`: geçti.
- `cargo test --workspace --locked`: 32 Rust testi geçti (15 çekirdek, 1 native log, 16 fixture/entegrasyon).
- `cargo clippy --workspace --all-targets -- -D warnings`: geçti.
- `npm run tauri build`: Windows release uygulaması ve NSIS paketi üretildi.
- Native Windows WebView2 smoke kontrolü: iki branch, logo, activity alanı ve diagnostics kontrolleri yüklendi; diagnostics native komutla kapanıp yeniden açıldı. Hiçbir install/uninstall/patch komutu çağrılmadı; Steam kapatılmadı.
- Linux/macOS Rust standard library indirmesi bu ortamda ilerlemedi (iki indirme dosyası 0 bayt); çapraz hedef kontrolü çalıştırılamadı. Yerel CI işleri hazırdır, sonuçları bu çalışma sırasında alınmamıştır.

## Yayın kabulü: dış ortamda kalan işler

- [x] Windows WebView2: release uygulamasının açılışı, ana görünüm/varlıklar, diagnostics yükleme ve kapatıp yeniden açma için otomatik smoke kontrolü.
- [ ] Windows: kullanıcı etkileşimiyle odak, yeniden boyutlandırma ve klavye kullanılabilirliği kabulü (F5/Escape UI testlerinde kapsanır).
- [ ] Ayrı bir Steam/oyun ortamında iki branch'in gerçek kurulumu, tekrar kurulumu, kaldırılması ve oyunun çalıştırılması.
- [ ] Steam'in gerçek indirme, kapatma, son config flush ve yeniden başlatma davranışı.
- [ ] Linux ve Apple Silicon macOS makinelerinde native CI sonuçları, paket açma ve gerçek Steam/oyun kabulü.
- [ ] Yayın için imza/notarization isteniyorsa sertifika, kimlik ve secret'ların sağlanması (Go sürümünün özelliği değildir).
- [ ] Kabul sonuçlarını kaydettikten sonra ana etiket/release akışını Installer 2'ye geçirmek.

Gerçek Steam/oyun dosyaları otomatik testlerle değiştirilmez. Üç hedefin dosya düzenlerinin Windows üzerinde fixture testlerinden geçmesi, Linux/macOS üzerinde native doğrulama yerine geçmez.

## İlgili dosyalar ve kaynaklar

- [Çekirdek](../crates/installer-core/src/lib.rs), [platform seçimi](../crates/installer-core/src/target.rs), [işlem/kurtarma](../crates/installer-core/src/transaction.rs)
- [Üç platform CI](../../.github/workflows/installer2.yml), [mimari](ARCHITECTURE.md), [testler](../tests/README.md)
- Platform birleştirme ve CI yapılandırması için [Tauri configuration](https://v2.tauri.app/develop/configuration-files/#platform-specific-configuration), [Tauri GitHub pipelines](https://v2.tauri.app/distribute/pipelines/github/) ve [native bağımlılıklar](https://v2.tauri.app/start/prerequisites/) esas alındı.
- Süreçler arası kilit [Rust File::try_lock](https://doc.rust-lang.org/stable/std/fs/struct.File.html#method.try_lock) kullanır (Rust 1.89+).
