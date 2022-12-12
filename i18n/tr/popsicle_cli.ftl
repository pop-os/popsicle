question = '{$image_path}' konumundaki disk görüntüsünü listelenen bellek aygıtlarına yazdırmak istediğinize emin misiniz?

yn = e/H
y = e

# Arguments
arg-image = DİSK GÖRÜNTÜSÜ
arg-image-desc = (Girdi) disk görüntüsü dosyası

arg-disks = DİSKLER
arg-disks-desc = (Çıktı) bellek aygıtı

arg-all-desc = Algılanan tüm USB bellek aygıtlarına yazdır
arg-check-desc = Yazılan disk görüntüsünün kaynak disk görüntüsüyle aynı olup olmadığını kontrol et
arg-unmount-desc = Bağlı bellek aygıtlarının bağlantısını kes
arg-yes-desc = Onay almadan devam et

# errors
error-caused-by = sebep:
error-image-not-set = {arg-image} seçilmedi.
error-image-open = '{$image_path}' konumundaki disk görüntüsü açılamadı.
error-image-metadata = '{$image_path}' konumundaki disk görüntüsünün üst verisine ulaşılamadı.
error-disks-fetch = USB bellek aygıtlarının listesine ulaşılamadı.
error-no-disks-specified = Bellek aygıtı seçilmedi.
error-fetching-mounts = Bağlı cihazların listesine ulaşılamadı.
error-opening-disks = Bellek aygıtı açılamadı.
error-exiting = Yazdırılmadan sonlandırılıyor.
error-reading-mounts = Bağlı bellek aygıtları okunurken bir hata oluştu.
