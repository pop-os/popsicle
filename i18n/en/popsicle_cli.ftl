question = Are you sure you want to flash '{$image_path}' to the following drives?

yn = y/N
y = y

# Arguments
arg-image = IMAGE
arg-image-desc = Input image file

arg-disks = DISKS
arg-disks-desc = Output disk devices

arg-all-desc = Flash all detected USB drives
arg-check-desc = Check if written image matches source image
arg-unmount-desc = Unmount mounted devices
arg-yes-desc = Continue without confirmation

# errors
error-caused-by = caused by
error-image-not-set = {arg-image} not set
error-image-open = unable to open image at '{$image_path}'
error-image-metadata = unable to fetch image metadata at '{$image_path}'
error-disks-fetch = failed to fetch list of USB disks
error-no-disks-specified = no disks specified
error-fetching-mounts = failed to fetch list of mounts
error-opening-disks = failed to open disks
error-exiting = exiting without flashing
error-reading-mounts = error reading mounts
