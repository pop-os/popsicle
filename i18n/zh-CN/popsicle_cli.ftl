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
