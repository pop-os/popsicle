question = Weet u zeker dat u '{$image_path}' naar de volgende schijven wilt flashen?
  <#-- Ja of Nee -->
yn = j/N
y = j

# Arguments
arg-image = IMAGE
arg-image-desc = Imagebestand selecteren

arg-disks = SCHIJVEN
arg-disks-desc = Beschikbare schijven selecteren

arg-all-desc = Alle gedetecteerde USB-schijven flashen
arg-check-desc = Controleer of het geschreven image overeenkomt met het bronimage
arg-unmount-desc = Ontkoppel aangekoppelde apparaten
arg-yes-desc = Doorgaan zonder bevestiging

# errors
error-caused-by = veroorzaakt door
error-image-not-set = {arg-image} niet ingesteld
error-image-open = kan het image op '{$image_path}' niet openen
error-image-metadata = kan de metadata van het image op '{$image_path}' niet ophalen
error-disks-fetch = kon de lijst van USB-schijven niet ophalen
error-no-disks-specified = geen schijven gespecificeerd
error-fetching-mounts = kon de lijst van gekopelde schijven niet ophalen
error-opening-disks = kon schijven niet openen
error-exiting = afsluiten zonder te flashen
error-reading-mounts = kon gekoppelde schijven niet lezen
