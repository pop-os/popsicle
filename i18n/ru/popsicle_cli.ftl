question = Вы уверены, что хотите записать '{$image_path}' на следующие диски?

yn = y/n
y = y

# Arguments
arg-image = ОБРАЗ
arg-image-desc = Ввод файл образа

arg-disks = ДИСК
arg-disks-desc = Вывод дисковых устройств
arg-all-desc = Записать на все обнаруженные USB-накопители

arg-check-desc = Проверьте, соответствует ли записанный образ исходному
arg-unmount-desc = Размонтировать устройства
arg-yes-desc = Продолжить без подтверждения

# errors
error-caused-by = вызвано
error-image-not-set = {arg-image} не задан
error-image-open = не удалось открыть образ по '{$image_path}'
error-image-metadata = не удалось получить метаданные образа по '{$image_path}'
error-disks-fetch = не удалось получить список USB-дисков
error-no-disks-specified = диски не указаны
error-fetching-mounts = не удалось получить список монтирования
error-opening-disks = не удалось открыть диски
error-exiting = выход без записи
error-reading-mounts = ошибка чтения монтирования
