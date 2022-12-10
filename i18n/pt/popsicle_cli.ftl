question = Gravar '{$image_path}' nos seguintes dispositivos?

yn = s/N
y = s

# Arguments
arg-image = IMAGEM
arg-image-desc = Ficheiro de imagem de entrada

arg-disks = DISPOSITIVOS
arg-disks-desc = Dispositivos de saída

arg-all-desc = Gravar todos os dispositivos USB detetados
arg-check-desc = Verificar se a imagem gravada corresponde à imagem de origem
arg-unmount-desc = Desmontar dispositivos montados
arg-yes-desc = Prosseguir sem confirmação

# errors
error-caused-by = causado por
error-image-not-set = {arg-image} não definida
error-image-open = não foi possível abrir a imagem em '{$image_path}'
error-image-metadata = não foi possível pesquisar metadados da imagem em '{$image_path}'
error-disks-fetch = falha ao pesquisar dispositivos USB
error-no-disks-specified = nenhum dipositivo especificado
error-fetching-mounts = falha na pesquisa de pontos de montagem
error-opening-disks = falha ao abrir dispositivos
error-exiting = sair sem gravar
error-reading-mounts = erro ao ler os pontos de montagem
