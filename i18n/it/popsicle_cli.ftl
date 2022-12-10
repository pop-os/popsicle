question = Sei sicuro di voler caricare '{$image_path}' nelle seguenti unità USB?

yn = s/N
y = s

# Arguments
arg-image = IMMAGINE
arg-image-desc = Input file ISO

arg-disks = Unità USB
arg-disks-desc = Output 

arg-all-desc = Crea unità in tutte le unità USB
arg-check-desc = Controlla se l'immagine scritta corrisponde all'immagine di origine
arg-unmount-desc = Formatta le unità USB create
arg-yes-desc = Continua senza conferma

# errors
error-caused-by = causato da
error-image-not-set = {arg-image} non configurata
error-image-open = impossibile aprire l'immagine su '{$image_path}'
error-image-metadata = impossibile recuperare i metadati dell'immagine su '{$image_path}'
error-disks-fetch = impossibile recuperare l'elenco delle unità USB
error-no-disks-specified = nessuna unità specificata
error-fetching-mounts = impossibile recuperare l'elenco delle unità USB create
error-opening-disks = impossibile aprire le unità USB
error-exiting = esci senza creare
error-reading-mounts = errore di lettura delle unità USB create
