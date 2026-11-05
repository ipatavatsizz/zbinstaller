# Rust uygulama adayları

## Deneysel dağıtım ve imza politikası

2026-11-04: mevcut Go yayın akışındaki imzasız dağıtım, Rust deneysel adaylarında da korunur. Windows EXE Authenticode ile imzalanmaz. Linux dosyası ve macOS APP ZIP'i imzasızdır; macOS notarization uygulanmaz. Sertifika veya imzalama secret'ı bu çalışma için gerekli değildir. Windows SmartScreen ve macOS Gatekeeper uyarıları görülebilir. Bu kayıt imzalı veya notarized dağıtım kabulü değildir.

Aracın kendisi kurulmaz: Windows EXE, Linux çalıştırılabilir dosyası, macOS APP içeren ZIP indirilir ve doğrudan açılır. Windows WebView2, Linux WebKitGTK 4.1 gerektirir. Linux/macOS desteği deneysel olarak etiketlenir. Native CI sonuçları gerçek Steam/oyun veya tüm masaüstü dağıtımlarının kabulü değildir.

## Adayın doğrulanması

1. Aynı commit'te frontend build/UI testleri, Rust fmt/test/Clippy ve üç platform derlemesi başarılı olmalı.
2. Windows izole süreç ve başlatıcı kabul testleri workspace testlerine dahildir. Linux/macOS test derlemesi gerçek Tauri/WebKit pencerelerinde native UI suite'ini geçmeli.
3. İndirilebilir release dosyaları WebDriver feature/config olmadan derlenir. Windows'ta indirilen EXE native UI testinden geçirilir. Linux/macOS native Actions runner'larında production executable/ZIP-APP görünür pencere smoke'u ayrıca doğrulanır; mevcut WDIO test-build sonucu bu kontrolün yerine geçmez. Bu ek smoke henüz çalıştırılmadı.
4. Taslakta yalnızca üç uygulama dosyası ile SHA-256 doğrulama listesi bulunur; UI ekran görüntüleri ayrı Actions artifact'larıdır.
5. Aday commit'i, workflow bağlantısı, dosya adları ve Windows sonucu [geçiş kaydına](MIGRATION.md) yazılır. Başarısız testte aday kabul edilmez.

Yeni adaylar önce taslak prerelease olarak hazırlanır. Taslağın public yayımlanması ayrı yayın adımıdır. Eski Go kaynakları referans/geri dönüş için korunur; Rust uygulamasını derlemek veya çalıştırmak Go gerektirmez. Güncel görevde gerekli Windows Steam config roundtrip ve native Actions production smoke sonuçları [kabul tablosunda](MIGRATION.md#kabul-sonuç-kaydı) izlenir. Gerçek oyun ve ayrı manuel masaüstü kapsam dışı, çalıştırılmadı olarak tutulur; görev engeli değildir.

## Doğrulanan aday

2026-11-04: `v26.11.3-rust-candidate.20261104.2`, commit `c7662e9e9489a669381ed9315b2f39f0f2fe5612`. [Etiket workflow'u](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41903125865) üç native build/test/lint ile Linux/macOS native UI suite'lerini geçti ve [taslağı](https://github.com/ipatavatsizz/zbinstaller/releases/tag/untagged-75932294f8141d3d5a54) üretti. Üç uygulama indirildi, hash'leri doğrulandı; indirilen Windows EXE native UI kabulünü geçti. macOS ZIP'in APP yapısı doğrulandı; macOS/Linux dosyaları Windows'ta çalıştırılmadı. Hash listesi ve kapsam [geçiş kaydında](MIGRATION.md#34-ana-yayın-akışını-değiştirme).

## Kalan kabul kapısı

[30–34 prosedürü](../tests/README.md#gerçek-kabul-çalıştırma-prosedürü-3034) ve [güncel kabul kaydı](MIGRATION.md#kabul-ortamı-ve-engeller-2026-11-05), yedekli Windows Steam config roundtrip, Linux/macOS native Actions WDIO ve production artifact görünür pencere smoke'u ile aynı kaynak CI/hash kanıtlarını gerektirir. Güvenli Windows config testi 2026-11-05 yerel çalışmada geçti (Tester kabulü beklenir); Linux/macOS production smoke henüz uygulanıp çalıştırılmadı; önceki CI sonuçları yeni değişikliklere aktarılmaz. Config yazmalarından önce tam hedef listesi ve bağımsız geri yüklenebilir yedek gerekir; geçici değişiklikler ve orijinal backup'lar doğrulanarak geri yüklenir. Gerçek oyun dosyaları/kurulum/açılış ve ayrı manuel masaüstü kapsam dışıdır, başarı sayılmaz ve görev engeli değildir. Public yayın ayrı karardır; bu çalışma public yayın yapmaz.
