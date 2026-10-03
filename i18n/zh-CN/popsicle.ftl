app-title = USB 镜像刷入

# Images View
cannot-select-directories = 文件选择器无法选择路径
check-label = 检查
choose-image-button = 选择镜像
generating-checksum = 正在创建校验值
hash-label = Hash:
image-view-description = 请选择你想刷入的 .iso 或 .img。你现在也可以插入 USB 闪存盘。
image-view-title = 选择镜像
no-image-selected = 没有选择镜像
none = 无
warning = 警告：

# Devices View
device-too-small = 设备太小
devices-view-description = 刷入会抹除掉目标磁盘的所有数据。
devices-view-title = 选择磁盘
select-all = 全选

# Flashing View
flash-view-description = 请勿在刷入过程中拔出设备。
flash-view-title = 正在刷入

# Summary View
flashing-completed = 刷入完成
flashing-completed-with-errors = 刷入完成但出错
flash-again = 重刷

# Error View
critical-error = 发生严重错误

# Misc
cancel = 取消
close = 关闭
done = 完成
next = 下一步
open = 打开
task-finished = 完成

# Events
error = 错误：{$why}
partial-flash = {$total} 中的 {$number}  个设备已成功刷入
successful-flash = {$total} 个设备已成功刷入
win-isos-not-supported = 目前不支持 Windows 的 ISO

# Errors
iso-open-failed = 无法打开 ISO
no-value-found = 没有发现值

# Command line
question = 你确实要将 '{$image_path}' 镜像刷入到所示的磁盘吗？

yn = y/N
y = y

# Arguments
arg-image = 镜像
arg-image-desc = 要刷入的镜像文件

arg-disks = 磁盘
arg-disks-desc = 要输出到的磁盘

arg-all-desc = 刷入所有检测到的 USB 设备
arg-check-desc = 检测写入的镜像是否与源镜像一致
arg-unmount-desc = 卸载已挂载的设备
arg-yes-desc = 继续且无需确认

# errors
error-caused-by = 导致的原因
error-image-not-set = {arg-image} 未设置
error-image-open = 无法打开下列镜像：'{$image_path}'
error-image-metadata = 无法获取下列镜像的元数据：'{$image_path}'
error-disks-fetch = 无法获取 USB 磁盘列表
error-no-disks-specified = 未设定磁盘
error-fetching-mounts = 无法获取挂载项列表
error-opening-disks = 无法打开磁盘
error-exiting = 退出但不刷入
error-reading-mounts = 无法读取挂载项
