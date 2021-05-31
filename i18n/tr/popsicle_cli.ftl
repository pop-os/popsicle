question = '{$image_path}' konumundaki disk görüntüsünü listelenen bellek aygıtlarına yazdırmak istediğinize emin misiniz?

yn = e/H

# Arguments
arg-image = DİSK GÖRÜNTÜSÜ
arg-image-desc = (Girdi) disk görüntüsü dosyası

arg-disks = DISKS
arg-disks-desc = (Çıktı) bellek aygıtı

arg-all-desc = Algılanan tüm USB bellek aygıtlarına yazdır
arg-check-desc = Yazılan disk görüntüsünün kaynak disk görüntüsüyle aynı olup olmadığını kontrol et
arg-unmount-desc = Bağlı bellek aygıtlarının bağlantısını kes
arg-yes-desc = Onay almadan devam et

# errors
error-caused-by = sebep:
error-image-not-set = {arg-image} seçilmedi
error-image-open = '{$image_path}' konumundaki disk görüntüsü açılamadı
error-image-metadata = '{$image_path}' konumundaki disk görüntüsünün üst verisine ulaşılamadı
error-disks-fetch = USB bellek aygıtlarının listesine ulaşılamadı
error-no-disks-specified = bellek aygıtı seçilmedi
error-fetching-mounts = bağlı cihazların listesine ulaşılamadı
error-opening-disks = bellek aygıtı açılamadı
error-exiting = yazdırılmadan sonlandırılıyor
error-reading-mounts = bağlı bellek aygıtları okunurken bir hata oluştu
