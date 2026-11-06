# Uygulama dağıtımı ve doğrulama

## Dağıtım ve yayın politikası

Araç kendisini sisteme kurmaz: Windows x64 EXE, Linux x64 çalıştırılabilir dosyası ve Apple Silicon macOS APP içeren ZIP indirilip doğrudan açılır. Setup kurucusu, sistem paketi veya DMG dağıtılmaz. Windows WebView2, Linux WebKitGTK 4.1 ve masaüstü bağımlılıkları gerektirir. Linux/macOS desteği deneyseldir; Intel macOS bu çıktılara dahil değildir.

Deneysel uygulamalar imzasız dağıtılır; Windows Authenticode ve macOS notarization uygulanmaz. SmartScreen/Gatekeeper uyarıları görülebilir. Native CI doğrulaması, bu son kullanıcı uyarı akışlarının veya tüm masaüstü dağıtımlarının testi değildir.

Installer kabulü [test ölçütlerine](../tests/README.md#installer-kabul-ölçütleri) dayanır: doğru platform payload ve başlatma seçenekleri, idempotent tekrar kurulum, ilgisiz ayarları koruyan kaldırma, ilk yedekler ve rollback/kurtarma. Değişiklikler oyun kapalıyken yapılır. Project Zomboid'i veya modu oyun içinde başlatmak installer kabulü ya da public yayın için önkoşul değildir. Testler geliştiricinin gerçek oyun dosyalarına yazmaz.

Yeni sürüm/adayı hazırlarken aynı kaynak commit'inde frontend build/UI/yayın-link testleri, Rust fmt/workspace testleri/Clippy ve üç native derleme doğrulanır. Linux/macOS'ta indirilen production dosyasının görünür ve boyanmış pencere smoke'u ile ayrı WebDriver feature build'indeki etkileşim testleri kaydedilir. Dosya adları ve indirilen dosyaların SHA-256 değerleri kaynak/run kimliğiyle birlikte tutulur. Test derlemesi ile production dosyası kanıtı birbirine aktarılmaz; her yeni aday kendi kanıtını gerektirir.

[Native workflow](../.github/workflows/installer2.yml) üç uygulamayı toplar; isteğe bağlı taslak prerelease oluşturabilir. [Etiket workflow'u](../.github/workflows/release.yml) aynı Rust doğrulamasını kullanır ve üç dosya ile `SHA256SUMS` içeren taslak hazırlar. Taslak hazırlama public yayın değildir; public yayın ayrı bir karardır. Bu kayıttaki doğrulama sırasında yeni tag/taslak/public release oluşturulmadı; aşağıdaki run'ın draft-release adımı atlandı.

## Kök yerleşimin yerel kontrolü

2026-11-06 tarihinde commit edilmemiş kök yerleşimi çalışma ağacında frontend build, 24 UI testi, 22 yayın testi, Rust fmt, `cargo test -p installer-core --locked` (32 başarılı, 3 opt-in test atlandı) ve workspace/all-targets Clippy geçti. Göreli belge bağlantıları da doğrulandı. Native UI CI yolları artık Tauri `mainBinaryName` değerinden türetiliyor; yerel native test yönergesindeki çıktı yolları derleme komutuyla eşleştirildi. Bu değişikliklerle üç platform native CI veya production artifact pencere kontrolleri yeniden çalıştırılmadı; aşağıdaki kaynak/artifact kaydı tarihsel kimliğini korur.

## Son doğrulanan kaynak ve dosyalar

Test edilen kaynak commit'i **`b47578f3262b46ba4421abb0030c9731cb4f634a`**, [Actions run `42231971879`, attempt 2](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/attempts/2), 2026-11-06, sonuç `success`. Frontend build/UI/yayın-link testleri, Rust fmt/workspace testleri/Clippy ve üç native derleme geçti; hiçbir kontrol atlanmadı. Bu kayıt taşınmadan önceki `installer2/` kaynak yerleşimine aittir. Sonraki belge ve klasör düzeni değişiklikleri bu test edilen kaynak commit'inden ayrıdır; bu koşu yeni kök yerleşimin üç platform CI doğrulaması olarak sunulmaz. Tablodaki artifact kimlikleri korunur.

Windows ve macOS işleri attempt 1'de başarılı oldu ve attempt 2'de bu sonuçlar yeniden kullanıldı. İlk Linux işi, Azure Ubuntu aynasından WebKitGTK paketi indirilirken 20 dakikayı aşan aktarım gecikmesi nedeniyle iptal edildi; uygulama build/test adımlarına henüz ulaşmamıştı. Attempt 2'de yalnız iptal edilen Linux işi yeniden çalıştırıldı ve bütün kontrolleri geçti. İlk denemenin iptal/log kaydı korunur; kaynak veya workflow değiştirilmedi.

| Hedef / native iş | İndirilebilir dosya | SHA-256 |
| --- | --- | --- |
| [Windows x64](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/job/127951179016) | [`ZombieBuddyInstaller-windows-amd64.exe`](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/artifacts/13509243262) | `627681c60448e3c6f81d2f62165be230d9f54cc7383bbe9aacb8d52e0ccf5538` |
| [Linux x64](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/job/127951177045) | [`ZombieBuddyInstaller-linux-amd64`](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/artifacts/13512580632) | `89d18cf4d7ff1958c30e42bea646abf272c51c3133eb0fc275b898eb5b832320` |
| [macOS arm64](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/job/127951241268) | [`ZombieBuddyInstaller-macos-arm64.zip`](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/artifacts/13510696921) | `134f6b8e7c77f4935339c39732e04b63fff4d36800bb95d76c755798120523d8` |

Üç application artifact'ı indirildi ve SHA-256 değerleri bağımsız hesaplandı. Windows dosyası PE x64, Linux dosyası ELF64 x64, macOS ZIP'i geçerli arm64 APP olarak doğrulandı. Linux/macOS production smoke kayıtlarının kaynak commit'i ve dosya hash'i indirilen artifact'larla eşleşir.

İndirilen Windows EXE'si yerelde `tests/native/windows-ui.ps1 -Executable '<indirilen-exe>'` ile doğrulandı: native pencere, klavye/menü kullanımı ve diagnostics kontrolleri geçti. Bu sonuç doğrudan tablodaki EXE'ye aittir; tarihsel bir binary'nin sonucu aktarılmadı. Test kurulum/kaldırma/Workshop/yama çağrılarını engelledi; gerçek Steam veya oyun değişikliği yapılmadı.

Linux/macOS production dosyaları upload → download → `cmp` sonrasında içerikleri değiştirilmeden açıldı. [Linux smoke kanıtı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/artifacts/13511956107) ve [macOS smoke kanıtı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/artifacts/13509058328), `result.json`, `application.log`, `production-window.png` ile kaynak/hash/PID/görünür pencere/çizim sonucunu içerir. Son ekran görüntüleri görsel incelendi. Production çizim kontrolü renk/parlaklık sezgisidir; tam etkileşim testi değildir.

[Linux WDIO](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/artifacts/13511376875) ve [macOS WDIO](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42231971879/artifacts/13510577940), ayrı test-feature build'lerinde 1/1 geçti; branch arayüzü ve yeniden açılabilir diagnostics kontrollerini doğrular. [Test-build etkileşim kapsamı](../tests/native/unix-ui-acceptance.md), production startup/paint smoke'undan ayrıdır. Yerel dosya/hash/native UI kanıtları ve ilk Linux deneme kayıtları, git'e alınmayan `installer2/target/acceptance/current-source-b47578f/` altında tutulur; ham config veya özel kurtarma içerikleri bu kanıta dahil edilmez.

2026-11-05 tarihli bağımsız disk yedekli gerçek Steam config roundtrip ayrıca doğrulandı: hedefli gerçek patch, ilk backup/idempotency, ilgisiz seçeneklerin korunması, orijinal config/backup içerik-varlık-izin geri yüklemesi ve normal restart geçti. Canlı test `aed9b22` üzerindeki çalışma ağacında çalıştı ve test desteği `460b969` ile commit edildi. Son DACL karşılaştırması yalnız bilgilendirici `AI` bitini dışlar; `AR` dahil diğer kontrol bayrakları ve sıralı binary ACE'ler korunur. Son karşılaştırma izole regresyon/hosted Windows testleriyle doğrulandı; canlı Steam yazmaları tekrarlanmadı. Yerel özel kurtarma yolları `installer2/target/acceptance/windows-steam-config-step2-final.log` içinde kalır; config/yedek içerikleri yayın artifact'ı değildir.

Gerçek oyun kurulumu/oyun içi mod çalışması, gerçek Workshop indirme/abonelik matrisi ve manuel Gatekeeper/SmartScreen dağıtım akışı bu kanıtla doğrulanmış sayılmaz ve çalıştırılmadı. Bunlar oyun açılışını installer kabulüne veya public yayına önkoşul yapmaz. İzole fixture testleri ile native pencere testlerinin kapsamı [test belgesinde](../tests/README.md) ayrıdır. Linux CI'da mevcut 100 ms fixture indirme bütçesi aralıklı zaman aşımı üretebilir; bu koşudaki ilk Linux denemesinin iptal nedeni Ubuntu paket aktarım gecikmesidir.

## Tarihsel taslak

`v26.11.3-rust-candidate.20261104.2`, kaynak `c7662e9e9489a669381ed9315b2f39f0f2fe5612`: [önceki workflow](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41903125865) ve [taslak prerelease](https://github.com/ipatavatsizz/zbinstaller/releases/tag/untagged-75932294f8141d3d5a54). Bu taslak yukarıdaki yeni kaynak/dosyalarla güncellenmedi; güncel artifact kimliği olarak kullanılmaz. Public yayın yapılmadı.
